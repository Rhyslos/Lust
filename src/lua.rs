//! Writes a checked program as Lua 5.1: plain enough for Garry's Mod (LuaJIT) and Luau.
//!
//! Every output line remembers which Lust line it came from, so errors while running
//! can point at the Lust code. Lines carry a marker while being built: \x01LINE\x02.

use std::collections::HashSet;

use crate::ast::*;
use crate::check::{CallRes, Info, NameRes};
use crate::types::Ty;

pub const RUNTIME: &str = include_str!("runtime.lua");

const LUA_KEYWORDS: &[&str] = &[
    "and", "break", "do", "else", "elseif", "end", "false", "for", "function", "goto", "if", "in", "local", "nil", "not", "or", "repeat",
    "return", "then", "true", "until", "while",
];

/// A Lust name as a Lua name. Lua's own words get a _ on the end.
pub fn lname(n: &str) -> String {
    if LUA_KEYWORDS.contains(&n) {
        format!("{n}_")
    } else {
        n.to_string()
    }
}

pub struct Output {
    pub code: String,
    /// For each line of `code` (from 1), the Lust line it came from. 0 for Lust's own runtime.
    pub lines: Vec<u32>,
}

struct LoopFrame {
    /// Set when the loop's body has a skip: the body sits in repeat … until true.
    wrapped: bool,
    stop_var: String,
}

struct W<'a> {
    info: &'a Info,
    buf: String,
    indent: usize,
    tmp: usize,
    class: Option<usize>,
    loops: Vec<LoopFrame>,
    object_names: HashSet<String>,
}

pub fn write(prog: &Program, info: &Info) -> Output {
    let mut w = W {
        info,
        buf: String::new(),
        indent: 0,
        tmp: 0,
        class: None,
        loops: vec![],
        object_names: HashSet::new(),
    };
    w.program(prog);
    // Strip the line markers into a line map.
    let mut code = String::new();
    let mut lines = Vec::new();
    let runtime_lines = RUNTIME.lines().count() + 2;
    code.push_str("local _L = (function()\n");
    code.push_str(RUNTIME);
    code.push_str("\nend)()\n");
    lines.extend(std::iter::repeat(0).take(runtime_lines + 1));
    let mut current = 0u32;
    for raw in w.buf.lines() {
        let mut text = String::new();
        let mut rest = raw;
        while let Some(i) = rest.find('\x01') {
            text.push_str(&rest[..i]);
            let after = &rest[i + 1..];
            let j = after.find('\x02').unwrap_or(0);
            current = after[..j].parse().unwrap_or(current);
            rest = &after[j + 1..];
        }
        text.push_str(rest);
        code.push_str(&text);
        code.push('\n');
        lines.push(current);
    }
    Output { code, lines }
}

fn lua_string(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 32 => out.push_str(&format!("\\{}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn has_skip(stmts: &[Stmt]) -> bool {
    stmts.iter().any(|s| match s {
        Stmt::Skip(_) => true,
        Stmt::If { branches, otherwise, .. } => {
            branches.iter().any(|(_, b)| has_skip(b)) || otherwise.as_ref().map(|o| has_skip(o)).unwrap_or(false)
        }
        Stmt::Try { body, handler, .. } => has_skip(body) || has_skip(handler),
        _ => false,
    })
}

impl<'a> W<'a> {
    fn line(&mut self, lust_line: u32, text: &str) {
        self.buf.push_str(&format!("\x01{lust_line}\x02"));
        for _ in 0..self.indent {
            self.buf.push_str("  ");
        }
        self.buf.push_str(text);
        self.buf.push('\n');
    }

    fn fresh(&mut self, base: &str) -> String {
        self.tmp += 1;
        format!("_{base}{}", self.tmp)
    }

    fn class_table(&self, c: usize) -> String {
        let ci = &self.info.classes[c];
        if ci.is_object {
            format!("_O_{}", ci.name)
        } else {
            lname(&ci.name)
        }
    }

    // ----- the program -----

    fn program(&mut self, prog: &Program) {
        // States: each is a table listing its options.
        for item in &prog.items {
            if let Item::State(s) = item {
                if let Some(&si) = self.info.state_index.get(&s.name) {
                    let opts: Vec<String> = self.info.states[si].options.iter().map(|o| lua_string(o)).collect();
                    self.line(
                        s.pos.line,
                        &format!(
                            "local {} = _L.state({}, {{{}}})",
                            lname(&s.name),
                            lua_string(&s.name),
                            opts.join(", ")
                        ),
                    );
                }
            }
        }
        // Names first, so everything can use everything.
        let mut forward = Vec::new();
        for item in &prog.items {
            match item {
                Item::Method(m) => forward.push(lname(&m.name)),
                Item::Class(c) => {
                    if c.is_object {
                        self.object_names.insert(c.name.clone());
                        forward.push(lname(&c.name));
                    }
                }
                _ => {}
            }
        }
        if !forward.is_empty() {
            self.line(0, &format!("local {}", forward.join(", ")));
        }
        // Classes, parents first.
        let mut done = HashSet::new();
        let classes: Vec<&ClassDecl> = prog
            .items
            .iter()
            .filter_map(|i| match i {
                Item::Class(c) => Some(c),
                _ => None,
            })
            .collect();
        while done.len() < classes.len() {
            let before = done.len();
            for c in &classes {
                if done.contains(&c.name) {
                    continue;
                }
                let ready = c
                    .parent
                    .as_ref()
                    .map(|(p, _)| done.contains(p) || !classes.iter().any(|x| &x.name == p))
                    .unwrap_or(true);
                if ready {
                    self.class(c);
                    done.insert(c.name.clone());
                }
            }
            if done.len() == before {
                break;
            }
        }
        // Methods on their own.
        for item in &prog.items {
            if let Item::Method(m) = item {
                self.class = None;
                let params: Vec<String> = m.params.iter().map(|p| lname(&p.name)).collect();
                self.line(m.pos.line, &format!("{} = function({})", lname(&m.name), params.join(", ")));
                self.indent += 1;
                self.stmts(&m.body);
                self.indent -= 1;
                self.line(m.pos.line, "end");
            }
        }
        // Objects come to life once every class exists.
        for c in &classes {
            if c.is_object {
                if let Some(&ci) = self.info.class_index.get(&c.name) {
                    let t = self.class_table(ci);
                    self.line(c.pos.line, &format!("{} = _L.new({t})", lname(&c.name)));
                }
            }
        }
        self.class = None;
        self.stmts(&prog.body);
    }

    fn class(&mut self, c: &ClassDecl) {
        let Some(&ci) = self.info.class_index.get(&c.name) else { return };
        let table = self.class_table(ci);
        let parent = self.info.classes[ci]
            .parent
            .map(|p| self.class_table(p))
            .unwrap_or_else(|| "nil".into());
        self.line(c.pos.line, &format!("local {table} = _L.class({}, {parent})", lua_string(&c.name)));
        self.class = None;
        self.line(c.pos.line, &format!("function {table}.__fields(self)"));
        self.indent += 1;
        if parent != "nil" {
            self.line(c.pos.line, &format!("{parent}.__fields(self)"));
        }
        for f in &c.fields {
            let value = match &f.default {
                Some(d) => self.expr(d),
                None => "nil".into(),
            };
            self.line(f.pos.line, &format!("self.{} = {value}", lname(&f.name)));
        }
        self.indent -= 1;
        self.line(c.pos.line, "end");
        self.class = Some(ci);
        for m in &c.methods {
            let params: Vec<String> = m.params.iter().map(|p| lname(&p.name)).collect();
            self.line(m.pos.line, &format!("function {table}:{}({})", lname(&m.name), params.join(", ")));
            self.indent += 1;
            self.stmts(&m.body);
            self.indent -= 1;
            self.line(m.pos.line, "end");
        }
        self.class = None;
    }

    // ----- statements -----

    fn stmts(&mut self, stmts: &[Stmt]) {
        for s in stmts {
            self.stmt(s);
        }
    }

    fn stmt(&mut self, s: &Stmt) {
        let ln = s.pos().line;
        match s {
            Stmt::Var { name, value, .. } => {
                let v = self.expr(value);
                self.line(ln, &format!("local {} = {v}", lname(name)));
            }
            Stmt::Assign { target, value, .. } | Stmt::SetState { target, option: value, .. } => {
                let v = self.expr(value);
                self.assign(ln, target, v);
            }
            Stmt::Expr(e) => {
                let code = self.expr(e);
                self.line(ln, &code);
            }
            Stmt::If { branches, otherwise, .. } => {
                for (i, (cond, body)) in branches.iter().enumerate() {
                    let c = self.expr(cond);
                    let word = if i == 0 { "if" } else { "elseif" };
                    self.line(cond.pos.line, &format!("{word} {c} then"));
                    self.indent += 1;
                    self.stmts(body);
                    self.indent -= 1;
                }
                if let Some(o) = otherwise {
                    self.line(ln, "else");
                    self.indent += 1;
                    self.stmts(o);
                    self.indent -= 1;
                }
                self.line(ln, "end");
            }
            Stmt::Loop { kind, body, .. } => self.loop_stmt(ln, kind, body),
            Stmt::Stop(_) => {
                let frame = self.loops.last().unwrap();
                if frame.wrapped {
                    let v = frame.stop_var.clone();
                    self.line(ln, &format!("{v} = true"));
                }
                self.line(ln, "do break end");
            }
            Stmt::Skip(_) => self.line(ln, "do break end"),
            Stmt::Return(v, _) => match v {
                Some(e) => {
                    let code = self.expr(e);
                    self.line(ln, &format!("do return {code} end"));
                }
                None => self.line(ln, "do return end"),
            },
            Stmt::Try { body, err, handler, .. } => {
                let ok = self.fresh("ok");
                let e = self.fresh("err");
                self.line(ln, &format!("local {ok}, {e} = pcall(function()"));
                self.indent += 1;
                let saved = std::mem::take(&mut self.loops);
                self.stmts(body);
                self.loops = saved;
                self.indent -= 1;
                self.line(ln, "end)");
                self.line(ln, &format!("if not {ok} then"));
                self.indent += 1;
                self.line(err.1.line, &format!("local {} = _L.errtext({e})", lname(&err.0)));
                self.stmts(handler);
                self.indent -= 1;
                self.line(ln, "end");
            }
            Stmt::Lua { code, pos } => {
                let mut l = pos.line;
                for raw in code.lines() {
                    if raw.trim().is_empty() {
                        l += 1;
                        continue;
                    }
                    let saved = self.indent;
                    self.indent = 0;
                    self.line(l, raw);
                    self.indent = saved;
                    l += 1;
                }
            }
        }
    }

    fn assign(&mut self, ln: u32, target: &Expr, value: String) {
        match &target.kind {
            ExprKind::Index(t, i) => {
                let tc = self.expr(t);
                let ic = self.expr(i);
                let f = match self.info.ty(t) {
                    Ty::Map(_) => "mset",
                    _ => "aset",
                };
                self.line(ln, &format!("_L.{f}({tc}, {ic}, {value})"));
            }
            _ => {
                let t = self.expr(target);
                self.line(ln, &format!("{t} = {value}"));
            }
        }
    }

    fn loop_stmt(&mut self, ln: u32, kind: &LoopKind, body: &[Stmt]) {
        let wrapped = has_skip(body);
        let stop_var = self.fresh("stop");
        let mut close = vec!["end".to_string()];
        match kind {
            LoopKind::Forever => self.line(ln, "while true do"),
            LoopKind::While(c) => {
                let c = self.expr(c);
                self.line(ln, &format!("while {c} do"));
            }
            LoopKind::Times(n) => {
                let n = self.expr(n);
                let i = self.fresh("i");
                self.line(ln, &format!("for {i} = 1, {n} do"));
            }
            LoopKind::Each { vars, source } => {
                let src = self.expr(source);
                let names: Vec<String> = vars.iter().map(|(_, n, _)| lname(n)).collect();
                match self.info.ty(source) {
                    Ty::Array(_) => {
                        let c = self.fresh("list");
                        let i = self.fresh("i");
                        self.line(ln, "do");
                        self.indent += 1;
                        self.line(ln, &format!("local {c} = _L.acopy({src})"));
                        self.line(ln, &format!("for {i} = 1, {c}.n do"));
                        self.indent += 1;
                        self.line(ln, &format!("local {} = {c}[{i}]", names[0]));
                        self.indent -= 1;
                        close = vec!["end".into(), "end".into()];
                        self.indent += 1;
                        self.indent -= 1;
                    }
                    Ty::Map(_) => {
                        let m = self.fresh("map");
                        let k = self.fresh("k");
                        self.line(ln, "do");
                        self.indent += 1;
                        self.line(ln, &format!("local {m} = {src}"));
                        self.line(ln, &format!("for _, {k} in ipairs(_L.keys({m})) do"));
                        self.indent += 1;
                        self.line(
                            ln,
                            &format!(
                                "local {}, {} = {k}, {m}.vals[{k}]",
                                names[0],
                                names.get(1).cloned().unwrap_or_else(|| "_".into())
                            ),
                        );
                        self.indent -= 1;
                        close = vec!["end".into(), "end".into()];
                    }
                    Ty::CArray(cols) => {
                        let r = self.fresh("row");
                        self.line(ln, &format!("for _, {r} in ipairs(_L.rows({src})) do"));
                        self.indent += 1;
                        let mut fields = vec![format!("{r}.id")];
                        fields.extend(cols.iter().map(|(n, _)| format!("{r}.{}", lname(n))));
                        self.line(
                            ln,
                            &format!(
                                "local {} = {}",
                                names.join(", "),
                                fields[..names.len().min(fields.len())].join(", ")
                            ),
                        );
                        self.indent -= 1;
                    }
                    _ => {
                        // A state's options.
                        self.line(ln, &format!("for _, {} in ipairs({src}.list) do", names[0]));
                    }
                }
            }
        }
        self.indent += 1;
        if wrapped {
            self.line(ln, &format!("local {stop_var} = false"));
            self.line(ln, "repeat");
            self.indent += 1;
        }
        self.loops.push(LoopFrame {
            wrapped,
            stop_var: stop_var.clone(),
        });
        self.stmts(body);
        self.loops.pop();
        if wrapped {
            self.indent -= 1;
            self.line(ln, "until true");
            self.line(ln, &format!("if {stop_var} then break end"));
        }
        self.indent -= 1;
        // The array and map loops opened a do … end around the loop.
        if close.len() == 2 {
            self.line(ln, "end");
            self.indent -= 1;
            self.line(ln, "end");
        } else {
            self.line(ln, "end");
        }
    }

    // ----- expressions -----

    fn expr(&mut self, e: &Expr) -> String {
        match &e.kind {
            ExprKind::Int(v) => {
                if *v < 0 {
                    format!("({v})")
                } else {
                    v.to_string()
                }
            }
            ExprKind::Float(v) => {
                let s = if v.fract() == 0.0 && v.abs() < 1e15 {
                    format!("{v:.1}")
                } else {
                    format!("{v}")
                };
                if *v < 0.0 {
                    format!("({s})")
                } else {
                    s
                }
            }
            ExprKind::Str(s) => lua_string(s),
            ExprKind::Bool(b) => b.to_string(),
            ExprKind::Nothing => "nil".into(),
            ExprKind::Name(n) => {
                if let Some((_, path)) = self.info.options.get(&e.id) {
                    if !path.is_empty() {
                        return lua_string(path);
                    }
                }
                match self.info.names.get(&e.id) {
                    Some(NameRes::Field) => format!("self.{}", lname(n)),
                    Some(NameRes::OwnMethod) => format!("_L.bind(self, {})", lua_string(&lname(n))),
                    _ => lname(n),
                }
            }
            ExprKind::Field(t, f, _) => {
                if let Some((_, path)) = self.info.options.get(&e.id) {
                    return lua_string(path);
                }
                let tc = self.expr(t);
                format!("{tc}.{}", lname(f))
            }
            ExprKind::MethodRef(t, m, _) => {
                let tc = self.expr(t);
                format!("_L.bind({tc}, {})", lua_string(&lname(m)))
            }
            ExprKind::Method(t, m, args, _) => {
                let tc = self.expr(t);
                let a: Vec<String> = args.iter().map(|x| self.expr(x)).collect();
                match self.info.calls.get(&e.id) {
                    Some(CallRes::Builtin) => {
                        let prefix = match self.info.ty(t) {
                            Ty::Array(_) => "a",
                            Ty::Map(_) => "m",
                            _ => "c",
                        };
                        let mut all = vec![tc];
                        all.extend(a);
                        format!("_L.{prefix}{m}({})", all.join(", "))
                    }
                    Some(CallRes::FieldHook) => format!("{tc}.{}({})", lname(m), a.join(", ")),
                    _ => format!("{tc}:{}({})", lname(m), a.join(", ")),
                }
            }
            ExprKind::Call(name, args) => {
                let res = self.info.calls.get(&e.id).copied();
                if res == Some(CallRes::Builtin) {
                    return self.builtin(name, args);
                }
                let a: Vec<String> = args.iter().map(|x| self.expr(x)).collect();
                match res {
                    Some(CallRes::Own) => format!("self:{}({})", lname(name), a.join(", ")),
                    Some(CallRes::FieldHook) => format!("self.{}({})", lname(name), a.join(", ")),
                    _ => format!("{}({})", lname(name), a.join(", ")),
                }
            }
            ExprKind::Index(t, i) => {
                let tc = self.expr(t);
                let ic = self.expr(i);
                let f = match self.info.ty(t) {
                    Ty::Map(_) => "mget",
                    Ty::CArray(_) => "cget",
                    _ => "aget",
                };
                format!("_L.{f}({tc}, {ic})")
            }
            ExprKind::Len(inner) => {
                let c = self.expr(inner);
                match self.info.ty(inner) {
                    Ty::Str => format!("_L.slen({c})"),
                    Ty::Map(_) => format!("#({c}).keys"),
                    Ty::CArray(_) => format!("#({c}).rows"),
                    _ => format!("({c}).n"),
                }
            }
            ExprKind::Neg(inner) => {
                let c = self.expr(inner);
                format!("(-({c}))")
            }
            ExprKind::Not(inner) => {
                let c = self.expr(inner);
                format!("(not {c})")
            }
            ExprKind::Bin(op, l, r, _) => self.binary(*op, l, r),
            ExprKind::Either(c, a, b) => {
                let cc = self.expr(c);
                let ac = self.expr(a);
                let bc = self.expr(b);
                match self.info.ty(e) {
                    // These can never be false or nil, so Lua's and/or picks correctly.
                    Ty::Int | Ty::Float | Ty::Str | Ty::State(_) | Ty::Array(_) | Ty::Map(_) | Ty::CArray(_) => {
                        format!("({cc} and {ac} or {bc})")
                    }
                    _ => format!("(function() if {cc} then return {ac} else return {bc} end end)()"),
                }
            }
            ExprKind::List { items, keys } => match self.info.ty(e) {
                Ty::Map(_) => {
                    let ks: Vec<String> = keys.as_deref().unwrap_or(&[]).iter().map(|k| self.expr(k)).collect();
                    let vs: Vec<String> = items.iter().map(|v| self.expr(v)).collect();
                    format!("_L.map({{{}}}, {{{}}}, {})", ks.join(", "), vs.join(", "), items.len())
                }
                Ty::CArray(cols) => {
                    let names: Vec<String> = cols.iter().map(|(n, _)| lua_string(&lname(n))).collect();
                    let rows: Vec<String> = items.iter().map(|r| self.row(r)).collect();
                    format!("_L.carr({{{}}}, {{{}}})", names.join(", "), rows.join(", "))
                }
                Ty::Row(_) => self.row(e),
                _ => {
                    let vs: Vec<String> = items.iter().map(|v| self.expr(v)).collect();
                    format!("_L.arr({{{}}}, {})", vs.join(", "), items.len())
                }
            },
            ExprKind::New { class, inits, .. } => {
                let Some(&c) = self.info.class_index.get(class) else {
                    return "nil".into();
                };
                let table = self.class_table(c);
                let mut parts = vec![table];
                for (f, _, v) in inits {
                    parts.push(lua_string(&lname(f)));
                    parts.push(self.expr(v));
                }
                format!("_L.new({})", parts.join(", "))
            }
            ExprKind::NewHook(body) => {
                let params: Vec<String> = self
                    .info
                    .hook_params
                    .get(&e.id)
                    .cloned()
                    .unwrap_or_default()
                    .iter()
                    .map(|p| lname(p))
                    .collect();
                let saved_buf = std::mem::take(&mut self.buf);
                let saved_loops = std::mem::take(&mut self.loops);
                self.indent += 1;
                self.stmts(body);
                self.indent -= 1;
                let inner = std::mem::replace(&mut self.buf, saved_buf);
                self.loops = saved_loops;
                let pad = "  ".repeat(self.indent);
                format!("function({})\n{inner}{pad}end", params.join(", "))
            }
            ExprKind::Like(inner, class, _) => {
                let c = self.expr(inner);
                let t = self
                    .info
                    .class_index
                    .get(class)
                    .map(|&ci| self.class_table(ci))
                    .unwrap_or_else(|| "nil".into());
                format!("_L.like({c}, {t})")
            }
            ExprKind::Convert(inner, conv, _) => {
                let c = self.expr(inner);
                let t = self.info.ty(inner);
                match conv {
                    Conv::Text => self.text(&c, &t),
                    Conv::Int => match t {
                        Ty::Str => format!("_L.sint({c})"),
                        _ => format!("_L.toint({c})"),
                    },
                    Conv::Float => match t {
                        Ty::Str => format!("_L.sfloat({c})"),
                        _ => c,
                    },
                    Conv::IsNumber => format!("_L.isnum({c})"),
                    Conv::Lines => format!("_L.lines({c})"),
                }
            }
        }
    }

    fn row(&mut self, r: &Expr) -> String {
        match &r.kind {
            ExprKind::List { items, .. } => {
                let vs: Vec<String> = items.iter().map(|v| self.expr(v)).collect();
                format!("{{n = {}, {}}}", items.len(), vs.join(", "))
            }
            _ => self.expr(r),
        }
    }

    fn builtin(&mut self, name: &str, args: &[Expr]) -> String {
        match name {
            "print" => {
                let t = self.info.ty(&args[0]);
                let c = self.expr(&args[0]);
                format!("_L.print({})", self.text(&c, &t))
            }
            _ => {
                let a: Vec<String> = args.iter().map(|x| self.expr(x)).collect();
                format!("_L.{name}({})", a.join(", "))
            }
        }
    }

    /// Lua code that turns a value of type `t` into its text.
    fn text(&self, code: &str, t: &Ty) -> String {
        match t {
            Ty::Str => code.to_string(),
            Ty::State(_) => code.to_string(),
            _ => format!("{}({code})", self.fmt(t)),
        }
    }

    /// A Lua function that turns a value of type `t` into text, for print and lists.
    fn fmt(&self, t: &Ty) -> String {
        match t {
            Ty::Int => "_L.itext".into(),
            Ty::Float => "_L.ftext".into(),
            Ty::Truth => "_L.btext".into(),
            Ty::Str => "_L.qtext".into(),
            Ty::State(_) => "_L.stext".into(),
            Ty::Array(el) => format!("_L.ashow({})", self.fmt(el)),
            Ty::Map(v) => format!("_L.mshow({})", self.fmt(v)),
            Ty::CArray(cols) => {
                let f: Vec<String> = cols.iter().map(|(_, t)| self.fmt(t)).collect();
                format!("_L.cshow({{{}}})", f.join(", "))
            }
            Ty::Row(cols) => {
                let f: Vec<String> = cols.iter().map(|(_, t)| self.fmt(t)).collect();
                format!("_L.rshow({{{}}})", f.join(", "))
            }
            _ => "_L.show".into(),
        }
    }

    fn binary(&mut self, op: BinOp, l: &Expr, r: &Expr) -> String {
        let lt = self.info.ty(l);
        let lc = self.expr(l);
        let rc = self.expr(r);
        let ints = lt == Ty::Int && self.info.ty(r) == Ty::Int;
        match op {
            BinOp::Add if lt == Ty::Str => format!("({lc} .. {rc})"),
            BinOp::Add => format!("({lc} + {rc})"),
            BinOp::Sub => format!("({lc} - {rc})"),
            BinOp::Mul => format!("({lc} * {rc})"),
            BinOp::Div if ints => format!("_L.idiv({lc}, {rc})"),
            BinOp::Div => format!("({lc} / {rc})"),
            BinOp::Mod if ints => format!("_L.imod({lc}, {rc})"),
            BinOp::Mod => format!("_L.fmod({lc}, {rc})"),
            BinOp::Eq | BinOp::Ne if matches!(lt, Ty::State(_)) => {
                let s = format!("_L.sin({lc}, {rc})");
                if op == BinOp::Ne {
                    format!("(not {s})")
                } else {
                    s
                }
            }
            BinOp::Eq | BinOp::Same => format!("({lc} == {rc})"),
            BinOp::Ne | BinOp::NotSame => format!("({lc} ~= {rc})"),
            BinOp::Lt => format!("({lc} < {rc})"),
            BinOp::Gt => format!("({lc} > {rc})"),
            BinOp::Le => format!("({lc} <= {rc})"),
            BinOp::Ge => format!("({lc} >= {rc})"),
            BinOp::And => format!("({lc} and {rc})"),
            BinOp::Or => format!("({lc} or {rc})"),
        }
    }
}
