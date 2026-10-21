//! Writes a checked program as one Rust file, which `rustc` turns into a real program.
//!
//! How Lust maps onto Rust:
//! - Things made from classes are shared: `Option<Rc<RefCell<…>>>`, where None is `nothing`.
//!   A class and everything `like` it share one struct, with a class number, so a Wizard
//!   fits wherever a Player goes. Methods go through a dispatcher that picks the right version.
//! - Arrays, maps and cMaps are shared too, like in Lua.
//! - Hooks are `Option<Rc<dyn Fn(…) -> …>>`. A `new hook` takes copies of the variables it uses.
//! - Every statement first records its Lust line, so errors while running can name it.

use std::collections::HashSet;

use crate::ast::*;
use crate::check::{CallRes, Info, NameRes};
use crate::types::Ty;

pub const RUNTIME: &str = include_str!("native_rt.rs");

struct W<'a> {
    info: &'a Info,
    /// Row structs for cMaps, by their columns.
    rows: Vec<Vec<(String, Ty)>>,
    tmp: usize,
    /// The class whose method is being written; `this` is in scope.
    class: Option<usize>,
    /// What the method or hook being written gives back.
    current_ret: Option<Ty>,
    /// A key-maker for the cMap about to be written.
    maker: Option<String>,
}

pub fn write(prog: &Program, info: &Info, file: &str) -> String {
    let mut w = W {
        info,
        rows: vec![],
        tmp: 0,
        class: None,
        current_ret: None,
        maker: None,
    };
    let mut defs = String::new();
    let mut body = String::new();

    // Classes: one struct per family (a class and everything like it).
    let n = info.classes.len();
    for c in 0..n {
        defs.push_str(&format!("const ID_{c}: u32 = {};\n", c + 1));
    }
    for root in 0..n {
        if info.classes[root].parent.is_some() {
            continue;
        }
        let family: Vec<usize> = (0..n).filter(|&c| w.root(c) == root).collect();
        let mut fields = String::new();
        for &c in &family {
            for f in &info.classes[c].fields {
                fields.push_str(&format!("    {}: {},\n", w.fname(c, &f.name), w.ty(&f.ty)));
            }
        }
        defs.push_str(&format!("#[derive(Default)]\nstruct O{root} {{\n    class: u32,\n{fields}}}\n"));
        defs.push_str(&format!(
            "impl rt::HasClass for O{root} {{ fn class_id(&self) -> u32 {{ self.class }} }}\n"
        ));
        let arms: Vec<String> = family
            .iter()
            .map(|&c| format!("ID_{c} => {:?}.to_string(),", info.classes[c].name))
            .collect();
        defs.push_str(&format!(
            "fn show{root}(o: &Option<Rc<RefCell<O{root}>>>) -> String {{ match o {{ None => \"nothing\".to_string(), Some(r) => {{ let c = r.borrow().class; match c {{ {} _ => \"?\".to_string() }} }} }} }}\n",
            arms.join(" ")
        ));
    }

    // Constructors and methods.
    let class_decls: Vec<&ClassDecl> = prog
        .items
        .iter()
        .filter_map(|i| match i {
            Item::Class(c) => Some(c),
            _ => None,
        })
        .collect();
    for c in &class_decls {
        let Some(&ci) = info.class_index.get(&c.name) else { continue };
        let root = w.root(ci);
        let mut init = String::new();
        if let Some(p) = info.classes[ci].parent {
            init.push_str(&format!("    init{p}(this);\n"));
        }
        w.class = None;
        for f in &c.fields {
            let Some(d) = &f.default else { continue };
            let Some(decl) = w.decl_class(ci, &f.name) else { continue };
            let want = info.field(ci, &f.name).map(|x| x.ty.clone()).unwrap_or(Ty::Error);
            let v = match &f.ty {
                Some(t) => w.made(t, d, &want),
                None => w.coerced(d, &want),
            };
            init.push_str(&format!(
                "    {{ let v = {v}; this.borrow_mut().{} = v; }}\n",
                w.fname(decl, &f.name)
            ));
        }
        defs.push_str(&format!("fn init{ci}(this: &Rc<RefCell<O{root}>>) {{\n{init}}}\n"));
        defs.push_str(&format!(
            "fn new{ci}() -> Rc<RefCell<O{root}>> {{ let this = Rc::new(RefCell::new(O{root} {{ class: ID_{ci}, ..Default::default() }})); init{ci}(&this); this }}\n"
        ));
        w.class = Some(ci);
        for m in &c.methods {
            let sig = info.classes[ci].methods[&m.name].clone();
            let params: Vec<String> = m
                .params
                .iter()
                .zip(&sig.params)
                .map(|(p, (_, t))| format!("mut v_{}: {}", p.name, w.ty(t)))
                .collect();
            let ret = w.ret(&sig.ret);
            let mut b = String::from("    let _depth = rt::Depth::enter();\n");
            w.current_ret = Some(sig.ret.clone());
            w.stmts(&m.body, &mut b, 1);
            if sig.ret != Ty::Void {
                b.push_str("    rt::unreachable()\n");
            }
            let mut all = vec![format!("this: &Rc<RefCell<O{root}>>")];
            all.extend(params);
            defs.push_str(&format!("fn m{ci}_{}({}){ret} {{\n{b}}}\n", m.name, all.join(", ")));
        }
        w.class = None;
    }
    // Dispatchers: one per family and method name, choosing each class's own version.
    for root in 0..n {
        if info.classes[root].parent.is_some() {
            continue;
        }
        let family: Vec<usize> = (0..n).filter(|&c| w.root(c) == root).collect();
        let mut names: Vec<String> = family.iter().flat_map(|&c| info.classes[c].methods.keys().cloned()).collect();
        names.sort();
        names.dedup();
        for m in names {
            // The signature from whichever class declares it first.
            let sig = family.iter().find_map(|&c| info.classes[c].methods.get(&m)).unwrap().clone();
            let params: Vec<String> = sig.params.iter().map(|(p, t)| format!("v_{p}: {}", w.ty(t))).collect();
            let args: Vec<String> = sig.params.iter().map(|(p, _)| format!("v_{p}")).collect();
            let mut arms = Vec::new();
            for &c in &family {
                // The nearest class up the family that has its own version.
                let mut cur = Some(c);
                while let Some(x) = cur {
                    if info.classes[x].methods.contains_key(&m) {
                        let mut a = vec!["this".to_string()];
                        a.extend(args.iter().cloned());
                        arms.push(format!("ID_{c} => m{x}_{m}({}),", a.join(", ")));
                        break;
                    }
                    cur = info.classes[x].parent;
                }
            }
            let mut all = vec![format!("this: &Rc<RefCell<O{root}>>")];
            all.extend(params);
            defs.push_str(&format!(
                "fn d{root}_{m}({}){} {{ let c = this.borrow().class; match c {{ {} _ => rt::fail(\"this thing has no method called {m}.\".into()) }} }}\n",
                all.join(", "),
                w.ret(&sig.ret),
                arms.join(" ")
            ));
        }
    }

    // Methods on their own.
    for item in &prog.items {
        if let Item::Method(m) = item {
            let sig = info.methods[&m.name].clone();
            let params: Vec<String> = m
                .params
                .iter()
                .zip(&sig.params)
                .map(|(p, (_, t))| format!("mut v_{}: {}", p.name, w.ty(t)))
                .collect();
            let mut b = String::from("    let _depth = rt::Depth::enter();\n");
            w.class = None;
            w.current_ret = Some(sig.ret.clone());
            w.stmts(&m.body, &mut b, 1);
            if sig.ret != Ty::Void {
                b.push_str("    rt::unreachable()\n");
            }
            defs.push_str(&format!("fn t_{}({}){} {{\n{b}}}\n", m.name, params.join(", "), w.ret(&sig.ret)));
        }
    }

    // Objects live in thread-locals, made at the start in the order they're written.
    let mut objects = String::new();
    for c in &class_decls {
        if !c.is_object {
            continue;
        }
        let Some(&ci) = info.class_index.get(&c.name) else { continue };
        let root = w.root(ci);
        defs.push_str(&format!(
            "thread_local! {{ static OBJ{ci}: RefCell<Option<Rc<RefCell<O{root}>>>> = RefCell::new(None); }}\nfn obj{ci}() -> Rc<RefCell<O{root}>> {{ OBJ{ci}.with(|o| o.borrow().clone().expect(\"object\")) }}\n"
        ));
        objects.push_str(&format!(
            "    {{ let o = new{ci}(); OBJ{ci}.with(|x| *x.borrow_mut() = Some(o)); }}\n"
        ));
    }
    // States aren't types in Rust: options are &'static str.

    w.class = None;
    body.push_str(&objects);
    w.stmts(&prog.body, &mut body, 1);

    // Row structs, written last since writing found them.
    let mut rows = String::new();
    let mut i = 0;
    while i < w.rows.len() {
        let cols = w.rows[i].clone();
        let fields: Vec<String> = cols.iter().map(|(n, t)| format!("    f_{n}: {},\n", w.ty(t))).collect();
        rows.push_str(&format!(
            "#[derive(Default, Clone)]\nstruct Row{i} {{\n{}}}\nimpl rt::Row for Row{i} {{ type K = {kt}; fn key(&self) -> {kt} {{ self.f_{kn}.clone() }} fn set_key(&mut self, k: {kt}) {{ self.f_{kn} = k; }} }}\n",
            fields.concat(),
            kt = w.ty(&cols[0].1),
            kn = cols[0].0
        ));
        i += 1;
    }

    format!(
        "// Written by Lust from {file}. It's rewritten on every build, so don't edit it.\n#![allow(warnings)]\nuse std::cell::RefCell;\nuse std::rc::Rc;\n\nmod rt {{\n{RUNTIME}\n}}\n\n{rows}{defs}\nfn program() {{\n{body}}}\n\nfn main() {{\n    rt::main(program, {file:?});\n}}\n"
    )
}

fn has_this(stmts: &[Stmt], info: &Info) -> bool {
    let mut found = false;
    walk_stmts(stmts, &mut |e| {
        if matches!(info.names.get(&e.id), Some(NameRes::Field | NameRes::OwnMethod)) {
            found = true;
        }
        if let ExprKind::Call(..) = e.kind {
            if matches!(info.calls.get(&e.id), Some(CallRes::Own | CallRes::FieldHook)) {
                found = true;
            }
        }
    });
    found
}

/// Calls `f` on every expression in these statements, hooks inside them included.
fn walk_stmts(stmts: &[Stmt], f: &mut dyn FnMut(&Expr)) {
    for s in stmts {
        match s {
            Stmt::Var { value, .. } => walk_expr(value, f),
            Stmt::Assign { target, value, .. } | Stmt::SetState { target, option: value, .. } => {
                walk_expr(target, f);
                walk_expr(value, f);
            }
            Stmt::Expr(e) => walk_expr(e, f),
            Stmt::If { branches, otherwise, .. } => {
                for (c, b) in branches {
                    walk_expr(c, f);
                    walk_stmts(b, f);
                }
                if let Some(o) = otherwise {
                    walk_stmts(o, f);
                }
            }
            Stmt::Loop { kind, body, .. } => {
                match kind {
                    LoopKind::Times(e) | LoopKind::While(e) => walk_expr(e, f),
                    LoopKind::Each { source, .. } => walk_expr(source, f),
                    LoopKind::Forever => {}
                }
                walk_stmts(body, f);
            }
            Stmt::Return(Some(e), _) => walk_expr(e, f),
            Stmt::Try { body, handler, .. } => {
                walk_stmts(body, f);
                walk_stmts(handler, f);
            }
            _ => {}
        }
    }
}

fn walk_expr(e: &Expr, f: &mut dyn FnMut(&Expr)) {
    f(e);
    match &e.kind {
        ExprKind::Field(t, ..) | ExprKind::MethodRef(t, ..) | ExprKind::Len(t) | ExprKind::Neg(t) | ExprKind::Not(t) => walk_expr(t, f),
        ExprKind::Like(t, ..) | ExprKind::Convert(t, ..) => walk_expr(t, f),
        ExprKind::Method(t, _, args, _) => {
            walk_expr(t, f);
            for a in args {
                walk_expr(a, f);
            }
        }
        ExprKind::Call(_, args) => {
            for a in args {
                walk_expr(a, f);
            }
        }
        ExprKind::Index(a, b) | ExprKind::Bin(_, a, b, _) => {
            walk_expr(a, f);
            walk_expr(b, f);
        }
        ExprKind::Either(a, b, c) => {
            walk_expr(a, f);
            walk_expr(b, f);
            walk_expr(c, f);
        }
        ExprKind::List { items, keys } => {
            for i in items {
                walk_expr(i, f);
            }
            for k in keys.iter().flatten() {
                walk_expr(k, f);
            }
        }
        ExprKind::New { inits, .. } => {
            for (_, _, v) in inits {
                walk_expr(v, f);
            }
        }
        ExprKind::NewHook(body) => walk_stmts(body, f),
        _ => {}
    }
}

/// The variables from outside that a hook uses. A hook gets its own copies of them,
/// made when the hook is, in both Lua and Rust.
pub fn captures(body: &[Stmt], params: &[String], info: &Info) -> Vec<String> {
    let mut inner = HashSet::new();
    declared(body, &mut inner);
    inner.extend(params.iter().cloned());
    let mut captured: Vec<String> = Vec::new();
    walk_stmts(body, &mut |x| {
        let name = match &x.kind {
            ExprKind::Name(n) if info.names.get(&x.id) == Some(&NameRes::Local) => n,
            ExprKind::Call(n, _) if info.calls.get(&x.id) == Some(&CallRes::LocalHook) => n,
            _ => return,
        };
        if !inner.contains(name) && !captured.contains(name) {
            captured.push(name.clone());
        }
    });
    captured
}

/// Names declared inside these statements: loop items, variables, error names.
fn declared(stmts: &[Stmt], out: &mut HashSet<String>) {
    for s in stmts {
        match s {
            Stmt::Var { name, .. } => {
                out.insert(name.clone());
            }
            Stmt::If { branches, otherwise, .. } => {
                for (_, b) in branches {
                    declared(b, out);
                }
                if let Some(o) = otherwise {
                    declared(o, out);
                }
            }
            Stmt::Loop { kind, body, .. } => {
                if let LoopKind::Each { vars, .. } = kind {
                    for (_, n, _) in vars {
                        out.insert(n.clone());
                    }
                }
                declared(body, out);
            }
            Stmt::Try { body, err, handler, .. } => {
                out.insert(err.0.clone());
                declared(body, out);
                declared(handler, out);
            }
            _ => {}
        }
    }
}

impl<'a> W<'a> {
    fn root(&self, mut c: usize) -> usize {
        while let Some(p) = self.info.classes[c].parent {
            c = p;
        }
        c
    }

    /// The class that declares this field: itself or the nearest one above it.
    fn decl_class(&self, c: usize, name: &str) -> Option<usize> {
        let mut cur = Some(c);
        let mut found = None;
        while let Some(i) = cur {
            if self.info.classes[i].fields.iter().any(|f| f.name == name) {
                found = Some(i);
            }
            cur = self.info.classes[i].parent;
        }
        found
    }

    fn fname(&self, decl: usize, name: &str) -> String {
        format!("f{decl}_{name}")
    }

    fn field_of(&self, c: usize, name: &str) -> String {
        let d = self.decl_class(c, name).unwrap_or(c);
        self.fname(d, name)
    }

    fn row_index(&mut self, cols: &[(String, Ty)]) -> usize {
        if let Some(i) = self.rows.iter().position(|r| r.as_slice() == cols) {
            return i;
        }
        self.rows.push(cols.to_vec());
        self.rows.len() - 1
    }

    fn ty(&mut self, t: &Ty) -> String {
        match t {
            Ty::Int => "i64".into(),
            Ty::Float => "f64".into(),
            Ty::Str => "String".into(),
            Ty::Truth => "bool".into(),
            Ty::State(_) | Ty::StateList(_) => "&'static str".into(),
            Ty::Class(c) => format!("Option<Rc<RefCell<O{}>>>", self.root(*c)),
            Ty::Array(e) => format!("rt::Arr<{}>", self.ty(e)),
            Ty::Map(e) => format!("rt::Map<{}>", self.ty(e)),
            Ty::CMap(cols) => format!("rt::CMap<Row{}>", self.row_index(cols)),
            Ty::Row(cols) => format!("Rc<RefCell<Row{}>>", self.row_index(cols)),
            Ty::Hook(sig) => {
                let p: Vec<String> = sig.params.iter().map(|(t, _)| self.ty(t)).collect();
                let r = self.ty(&sig.ret);
                format!("Option<Rc<dyn Fn({}) -> {r}>>", p.join(", "))
            }
            Ty::Void | Ty::Nothing | Ty::Error => "()".into(),
        }
    }

    fn ret(&mut self, t: &Ty) -> String {
        if *t == Ty::Void {
            String::new()
        } else {
            format!(" -> {}", self.ty(t))
        }
    }

    fn fresh(&mut self) -> usize {
        self.tmp += 1;
        self.tmp
    }

    // ----- statements -----

    fn stmts(&mut self, stmts: &[Stmt], out: &mut String, depth: usize) {
        for s in stmts {
            self.stmt(s, out, depth);
        }
    }

    fn stmt(&mut self, s: &Stmt, out: &mut String, depth: usize) {
        let pad = "    ".repeat(depth);
        let ln = s.pos().line;
        if !matches!(s, Stmt::Stop(_) | Stmt::Skip(_)) {
            out.push_str(&format!("{pad}rt::at({ln});\n"));
        }
        match s {
            Stmt::Var { ty, name, value, .. } => {
                let want = self.var_type(s);
                let v = self.made(ty, value, &want);
                let rt = self.ty(&want);
                out.push_str(&format!("{pad}let mut v_{name}: {rt} = {v};\n"));
            }
            Stmt::Assign { target, value, .. } | Stmt::SetState { target, option: value, .. } => {
                let want = self.info.ty(target);
                let v = self.coerced(value, &want);
                self.assign(target, v, out, &pad);
            }
            Stmt::Expr(e) => {
                let c = self.expr(e);
                out.push_str(&format!("{pad}{c};\n"));
            }
            Stmt::If { branches, otherwise, .. } => {
                for (i, (c, b)) in branches.iter().enumerate() {
                    let cc = self.expr(c);
                    if i == 0 {
                        out.push_str(&format!("{pad}if {cc} {{\n"));
                    } else {
                        out.push_str(&format!("{pad}}} else if {cc} {{\n"));
                    }
                    self.stmts(b, out, depth + 1);
                }
                if let Some(o) = otherwise {
                    out.push_str(&format!("{pad}}} else {{\n"));
                    self.stmts(o, out, depth + 1);
                }
                out.push_str(&format!("{pad}}}\n"));
            }
            Stmt::Loop { kind, body, .. } => {
                match kind {
                    LoopKind::Forever => out.push_str(&format!("{pad}loop {{\n")),
                    LoopKind::Times(n) => {
                        let nc = self.expr(n);
                        out.push_str(&format!("{pad}for _ in 0..{nc} {{\n"));
                    }
                    LoopKind::While(c) => {
                        let cc = self.expr(c);
                        out.push_str(&format!("{pad}while {cc} {{\n"));
                    }
                    LoopKind::Each { vars, source } => {
                        let src = self.expr(source);
                        let inner = "    ".repeat(depth + 1);
                        match self.info.ty(source) {
                            Ty::Array(_) => out.push_str(&format!("{pad}for v_{} in rt::acopy(&{src}) {{\n", vars[0].1)),
                            Ty::Map(_) => {
                                let v2 = vars.get(1).map(|v| format!("v_{}", v.1)).unwrap_or_else(|| "_".into());
                                out.push_str(&format!("{pad}for (v_{}, {v2}) in rt::mentries(&{src}) {{\n", vars[0].1));
                            }
                            Ty::CMap(cols) => {
                                let r = self.fresh();
                                out.push_str(&format!("{pad}for r{r} in rt::rows(&{src}) {{\n"));
                                for (i, (_, name, _)) in vars.iter().enumerate() {
                                    let field = format!("f_{}", cols[i].0);
                                    out.push_str(&format!("{inner}let v_{name} = r{r}.borrow().{field}.clone();\n"));
                                }
                            }
                            Ty::StateList(s) => {
                                let opts: Vec<String> = self.info.states[s].options.iter().map(|o| format!("{o:?}")).collect();
                                out.push_str(&format!("{pad}for v_{} in [{}] {{\n", vars[0].1, opts.join(", ")));
                            }
                            _ => out.push_str(&format!("{pad}for _ in 0..0 {{\n")),
                        }
                    }
                }
                self.stmts(body, out, depth + 1);
                out.push_str(&format!("{pad}}}\n"));
            }
            Stmt::Stop(_) => out.push_str(&format!("{pad}break;\n")),
            Stmt::Skip(_) => out.push_str(&format!("{pad}continue;\n")),
            Stmt::Return(v, _) => match v {
                Some(e) => {
                    let want = self.current_ret.clone().unwrap_or(Ty::Error);
                    let c = self.coerced(e, &want);
                    out.push_str(&format!("{pad}return {c};\n"));
                }
                None => out.push_str(&format!("{pad}return;\n")),
            },
            Stmt::Try { body, err, handler, .. } => {
                out.push_str(&format!("{pad}if let Some(e) = rt::attempt(|| {{\n"));
                self.stmts(body, out, depth + 1);
                out.push_str(&format!("{pad}}}) {{\n"));
                out.push_str(&format!("{pad}    let mut v_{}: String = e;\n", err.0));
                self.stmts(handler, out, depth + 1);
                out.push_str(&format!("{pad}}}\n"));
            }
            Stmt::Lua { .. } => out.push_str(&format!("{pad}rt::fail(\"lua blocks only run in LustScript.\".into());\n")),
        }
    }

    /// A variable's or field's value. A cMap made with a key-maker in its type gets the
    /// key-maker, before its first rows are added so they use it too.
    fn made(&mut self, t: &TypeRef, value: &Expr, want: &Ty) -> String {
        let Some(m) = t.key_maker() else { return self.coerced(value, want) };
        let mc = self.expr(m);
        if matches!(value.kind, ExprKind::List { .. }) {
            self.maker = Some(mc);
            self.expr(value)
        } else {
            let v = self.expr(value);
            format!("rt::cmaker({v}, {mc})")
        }
    }

    fn var_type(&self, s: &Stmt) -> Ty {
        // The declared type is what the checker resolved; it's the type the value had to fit.
        if let Stmt::Var { value, .. } = s {
            if let Some(t) = self.info.decl_types.get(&value.id) {
                return t.clone();
            }
            return self.info.ty(value);
        }
        Ty::Error
    }

    fn assign(&mut self, target: &Expr, v: String, out: &mut String, pad: &str) {
        match &target.kind {
            ExprKind::Name(n) => match self.info.names.get(&target.id) {
                Some(NameRes::Field) => {
                    let c = self.class.unwrap();
                    let f = self.field_of(c, n);
                    out.push_str(&format!("{pad}{{ let v = {v}; this.borrow_mut().{f} = v; }}\n"));
                }
                _ => out.push_str(&format!("{pad}v_{n} = {v};\n")),
            },
            ExprKind::Field(t, f, _) => {
                let tc = self.expr(t);
                match self.info.ty(t) {
                    Ty::Row(_) => out.push_str(&format!("{pad}{{ let v = {v}; let o = {tc}; o.borrow_mut().f_{f} = v; }}\n")),
                    Ty::Class(c) => {
                        let fname = self.field_of(c, f);
                        out.push_str(&format!(
                            "{pad}{{ let v = {v}; let o = rt::obj(&{tc}); o.borrow_mut().{fname} = v; }}\n"
                        ));
                    }
                    _ => {}
                }
            }
            ExprKind::Index(t, i) => {
                let tc = self.expr(t);
                let ic = self.expr(i);
                match self.info.ty(t) {
                    Ty::Map(_) => out.push_str(&format!("{pad}{{ let v = {v}; rt::mset(&{tc}, {ic}, v); }}\n")),
                    _ => out.push_str(&format!("{pad}{{ let v = {v}; rt::aset(&{tc}, {ic}, v); }}\n")),
                }
            }
            _ => {}
        }
    }

    // ----- expressions -----

    /// The expression, turned into the type it's going to.
    fn coerced(&mut self, e: &Expr, want: &Ty) -> String {
        let got = self.info.ty(e);
        let c = self.expr(e);
        coerce(c, &got, want)
    }

    fn expr(&mut self, e: &Expr) -> String {
        match &e.kind {
            ExprKind::Int(v) => format!("({v}i64)"),
            ExprKind::Float(v) => format!("({v:?}f64)"),
            ExprKind::Str(s) => format!("{s:?}.to_string()"),
            ExprKind::Bool(b) => b.to_string(),
            ExprKind::Nothing => "None".into(),
            ExprKind::Name(n) => {
                if let Some((_, path)) = self.info.options.get(&e.id) {
                    if !path.is_empty() {
                        return format!("{path:?}");
                    }
                }
                match self.info.names.get(&e.id) {
                    Some(NameRes::Field) => {
                        let c = self.class.unwrap();
                        let f = self.field_of(c, n);
                        format!("{{ let v = this.borrow().{f}.clone(); v }}")
                    }
                    Some(NameRes::Object) => {
                        let ci = self.info.class_index[n];
                        format!("Some(obj{ci}())")
                    }
                    Some(NameRes::TopMethod) => {
                        let sig = self.info.methods[n].clone();
                        let (params, args, types) = self.closure_params(&sig.params);
                        let r = self.ty(&sig.ret);
                        format!("{{ let f: Rc<dyn Fn({types}) -> {r}> = Rc::new(move |{params}| t_{n}({args})); Some(f) }}")
                    }
                    Some(NameRes::OwnMethod) => {
                        let c = self.class.unwrap();
                        let sig = self.info.method(c, n).cloned().unwrap();
                        let root = self.root(c);
                        let (params, args, types) = self.closure_params(&sig.params);
                        let r = self.ty(&sig.ret);
                        let a = if args.is_empty() { "&t".to_string() } else { format!("&t, {args}") };
                        format!("{{ let t = this.clone(); let f: Rc<dyn Fn({types}) -> {r}> = Rc::new(move |{params}| d{root}_{n}({a})); Some(f) }}")
                    }
                    _ => format!("v_{n}.clone()"),
                }
            }
            ExprKind::Field(t, f, _) => {
                if let Some((_, path)) = self.info.options.get(&e.id) {
                    return format!("{path:?}");
                }
                let tc = self.expr(t);
                match self.info.ty(t) {
                    Ty::Row(_) => {
                        let field = format!("f_{f}");
                        format!("{{ let o = {tc}; let v = o.borrow().{field}.clone(); v }}")
                    }
                    Ty::Class(c) => {
                        let fname = self.field_of(c, f);
                        format!("{{ let o = rt::obj(&{tc}); let v = o.borrow().{fname}.clone(); v }}")
                    }
                    _ => "()".into(),
                }
            }
            ExprKind::MethodRef(t, m, _) => {
                let tc = self.expr(t);
                let Ty::Class(c) = self.info.ty(t) else { return "None".into() };
                let sig = self.info.method(c, m).cloned().unwrap();
                let root = self.root(c);
                let (params, args, types) = self.closure_params(&sig.params);
                let r = self.ty(&sig.ret);
                let a = if args.is_empty() { "&t".to_string() } else { format!("&t, {args}") };
                format!(
                    "{{ let t = rt::obj(&{tc}); let f: Rc<dyn Fn({types}) -> {r}> = Rc::new(move |{params}| d{root}_{m}({a})); Some(f) }}"
                )
            }
            ExprKind::Method(t, m, args, _) => {
                let tt = self.info.ty(t);
                let tc = self.expr(t);
                match self.info.calls.get(&e.id) {
                    Some(CallRes::Builtin) => match (&tt, m.as_str()) {
                        (Ty::Array(el), "inject") => {
                            let v = self.coerced(&args[0], &el.clone());
                            format!("rt::ainject(&{tc}, {v})")
                        }
                        (Ty::Array(_), "remove") => format!("rt::aremove(&{tc}, {})", self.expr(&args[0])),
                        (Ty::Map(_), "has") => format!("rt::mhas(&{tc}, &{})", self.expr(&args[0])),
                        (Ty::Map(_), "remove") => format!("rt::mremove(&{tc}, &{})", self.expr(&args[0])),
                        (Ty::CMap(cols), "inject") => {
                            let cols = cols.clone();
                            let row = self.row(&args[0], &cols);
                            format!("rt::cinject(&{tc}, {row})")
                        }
                        (Ty::CMap(_), "has") => format!("rt::chas(&{tc}, {})", self.expr(&args[0])),
                        (Ty::CMap(_), "remove") => format!("rt::cremove(&{tc}, {})", self.expr(&args[0])),
                        _ => "()".into(),
                    },
                    Some(CallRes::FieldHook) => {
                        let Ty::Class(c) = tt else { return "()".into() };
                        let fname = self.field_of(c, m);
                        let Some(Ty::Hook(sig)) = self.info.field(c, m).map(|f| f.ty.clone()) else {
                            return "()".into();
                        };
                        let a = self.args(args, &sig.params.iter().map(|(t, _)| t.clone()).collect::<Vec<_>>());
                        format!("{{ let o = rt::obj(&{tc}); let h = rt::hk(&o.borrow().{fname}); h({a}) }}")
                    }
                    _ => {
                        let Ty::Class(c) = tt else { return "()".into() };
                        let sig = self.info.method(c, m).cloned().unwrap();
                        let root = self.root(c);
                        let a = self.args(args, &sig.params.iter().map(|(_, t)| t.clone()).collect::<Vec<_>>());
                        let all = if a.is_empty() { "&o".to_string() } else { format!("&o, {a}") };
                        format!("{{ let o = rt::obj(&{tc}); d{root}_{m}({all}) }}")
                    }
                }
            }
            ExprKind::Call(name, args) => match self.info.calls.get(&e.id) {
                Some(CallRes::Builtin) => self.builtin(name, args),
                Some(CallRes::Top) => {
                    let sig = self.info.methods[name].clone();
                    let a = self.args(args, &sig.params.iter().map(|(_, t)| t.clone()).collect::<Vec<_>>());
                    format!("t_{name}({a})")
                }
                Some(CallRes::Own) => {
                    let c = self.class.unwrap();
                    let sig = self.info.method(c, name).cloned().unwrap();
                    let root = self.root(c);
                    let a = self.args(args, &sig.params.iter().map(|(_, t)| t.clone()).collect::<Vec<_>>());
                    let all = if a.is_empty() { "this".to_string() } else { format!("this, {a}") };
                    format!("d{root}_{name}({all})")
                }
                Some(CallRes::FieldHook) => {
                    let c = self.class.unwrap();
                    let fname = self.field_of(c, name);
                    let Some(Ty::Hook(sig)) = self.info.field(c, name).map(|f| f.ty.clone()) else {
                        return "()".into();
                    };
                    let a = self.args(args, &sig.params.iter().map(|(t, _)| t.clone()).collect::<Vec<_>>());
                    format!("{{ let h = rt::hk(&this.borrow().{fname}); h({a}) }}")
                }
                Some(CallRes::LocalHook) => {
                    let Some(Ty::Hook(sig)) = self.local_hook_type(e) else {
                        return "()".into();
                    };
                    let a = self.args(args, &sig.params.iter().map(|(t, _)| t.clone()).collect::<Vec<_>>());
                    format!("{{ let h = rt::hk(&v_{name}); h({a}) }}")
                }
                None => "()".into(),
            },
            ExprKind::Index(t, i) => {
                let tc = self.expr(t);
                let ic = self.expr(i);
                match self.info.ty(t) {
                    Ty::Map(_) => format!("rt::mget(&{tc}, &{ic})"),
                    Ty::CMap(_) => format!("rt::cget(&{tc}, {ic})"),
                    _ => format!("rt::aget(&{tc}, {ic})"),
                }
            }
            ExprKind::Len(inner) => {
                let c = self.expr(inner);
                match self.info.ty(inner) {
                    Ty::Str => format!("rt::slen(&{c})"),
                    Ty::Map(_) => format!("rt::mlen(&{c})"),
                    Ty::CMap(_) => format!("rt::clen(&{c})"),
                    _ => format!("rt::alen(&{c})"),
                }
            }
            ExprKind::Neg(inner) => format!("(-{})", self.expr(inner)),
            ExprKind::Not(inner) => format!("(!{})", self.expr(inner)),
            ExprKind::Bin(op, l, r, _) => self.binary(e, *op, l, r),
            ExprKind::Either(c, a, b) => {
                let want = self.info.ty(e);
                let cc = self.expr(c);
                let ac = self.coerced(a, &want);
                let bc = self.coerced(b, &want);
                format!("(if {cc} {{ {ac} }} else {{ {bc} }})")
            }
            ExprKind::List { items, keys } => match self.info.ty(e) {
                Ty::Array(el) => {
                    let et = self.ty(&el);
                    let vs: Vec<String> = items.iter().map(|i| self.coerced(i, &el)).collect();
                    format!("rt::arr::<{et}>(vec![{}])", vs.join(", "))
                }
                Ty::Map(v) => {
                    let vt = self.ty(&v);
                    let ks: Vec<String> = keys.as_deref().unwrap_or(&[]).iter().map(|k| self.expr(k)).collect();
                    let vs: Vec<String> = items.iter().map(|i| self.coerced(i, &v)).collect();
                    let pairs: Vec<String> = ks.iter().zip(&vs).map(|(k, v)| format!("({k}, {v})")).collect();
                    format!("rt::map::<{vt}>(vec![{}])", pairs.join(", "))
                }
                Ty::CMap(cols) => {
                    let ri = self.row_index(&cols);
                    let rows: Vec<String> = items.iter().map(|r| self.row(r, &cols)).collect();
                    let maker = match self.maker.take() {
                        Some(m) => format!("Some({m})"),
                        None => "None".into(),
                    };
                    format!("rt::carr::<Row{ri}>({:?}, {maker}, vec![{}])", cols[0].0, rows.join(", "))
                }
                _ => "()".into(),
            },
            ExprKind::New { class, inits, .. } => {
                let Some(&ci) = self.info.class_index.get(class) else {
                    return "None".into();
                };
                let mut s = format!("{{ let o = new{ci}(); ");
                for (f, _, v) in inits {
                    let want = self.info.field(ci, f).map(|x| x.ty.clone()).unwrap_or(Ty::Error);
                    let vc = self.coerced(v, &want);
                    let fname = self.field_of(ci, f);
                    s.push_str(&format!("{{ let v = {vc}; o.borrow_mut().{fname} = v; }} "));
                }
                s.push_str("Some(o) }");
                s
            }
            ExprKind::NewHook(body) => self.new_hook(e, body),
            ExprKind::Like(inner, class, _) => {
                let c = self.expr(inner);
                let Some(&target) = self.info.class_index.get(class) else {
                    return "false".into();
                };
                if !matches!(self.info.ty(inner), Ty::Class(_)) {
                    return "false".into();
                }
                let ids: Vec<String> = (0..self.info.classes.len())
                    .filter(|&x| self.info.is_like(x, target))
                    .map(|x| format!("ID_{x}"))
                    .collect();
                format!("rt::like(&{c}, &[{}])", ids.join(", "))
            }
            ExprKind::Convert(inner, conv, _) => {
                let t = self.info.ty(inner);
                let c = self.expr(inner);
                match conv {
                    Conv::Text => text(&c, &t),
                    Conv::Int => match t {
                        Ty::Str => format!("rt::sint(&{c})"),
                        _ => format!("rt::toint({c})"),
                    },
                    Conv::Float => match t {
                        Ty::Str => format!("rt::sfloat(&{c})"),
                        _ => format!("({c} as f64)"),
                    },
                    Conv::IsNumber => format!("rt::isnum(&{c})"),
                    Conv::Lines => format!("rt::lines(&{c})"),
                }
            }
        }
    }

    fn local_hook_type(&self, e: &Expr) -> Option<Ty> {
        // A local hook's type: the call's own type tells what it gives back, the arguments what it takes.
        if let ExprKind::Call(_, args) = &e.kind {
            let ret = self.info.ty(e);
            let params = args.iter().map(|a| (self.info.ty(a), None)).collect();
            // The real parameter types come from the declaration; the checker made the arguments fit them.
            return self
                .info
                .decl_types
                .get(&e.id)
                .cloned()
                .or(Some(Ty::Hook(std::rc::Rc::new(crate::types::HookSig { ret, params }))));
        }
        None
    }

    fn closure_params(&mut self, params: &[(String, Ty)]) -> (String, String, String) {
        let mut p = Vec::new();
        let mut a = Vec::new();
        let mut t = Vec::new();
        for (i, (_, ty)) in params.iter().enumerate() {
            let rt = self.ty(ty);
            p.push(format!("a{i}: {rt}"));
            a.push(format!("a{i}"));
            t.push(rt);
        }
        (p.join(", "), a.join(", "), t.join(", "))
    }

    fn args(&mut self, args: &[Expr], types: &[Ty]) -> String {
        let v: Vec<String> = args.iter().zip(types).map(|(a, t)| self.coerced(a, t)).collect();
        v.join(", ")
    }

    fn row(&mut self, r: &Expr, cols: &[(String, Ty)]) -> String {
        let ri = self.row_index(cols);
        let ExprKind::List { items, .. } = &r.kind else {
            return "()".into();
        };
        let with_key = items.len() == cols.len();
        let start = if with_key { 1 } else { 0 };
        let key = if with_key {
            format!("Some({})", self.expr(&items[0]))
        } else {
            "None".into()
        };
        let mut fields = Vec::new();
        for (i, (n, t)) in cols.iter().enumerate().skip(1) {
            let v = self.coerced(&items[start + i - 1], t);
            fields.push(format!("f_{n}: {v}, "));
        }
        format!("({key}, Row{ri} {{ {}..Default::default() }})", fields.concat())
    }

    fn new_hook(&mut self, e: &Expr, body: &[Stmt]) -> String {
        let Ty::Hook(sig) = self.info.ty(e) else { return "None".into() };
        let names = self.info.hook_params.get(&e.id).cloned().unwrap_or_default();
        let captured = captures(body, &names, self.info);
        let uses_this = self.class.is_some() && has_this(body, self.info);
        let mut s = String::from("{ ");
        for c in &captured {
            s.push_str(&format!("let v_{c} = v_{c}.clone(); "));
        }
        if uses_this {
            s.push_str("let this = this.clone(); ");
        }
        let params: Vec<String> = names
            .iter()
            .zip(&sig.params)
            .map(|(n, (t, _))| format!("mut v_{n}: {}", self.ty(t)))
            .collect();
        let types: Vec<String> = sig.params.iter().map(|(t, _)| self.ty(t)).collect();
        let r = self.ty(&sig.ret);
        let saved_ret = self.current_ret.replace(sig.ret.clone());
        let mut b = String::new();
        if uses_this {
            b.push_str("let this = &this;\n");
        }
        b.push_str("let _depth = rt::Depth::enter();\n");
        self.stmts(body, &mut b, 0);
        if sig.ret != Ty::Void {
            b.push_str("rt::unreachable()\n");
        }
        self.current_ret = saved_ret;
        s.push_str(&format!(
            "let f: Rc<dyn Fn({}) -> {r}> = Rc::new(move |{}| -> {r} {{\n{b}}}); Some(f) }}",
            types.join(", "),
            params.join(", ")
        ));
        s
    }

    fn builtin(&mut self, name: &str, args: &[Expr]) -> String {
        match name {
            "print" => {
                let t = self.info.ty(&args[0]);
                let c = self.expr(&args[0]);
                let shown = self.show(&c, &t);
                format!("rt::print({shown})")
            }
            "random" => format!("rt::random({}, {})", self.expr(&args[0]), self.expr(&args[1])),
            "ask" => format!("rt::ask({})", self.expr(&args[0])),
            "readText" => format!("rt::read_text({})", self.expr(&args[0])),
            "writeText" => format!("rt::write_text({}, {})", self.expr(&args[0]), self.expr(&args[1])),
            "wait" => format!("rt::wait({})", self.coerced(&args[0], &Ty::Float)),
            _ => "()".into(),
        }
    }

    /// Code that shows a value as text, the way print does.
    fn show(&mut self, code: &str, t: &Ty) -> String {
        match t {
            Ty::Str => code.to_string(),
            Ty::State(_) => format!("{code}.to_string()"),
            _ => {
                let f = self.fmt(t, false);
                format!("({f})(&{code})")
            }
        }
    }

    /// A closure taking &T and giving its text. Inside lists, text is in quotes.
    fn fmt(&mut self, t: &Ty, quoted: bool) -> String {
        let tt = self.ty(t);
        match t {
            Ty::Int => format!("|v: &{tt}| rt::itext(*v)"),
            Ty::Float => format!("|v: &{tt}| rt::ftext(*v)"),
            Ty::Truth => format!("|v: &{tt}| rt::btext(*v)"),
            Ty::Str if quoted => format!("|v: &{tt}| rt::qtext(v)"),
            Ty::Str => format!("|v: &{tt}| v.clone()"),
            Ty::State(_) => format!("|v: &{tt}| v.to_string()"),
            Ty::Class(c) => format!("|v: &{tt}| show{}(v)", self.root(*c)),
            Ty::Hook(_) => format!("|v: &{tt}| rt::hshow(v)"),
            Ty::Array(el) => {
                let inner = self.fmt(el, true);
                format!("|v: &{tt}| rt::ashow(v, {inner})")
            }
            Ty::Map(el) => {
                let inner = self.fmt(el, true);
                format!("|v: &{tt}| rt::mshow(v, {inner})")
            }
            Ty::CMap(cols) | Ty::Row(cols) => {
                let ri = self.row_index(cols);
                let mut parts: Vec<String> = Vec::new();
                for (n, ct) in cols.iter() {
                    let f = self.fmt(ct, true);
                    parts.push(format!("({f})(&r.f_{n})"));
                }
                let row = format!("|r: &Row{ri}| format!(\"[{{}}]\", vec![{}].join(\", \"))", parts.join(", "));
                if matches!(t, Ty::CMap(_)) {
                    format!("|v: &{tt}| rt::cshow(v, {row})")
                } else {
                    format!("|v: &{tt}| ({row})(&v.borrow())")
                }
            }
            _ => format!("|_v: &{tt}| String::from(\"?\")"),
        }
    }

    fn binary(&mut self, _e: &Expr, op: BinOp, l: &Expr, r: &Expr) -> String {
        let lt = self.info.ty(l);
        let rt = self.info.ty(r);
        let numeric = lt.is_number() && rt.is_number();
        let floaty = numeric && (lt == Ty::Float || rt == Ty::Float);
        let (lc, rc) = if floaty {
            (self.coerced(l, &Ty::Float), self.coerced(r, &Ty::Float))
        } else if matches!(op, BinOp::Eq | BinOp::Ne | BinOp::Same | BinOp::NotSame) && (lt == Ty::Nothing || rt == Ty::Nothing) {
            // Comparing with nothing: give both sides the other's type.
            let t = if lt == Ty::Nothing { rt.clone() } else { lt.clone() };
            (self.coerced(l, &t), self.coerced(r, &t))
        } else {
            (self.expr(l), self.expr(r))
        };
        match op {
            BinOp::Add if lt == Ty::Str => format!("rt::cat({lc}, {rc})"),
            BinOp::Add => format!("({lc} + {rc})"),
            BinOp::Sub => format!("({lc} - {rc})"),
            BinOp::Mul => format!("({lc} * {rc})"),
            BinOp::Div if !floaty => format!("rt::idiv({lc}, {rc})"),
            BinOp::Div => format!("({lc} / {rc})"),
            BinOp::Mod if !floaty => format!("rt::imod({lc}, {rc})"),
            BinOp::Mod => format!("({lc} % {rc})"),
            BinOp::Lt => format!("({lc} < {rc})"),
            BinOp::Gt => format!("({lc} > {rc})"),
            BinOp::Le => format!("({lc} <= {rc})"),
            BinOp::Ge => format!("({lc} >= {rc})"),
            BinOp::And => format!("({lc} && {rc})"),
            BinOp::Or => format!("({lc} || {rc})"),
            BinOp::Eq | BinOp::Ne | BinOp::Same | BinOp::NotSame => {
                let negate = matches!(op, BinOp::Ne | BinOp::NotSame);
                let t = if lt == Ty::Nothing { &rt } else { &lt };
                let s = match t {
                    Ty::State(_) if matches!(op, BinOp::Eq | BinOp::Ne) => format!("rt::sin({lc}, {rc})"),
                    Ty::Class(_) | Ty::Hook(_) | Ty::Nothing => format!("rt::same(&{lc}, &{rc})"),
                    Ty::Array(_) | Ty::Map(_) | Ty::CMap(_) => format!("Rc::ptr_eq(&{lc}, &{rc})"),
                    Ty::Row(_) => format!("Rc::ptr_eq(&{lc}, &{rc})"),
                    _ => format!("({lc} == {rc})"),
                };
                if negate {
                    format!("(!{s})")
                } else {
                    s
                }
            }
        }
    }
}

/// Text for .text(): numbers, truths, states and text itself.
fn text(code: &str, t: &Ty) -> String {
    match t {
        Ty::Int => format!("rt::itext({code})"),
        Ty::Float => format!("rt::ftext({code})"),
        Ty::Truth => format!("rt::btext({code})"),
        Ty::State(_) => format!("{code}.to_string()"),
        _ => code.to_string(),
    }
}

fn coerce(code: String, got: &Ty, want: &Ty) -> String {
    match (got, want) {
        (Ty::Int, Ty::Float) => format!("({code} as f64)"),
        _ => code,
    }
}
