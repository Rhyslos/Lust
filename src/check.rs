//! Checks a program before it runs: every name known, every type fitting, every
//! method giving back what it promises. Also records what the Lua writer needs to know.

use std::collections::HashMap;
use std::rc::Rc;

use crate::ast::*;
use crate::diag::Diags;
use crate::parser::BUILTINS;
use crate::types::{HookSig, Names, Ty};

pub struct StateInfo {
    pub name: String,
    /// Every option's full path, in the order written: "MainMenu", "ActiveGame", "ActiveGame.Combat".
    pub options: Vec<String>,
}

pub struct FieldInfo {
    pub name: String,
    pub ty: Ty,
}

#[derive(Clone)]
pub struct MethodSig {
    pub params: Vec<(String, Ty)>,
    pub ret: Ty,
}

pub struct ClassInfo {
    pub name: String,
    pub is_object: bool,
    pub parent: Option<usize>,
    pub fields: Vec<FieldInfo>,
    pub methods: HashMap<String, MethodSig>,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum NameRes {
    Local,
    Field,
    Object,
    StateList,
    TopMethod,
    OwnMethod,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum CallRes {
    Builtin,
    Top,
    Own,
    LocalHook,
    FieldHook,
}

#[derive(Default)]
pub struct Info {
    pub types: HashMap<usize, Ty>,
    pub names: HashMap<usize, NameRes>,
    pub calls: HashMap<usize, CallRes>,
    /// Expressions that name a state option: the state and the option's path.
    pub options: HashMap<usize, (usize, String)>,
    /// The names a `new hook { }` gives its values.
    pub hook_params: HashMap<usize, Vec<String>>,
    /// Declared types: a variable's, keyed by its value; a called local hook's, keyed by the call.
    pub decl_types: HashMap<usize, Ty>,
    pub states: Vec<StateInfo>,
    pub classes: Vec<ClassInfo>,
    pub methods: HashMap<String, MethodSig>,
    pub class_index: HashMap<String, usize>,
    pub state_index: HashMap<String, usize>,
}

impl Info {
    pub fn ty(&self, e: &Expr) -> Ty {
        self.types.get(&e.id).cloned().unwrap_or(Ty::Error)
    }

    pub fn field(&self, class: usize, name: &str) -> Option<&FieldInfo> {
        let mut c = Some(class);
        while let Some(i) = c {
            if let Some(f) = self.classes[i].fields.iter().find(|f| f.name == name) {
                return Some(f);
            }
            c = self.classes[i].parent;
        }
        None
    }

    pub fn method(&self, class: usize, name: &str) -> Option<&MethodSig> {
        let mut c = Some(class);
        while let Some(i) = c {
            if let Some(m) = self.classes[i].methods.get(name) {
                return Some(m);
            }
            c = self.classes[i].parent;
        }
        None
    }

    pub fn is_like(&self, class: usize, ancestor: usize) -> bool {
        let mut c = Some(class);
        while let Some(i) = c {
            if i == ancestor {
                return true;
            }
            c = self.classes[i].parent;
        }
        false
    }
}

#[derive(Clone)]
struct Local {
    ty: Ty,
    readonly: bool,
    pos: Pos,
}

#[derive(Clone)]
struct Ctx {
    class: Option<usize>,
    /// What the current method gives back. None at the top level of the file.
    ret: Option<Ty>,
    scopes: Vec<HashMap<String, Local>>,
    in_try: bool,
    in_hook: bool,
    /// Inside a `new hook { }`: the scopes from this index on belong to the hook.
    hook_base: usize,
}

pub struct Checker<'a> {
    pub info: Info,
    diags: &'a mut Diags,
    top_vars: HashMap<String, Pos>,
    class_names: Vec<String>,
    state_names: Vec<String>,
}

pub fn check(prog: &Program, diags: &mut Diags, is_script: bool) -> Info {
    let mut c = Checker {
        info: Info::default(),
        diags,
        top_vars: HashMap::new(),
        class_names: vec![],
        state_names: vec![],
    };
    c.run(prog, is_script);
    c.info
}

fn suggest(name: &str, candidates: impl IntoIterator<Item = String>) -> String {
    let lower = name.to_lowercase();
    let mut best: Option<(usize, String)> = None;
    for c in candidates {
        let d = distance(&lower, &c.to_lowercase());
        let limit = if name.len() <= 3 { 1 } else { 2 };
        if d <= limit && best.as_ref().map(|(bd, _)| d < *bd).unwrap_or(true) {
            best = Some((d, c));
        }
    }
    best.map(|(_, c)| format!(" Did you mean {c}?")).unwrap_or_default()
}

fn distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for i in 1..=a.len() {
        let mut cur = vec![i; b.len() + 1];
        for j in 1..=b.len() {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            cur[j] = (prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + cost);
        }
        prev = cur;
    }
    prev[b.len()]
}

fn describe(e: &Expr) -> String {
    match &e.kind {
        ExprKind::Name(n) => n.clone(),
        ExprKind::Field(t, f, _) => format!("{}.{f}", describe(t)),
        ExprKind::Index(t, _) => format!("{}[…]", describe(t)),
        ExprKind::Call(n, _) => format!("{n}(…)"),
        ExprKind::Method(_, m, _, _) => format!("{m}(…)"),
        _ => "this".into(),
    }
}

impl<'a> Checker<'a> {
    fn err(&mut self, pos: Pos, msg: impl Into<String>) {
        self.diags.add(pos, msg);
    }

    fn names(&self) -> Names<'_> {
        Names {
            classes: &self.class_names,
            states: &self.state_names,
        }
    }
    fn show(&self, t: &Ty) -> String {
        self.names().show(t)
    }
    fn a(&self, t: &Ty) -> String {
        self.names().with_article(t)
    }

    // ----- declarations -----

    fn run(&mut self, prog: &Program, _is_script: bool) {
        // 1. Names of states, classes and methods, so everything can refer to everything.
        for item in &prog.items {
            match item {
                Item::State(s) => {
                    if self.name_taken(&s.name, s.pos) {
                        continue;
                    }
                    let mut options = Vec::new();
                    fn walk(opts: &[StateOption], prefix: &str, out: &mut Vec<String>) {
                        for o in opts {
                            let path = if prefix.is_empty() {
                                o.name.clone()
                            } else {
                                format!("{prefix}.{}", o.name)
                            };
                            out.push(path.clone());
                            walk(&o.children, &path, out);
                        }
                    }
                    walk(&s.options, "", &mut options);
                    self.info.state_index.insert(s.name.clone(), self.info.states.len());
                    self.state_names.push(s.name.clone());
                    self.info.states.push(StateInfo {
                        name: s.name.clone(),
                        options,
                    });
                }
                Item::Class(c) => {
                    if self.name_taken(&c.name, c.pos) {
                        continue;
                    }
                    self.info.class_index.insert(c.name.clone(), self.info.classes.len());
                    self.class_names.push(c.name.clone());
                    self.info.classes.push(ClassInfo {
                        name: c.name.clone(),
                        is_object: c.is_object,
                        parent: None,
                        fields: vec![],
                        methods: HashMap::new(),
                    });
                }
                Item::Method(m) => {
                    if self.name_taken(&m.name, m.pos) {
                        continue;
                    }
                    self.info.methods.insert(
                        m.name.clone(),
                        MethodSig {
                            params: vec![],
                            ret: Ty::Void,
                        },
                    );
                }
            }
        }
        // 2. Parents, then signatures and fields.
        for item in &prog.items {
            if let Item::Class(c) = item {
                let Some(&ci) = self.info.class_index.get(&c.name) else { continue };
                if let Some((p, pos)) = &c.parent {
                    match self.info.class_index.get(p) {
                        Some(&pi) if self.info.classes[pi].is_object => {
                            self.err(*pos, format!("{p} is an object. A class can only be like another class."))
                        }
                        Some(&pi) => self.info.classes[ci].parent = Some(pi),
                        None => {
                            let s = suggest(p, self.class_names.clone());
                            self.err(*pos, format!("there's no class called {p}.{s}"))
                        }
                    }
                }
            }
        }
        // A class can't be like itself, even through others.
        for i in 0..self.info.classes.len() {
            let mut seen = vec![i];
            let mut c = self.info.classes[i].parent;
            while let Some(p) = c {
                if seen.contains(&p) {
                    let name = self.info.classes[i].name.clone();
                    let pos = prog.items.iter().find_map(|it| match it {
                        Item::Class(c) if c.name == name => Some(c.pos),
                        _ => None,
                    });
                    self.err(
                        pos.unwrap_or_default(),
                        format!("{name} ends up being like itself. Break the circle of like."),
                    );
                    self.info.classes[i].parent = None;
                    break;
                }
                seen.push(p);
                c = self.info.classes[p].parent;
            }
        }
        for item in &prog.items {
            match item {
                Item::Method(m) => {
                    let sig = self.signature(m);
                    self.info.methods.insert(m.name.clone(), sig);
                }
                Item::Class(c) => {
                    let Some(&ci) = self.info.class_index.get(&c.name) else { continue };
                    for m in &c.methods {
                        let sig = self.signature(m);
                        if self.info.classes[ci].methods.contains_key(&m.name) {
                            self.err(m.pos, format!("{} already has a method called {}.", c.name, m.name));
                        }
                        self.info.classes[ci].methods.insert(m.name.clone(), sig);
                    }
                }
                _ => {}
            }
        }
        // Fields go in parent-first order, so a class can see its parent's fields.
        let order = self.class_order(prog);
        for c in &order {
            let Some(&ci) = self.info.class_index.get(&c.name) else { continue };
            for f in &c.fields {
                let ty = match &f.ty {
                    Some(t) => self.resolve(t),
                    None => {
                        // state is GameState.Idle;
                        let mut ctx = Ctx {
                            class: None,
                            ret: None,
                            scopes: vec![HashMap::new()],
                            in_try: false,
                            in_hook: false,
                            hook_base: 0,
                        };
                        let d = f.default.as_ref().unwrap();
                        match self.expr(d, &mut ctx, None) {
                            Ty::State(s) if self.info.options.contains_key(&d.id) => Ty::State(s),
                            Ty::Error => Ty::Error,
                            _ => {
                                self.err(d.pos, "state is … needs one of a state's options: state is Mood.Happy;");
                                Ty::Error
                            }
                        }
                    }
                };
                if let Some(parent) = self.info.classes[ci].parent {
                    if let Some(pf) = self.info.field(parent, &f.name) {
                        if pf.ty != ty && ty != Ty::Error {
                            let (pn, pt) = (self.info.classes[parent].name.clone(), self.show(&pf.ty.clone()));
                            self.err(
                                f.pos,
                                format!(
                                    "{pn} already has {} as {pt}. A class like {pn} can change its starting value, but not its type.",
                                    f.name
                                ),
                            );
                        }
                        // Same field again: just a new starting value.
                        continue;
                    }
                }
                if self.info.classes[ci].fields.iter().any(|x| x.name == f.name) {
                    self.err(f.pos, format!("{} already has a field called {}.", c.name, f.name));
                    continue;
                }
                if self.info.method(ci, &f.name).is_some() {
                    self.err(
                        f.pos,
                        format!("{} has a method called {}, so a field can't use that name.", c.name, f.name),
                    );
                }
                self.info.classes[ci].fields.push(FieldInfo { name: f.name.clone(), ty });
            }
        }
        // Methods that replace a parent's must fit the same way.
        for c in &order {
            let Some(&ci) = self.info.class_index.get(&c.name) else { continue };
            let Some(parent) = self.info.classes[ci].parent else { continue };
            for m in &c.methods {
                if let Some(pm) = self.info.method(parent, &m.name).cloned() {
                    let mine = self.info.classes[ci].methods[&m.name].clone();
                    let same = pm.ret == mine.ret
                        && pm.params.len() == mine.params.len()
                        && pm.params.iter().zip(&mine.params).all(|(a, b)| a.1 == b.1);
                    if !same {
                        let pn = self.info.classes[parent].name.clone();
                        self.err(
                            m.pos,
                            format!(
                                "{} replaces {pn}'s {}, so it must take the same types and give back the same type.",
                                m.name, m.name
                            ),
                        );
                    }
                }
            }
        }
        // 3. Field starting values.
        for c in &order {
            let Some(&ci) = self.info.class_index.get(&c.name) else { continue };
            for f in &c.fields {
                if f.ty.is_none() {
                    continue;
                }
                let want = self.info.field(ci, &f.name).map(|x| x.ty.clone()).unwrap_or(Ty::Error);
                match &f.default {
                    Some(d) => {
                        let mut ctx = Ctx {
                            class: None,
                            ret: None,
                            scopes: vec![HashMap::new()],
                            in_try: false,
                            in_hook: false,
                            hook_base: 0,
                        };
                        let got = self.expr(d, &mut ctx, Some(&want));
                        self.expect_fits(d, &got, &want, &format!("{} is {}", f.name, self.a(&want)));
                    }
                    None => {
                        if !want.can_be_nothing() && want != Ty::Error {
                            let example = self.example(&want);
                            self.err(
                                f.pos,
                                format!("give {} a starting value: {} {} = {example};", f.name, self.show(&want), f.name),
                            );
                        }
                    }
                }
            }
        }
        // 4. The top level of the file, then every method.
        let mut top = Ctx {
            class: None,
            ret: None,
            scopes: vec![HashMap::new()],
            in_try: false,
            in_hook: false,
            hook_base: 0,
        };
        self.block_in(&prog.body, &mut top, true);
        for (k, v) in &top.scopes[0] {
            self.top_vars.insert(k.clone(), v.pos);
        }
        for item in &prog.items {
            match item {
                Item::Method(m) => self.method_body(m, None),
                Item::Class(c) => {
                    let ci = self.info.class_index.get(&c.name).copied();
                    for m in &c.methods {
                        self.method_body(m, ci);
                    }
                }
                _ => {}
            }
        }
    }

    fn class_order<'p>(&self, prog: &'p Program) -> Vec<&'p ClassDecl> {
        let classes: Vec<&ClassDecl> = prog
            .items
            .iter()
            .filter_map(|i| match i {
                Item::Class(c) => Some(c),
                _ => None,
            })
            .collect();
        let mut out: Vec<&ClassDecl> = Vec::new();
        let mut placed = std::collections::HashSet::new();
        while out.len() < classes.len() {
            let before = out.len();
            for c in &classes {
                if placed.contains(&c.name) {
                    continue;
                }
                let ready = match &c.parent {
                    Some((p, _)) => placed.contains(p) || !classes.iter().any(|x| &x.name == p),
                    None => true,
                };
                if ready {
                    placed.insert(c.name.clone());
                    out.push(c);
                }
            }
            if out.len() == before {
                for c in &classes {
                    if !placed.contains(&c.name) {
                        placed.insert(c.name.clone());
                        out.push(c);
                    }
                }
            }
        }
        out
    }

    /// Classes, states and methods share one set of names.
    fn name_taken(&mut self, name: &str, pos: Pos) -> bool {
        let what = if self.info.class_index.contains_key(name) {
            let c = &self.info.classes[self.info.class_index[name]];
            Some(if c.is_object { "an object" } else { "a class" })
        } else if self.info.state_index.contains_key(name) {
            Some("a state")
        } else if self.info.methods.contains_key(name) {
            Some("a method")
        } else {
            None
        };
        if let Some(w) = what {
            self.err(pos, format!("there's already {w} called {name}. Two things can't share a name."));
            return true;
        }
        false
    }

    fn signature(&mut self, m: &MethodDecl) -> MethodSig {
        let params = m.params.iter().map(|p| (p.name.clone(), self.resolve(&p.ty))).collect();
        let ret = m.ret.as_ref().map(|r| self.resolve(r)).unwrap_or(Ty::Void);
        MethodSig { params, ret }
    }

    fn resolve(&mut self, t: &TypeRef) -> Ty {
        match t {
            TypeRef::Named(n, pos) => match n.as_str() {
                "int" => Ty::Int,
                "float" => Ty::Float,
                "string" => Ty::Str,
                "truth" => Ty::Truth,
                _ => {
                    if let Some(&c) = self.info.class_index.get(n) {
                        return Ty::Class(c);
                    }
                    if let Some(&s) = self.info.state_index.get(n) {
                        return Ty::State(s);
                    }
                    let known: Vec<String> = self
                        .class_names
                        .iter()
                        .chain(self.state_names.iter())
                        .cloned()
                        .chain(["int", "float", "string", "truth"].iter().map(|s| s.to_string()))
                        .collect();
                    let s = suggest(n, known);
                    self.err(*pos, format!("Lust doesn't have a type called {n}.{s} The types are int, float, string, truth, arrays, maps, cArrays, hooks, classes and states."));
                    Ty::Error
                }
            },
            TypeRef::Array(e) => Ty::Array(Box::new(self.resolve(e))),
            TypeRef::Map(e) => Ty::Map(Box::new(self.resolve(e))),
            TypeRef::CArray(cols) => Ty::CArray(Rc::new(cols.iter().map(|c| (c.name.clone(), self.resolve(&c.ty))).collect())),
            TypeRef::Hook { ret, params } => {
                let ret = ret.as_ref().map(|r| self.resolve(r)).unwrap_or(Ty::Void);
                let params = params.iter().map(|p| (self.resolve(&p.ty), p.name.clone())).collect();
                Ty::Hook(Rc::new(HookSig { ret, params }))
            }
        }
    }

    fn example(&self, t: &Ty) -> String {
        match t {
            Ty::Int => "0".into(),
            Ty::Float => "1.5".into(),
            Ty::Str => "\"text\"".into(),
            Ty::Truth => "true".into(),
            Ty::Array(_) | Ty::Map(_) | Ty::CArray(_) => "[]".into(),
            Ty::State(s) => format!("{}.{}", self.info.states[*s].name, self.info.states[*s].options[0]),
            _ => "nothing".into(),
        }
    }

    fn method_body(&mut self, m: &MethodDecl, class: Option<usize>) {
        let sig = match class {
            Some(c) => self.info.classes[c].methods.get(&m.name).cloned(),
            None => self.info.methods.get(&m.name).cloned(),
        };
        let Some(sig) = sig else { return };
        let mut ctx = Ctx {
            class,
            ret: Some(sig.ret.clone()),
            scopes: vec![HashMap::new()],
            in_try: false,
            in_hook: false,
            hook_base: 0,
        };
        for (p, (_, ty)) in m.params.iter().zip(&sig.params) {
            if let Some(c) = class {
                if self.info.field(c, &p.name).is_some() {
                    let cn = self.info.classes[c].name.clone();
                    self.err(
                        p.pos,
                        format!("{cn} has a field called {}, so a parameter can't use that name.", p.name),
                    );
                }
            }
            ctx.scopes[0].insert(
                p.name.clone(),
                Local {
                    ty: ty.clone(),
                    readonly: false,
                    pos: p.pos,
                },
            );
        }
        self.block_in(&m.body, &mut ctx, false);
        if sig.ret != Ty::Void && sig.ret != Ty::Error && !returns(&m.body) {
            self.err(
                m.pos,
                format!(
                    "{} must return {} on every path, but it can reach its end without one.",
                    m.name,
                    self.a(&sig.ret)
                ),
            );
        }
    }

    // ----- statements -----

    fn block_in(&mut self, stmts: &[Stmt], ctx: &mut Ctx, same_scope: bool) {
        if !same_scope {
            ctx.scopes.push(HashMap::new());
        }
        for s in stmts {
            self.stmt(s, ctx);
        }
        if !same_scope {
            ctx.scopes.pop();
        }
    }

    fn block(&mut self, stmts: &[Stmt], ctx: &mut Ctx) {
        ctx.scopes.push(HashMap::new());
        for s in stmts {
            self.stmt(s, ctx);
        }
        ctx.scopes.pop();
    }

    fn lookup(&self, ctx: &Ctx, name: &str) -> Option<Local> {
        ctx.scopes.iter().rev().find_map(|s| s.get(name).cloned())
    }

    fn declare(&mut self, ctx: &mut Ctx, name: &str, ty: Ty, pos: Pos, readonly: bool) {
        if let Some(prev) = self.lookup(ctx, name) {
            self.err(
                pos,
                format!(
                    "{name} is already declared on line {}. Pick another name, or leave out the type to change it: {name} = …;",
                    prev.pos.line
                ),
            );
        } else if let Some(w) = self.taken_by(name) {
            self.err(pos, format!("{w}, so a variable can't use that name."));
        } else if let Some(c) = ctx.class {
            if self.info.field(c, name).is_some() {
                let cn = self.info.classes[c].name.clone();
                self.err(
                    pos,
                    format!("{cn} has a field called {name}, so a variable here can't use that name."),
                );
            }
        }
        ctx.scopes.last_mut().unwrap().insert(name.to_string(), Local { ty, readonly, pos });
    }

    fn taken_by(&self, name: &str) -> Option<String> {
        if self.info.methods.contains_key(name) {
            return Some(format!("there's already a method called {name}"));
        }
        None
    }

    fn stmt(&mut self, s: &Stmt, ctx: &mut Ctx) {
        match s {
            Stmt::Var { ty, name, pos, value } => {
                let want = self.resolve(ty);
                let got = self.expr(value, ctx, Some(&want));
                self.expect_fits(value, &got, &want, &format!("{name} is {}", self.a(&want)));
                self.info.decl_types.insert(value.id, want.clone());
                self.declare(ctx, name, want, *pos, false);
            }
            Stmt::Assign { target, value, pos } => {
                let want = self.expr(target, ctx, None);
                if ctx.in_hook {
                    if let ExprKind::Name(n) = &target.kind {
                        let outer = ctx
                            .scopes
                            .iter()
                            .position(|s| s.contains_key(n))
                            .map(|i| i < ctx.hook_base)
                            .unwrap_or(false);
                        if outer {
                            self.err(*pos, format!("a hook can read the variables around it, but not change them. To change {n} from a hook, keep it in an object's field."));
                        }
                    }
                }
                self.writable(target, ctx, *pos);
                let got = self.expr(value, ctx, Some(&want));
                self.expect_fits(value, &got, &want, &format!("{} is {}", describe(target), self.a(&want)));
            }
            Stmt::SetState { target, option, pos } => {
                let t = self.expr(target, ctx, None);
                match t {
                    Ty::State(s) => {
                        self.writable(target, ctx, *pos);
                        let o = self.expr(option, ctx, Some(&Ty::State(s)));
                        let is_option = self.info.options.contains_key(&option.id);
                        if o != Ty::Error && (o != Ty::State(s) || !is_option) {
                            let sn = self.info.states[s].name.clone();
                            let first = self.info.states[s].options[0].clone();
                            self.err(
                                option.pos,
                                format!(
                                    "{} is a {sn}, so it's set to one of {sn}'s options: {} is {sn}.{first};",
                                    describe(target),
                                    describe(target)
                                ),
                            );
                        }
                    }
                    Ty::StateList(s) => {
                        let sn = self.info.states[s].name.clone();
                        self.err(*pos, format!("{sn} is the list of options itself, so there's nothing to set here. Something has to be in it: a class with state is {sn}.…; in its has block, or a variable like {sn} now = {sn}.…;"));
                    }
                    Ty::Error => {}
                    other => {
                        self.err(
                            *pos,
                            format!(
                                "is sets a state to one of its options, but {} is {}. To change a value, use =: {} = …;",
                                describe(target),
                                self.a(&other),
                                describe(target)
                            ),
                        );
                    }
                }
            }
            Stmt::Expr(e) => {
                let t = self.expr(e, ctx, None);
                let is_lua_call = matches!(&e.kind, ExprKind::Method(..)) && t == Ty::Error;
                if t != Ty::Void && t != Ty::Error && !is_lua_call {
                    let what = describe(e);
                    self.err(
                        e.pos,
                        format!(
                            "{what} gives back {}, but nothing uses it. Put it in a variable: {} x = {what};",
                            self.a(&t),
                            self.show(&t)
                        ),
                    );
                }
            }
            Stmt::If { branches, otherwise, .. } => {
                for (cond, body) in branches {
                    let t = self.expr(cond, ctx, Some(&Ty::Truth));
                    self.expect_truth(cond, &t, "a question");
                    self.block(body, ctx);
                }
                if let Some(o) = otherwise {
                    self.block(o, ctx);
                }
            }
            Stmt::Loop { kind, body, .. } => {
                ctx.scopes.push(HashMap::new());
                match kind {
                    LoopKind::Forever => {}
                    LoopKind::Times(n) => {
                        let t = self.expr(n, ctx, Some(&Ty::Int));
                        if t != Ty::Int && t != Ty::Error {
                            self.err(n.pos, format!("loop needs a whole number of times, but this is {}.", self.a(&t)));
                        }
                    }
                    LoopKind::While(c) => {
                        let t = self.expr(c, ctx, Some(&Ty::Truth));
                        self.expect_truth(c, &t, "the loop's check");
                    }
                    LoopKind::Each { vars, source } => self.each(vars, source, ctx),
                }
                for s in body {
                    self.stmt(s, ctx);
                }
                ctx.scopes.pop();
            }
            Stmt::Stop(p) | Stmt::Skip(p) => {
                if ctx.in_try {
                    self.err(
                        *p,
                        "stop and skip can't be used inside try yet. Move the loop inside the try instead.",
                    );
                }
            }
            Stmt::Return(v, pos) => {
                if ctx.in_try {
                    self.err(
                        *pos,
                        "return can't be used inside try yet. Set a variable, and return after the try.",
                    );
                }
                match (&ctx.ret, v) {
                    (None, _) => self.err(*pos, "return only works inside a method."),
                    (Some(Ty::Void), Some(e)) => {
                        self.expr(e, ctx, None);
                        self.err(e.pos, "this method gives nothing back, so return takes no value. Give the method a type to return one: method int name() { }");
                    }
                    (Some(Ty::Void), None) => {}
                    (Some(r), None) => {
                        let r = r.clone();
                        self.err(
                            *pos,
                            format!(
                                "this method gives back {}: return a value, like return {};",
                                self.a(&r),
                                self.example(&r)
                            ),
                        );
                    }
                    (Some(r), Some(e)) => {
                        let r = r.clone();
                        let got = self.expr(e, ctx, Some(&r));
                        self.expect_fits(e, &got, &r, &format!("this method gives back {}", self.a(&r)));
                    }
                }
            }
            Stmt::Try { body, err, handler, .. } => {
                let was = ctx.in_try;
                ctx.in_try = true;
                self.block(body, ctx);
                ctx.in_try = was;
                ctx.scopes.push(HashMap::new());
                self.declare(ctx, &err.0, Ty::Str, err.1, false);
                for s in handler {
                    self.stmt(s, ctx);
                }
                ctx.scopes.pop();
            }
            Stmt::Lua { .. } => {}
        }
    }

    fn each(&mut self, vars: &[(TypeRef, String, Pos)], source: &Expr, ctx: &mut Ctx) {
        let src = self.expr(source, ctx, None);
        let declared: Vec<Ty> = vars.iter().map(|(t, _, _)| self.resolve(t)).collect();
        let wanted: Option<Vec<Ty>> = match &src {
            Ty::Array(e) => Some(vec![(**e).clone()]),
            Ty::Map(v) => Some(vec![Ty::Str, (**v).clone()]),
            Ty::CArray(cols) => Some(std::iter::once(Ty::Int).chain(cols.iter().map(|(_, t)| t.clone())).collect()),
            Ty::StateList(s) => Some(vec![Ty::State(*s)]),
            Ty::Error => None,
            other => {
                self.err(
                    source.pos,
                    format!(
                        "a loop can go through an array, a map, a cArray or a state's options, but this is {}.",
                        self.a(other)
                    ),
                );
                None
            }
        };
        if let Some(w) = wanted {
            if w.len() != vars.len() {
                let example = match &src {
                    Ty::Map(_) => "loop (string key, int value in prices) { }".to_string(),
                    Ty::CArray(cols) => {
                        let c: Vec<String> = cols.iter().map(|(n, t)| format!("{} {n}", self.show(t))).collect();
                        format!("loop (int id, {} in {}) {{ }}", c.join(", "), describe(source))
                    }
                    _ => format!("loop ({} item in {}) {{ }}", self.show(&w[0]), describe(source)),
                };
                self.err(
                    vars[0].2,
                    format!(
                        "this loop needs {} name{} here: {example}",
                        w.len(),
                        if w.len() == 1 { "" } else { "s" }
                    ),
                );
            } else {
                for ((d, (_, name, pos)), want) in declared.iter().zip(vars).zip(&w) {
                    if d != want && *d != Ty::Error && *want != Ty::Error {
                        self.err(
                            *pos,
                            format!("{name} gets {} here, so it's declared as {}.", self.a(want), self.show(want)),
                        );
                    }
                }
            }
        }
        for ((d, (_, name, pos)), _) in declared.into_iter().zip(vars).zip(0..) {
            self.declare(ctx, name, d, *pos, true);
        }
    }

    fn writable(&mut self, target: &Expr, ctx: &Ctx, pos: Pos) {
        match &target.kind {
            ExprKind::Name(n) => match self.info.names.get(&target.id) {
                Some(NameRes::Local) => {
                    if self.lookup(ctx, n).map(|l| l.readonly).unwrap_or(false) {
                        self.err(pos, format!("{n} is the loop's current item, so it can't be changed here."));
                    }
                }
                Some(NameRes::Field) | None => {}
                Some(NameRes::Object) => self.err(
                    pos,
                    format!("{n} is an object; it can't be replaced. Change its fields instead: {n}.something = …;"),
                ),
                Some(_) => self.err(pos, format!("{n} can't be changed.")),
            },
            ExprKind::Field(t, f, fp) => {
                if matches!(self.info.ty(t), Ty::Row(_)) && f == "id" {
                    self.err(*fp, "a row's id can't be changed.");
                }
                if self.info.options.contains_key(&target.id) {
                    self.err(pos, "a state's option can't be changed.");
                }
            }
            ExprKind::Index(t, _) => {
                if matches!(self.info.ty(t), Ty::CArray(_)) {
                    self.err(pos, "a whole row can't be replaced. Change its columns instead: bag[1].amount = 5;");
                }
            }
            _ => {}
        }
    }

    fn expect_truth(&mut self, e: &Expr, t: &Ty, what: &str) {
        if *t != Ty::Truth && *t != Ty::Error {
            let hint = match t {
                Ty::Int | Ty::Float => format!(" Compare it, like {} > 0", describe(e)),
                Ty::Str => format!(" Compare it, like {} == \"Ada\"", describe(e)),
                Ty::Class(_) => format!(" Ask whether it's there: {} !== nothing", describe(e)),
                _ => String::new(),
            };
            self.err(e.pos, format!("{what} needs a yes-or-no answer, but this is {}.{hint}", self.a(t)));
        }
    }

    fn fits(&self, got: &Ty, want: &Ty) -> bool {
        if got == want || *got == Ty::Error || *want == Ty::Error {
            return true;
        }
        match (got, want) {
            (Ty::Int, Ty::Float) => true,
            (Ty::Nothing, w) => w.can_be_nothing(),
            (Ty::Class(a), Ty::Class(b)) => self.info.is_like(*a, *b),
            (Ty::Hook(a), Ty::Hook(b)) => {
                a.ret == b.ret && a.params.len() == b.params.len() && a.params.iter().zip(&b.params).all(|(x, y)| x.0 == y.0)
            }
            (Ty::Array(a), Ty::Array(b)) | (Ty::Map(a), Ty::Map(b)) => a == b,
            (Ty::CArray(a), Ty::CArray(b)) => a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| x.1 == y.1),
            _ => false,
        }
    }

    fn expect_fits(&mut self, e: &Expr, got: &Ty, want: &Ty, subject: &str) {
        if self.fits(got, want) {
            return;
        }
        if *got == Ty::Void {
            self.err(e.pos, format!("{subject}, but {} doesn't give anything back.", describe(e)));
            return;
        }
        let fix = match (got, want) {
            (Ty::Float, Ty::Int) => " Use .int() to drop the part after the point.".to_string(),
            (g, Ty::Str) if g.is_number() || *g == Ty::Truth || matches!(g, Ty::State(_)) => " Use .text() to turn it into text.".into(),
            (Ty::Str, w) if w.is_number() => format!(
                " Use .{}() to turn text into a number.",
                if *w == Ty::Int { "int" } else { "float" }
            ),
            (Ty::Nothing, _) => " Only things made from classes, and hooks, can be nothing.".into(),
            (Ty::Hook(_), Ty::Hook(_)) => " A method only fits where it takes and gives back exactly the same types.".into(),
            _ => String::new(),
        };
        self.err(e.pos, format!("{subject}, but this is {}.{fix}", self.a(got)));
    }

    // ----- expressions -----

    fn expr(&mut self, e: &Expr, ctx: &mut Ctx, expected: Option<&Ty>) -> Ty {
        let t = self.expr_inner(e, ctx, expected);
        self.info.types.insert(e.id, t.clone());
        t
    }

    fn expr_inner(&mut self, e: &Expr, ctx: &mut Ctx, expected: Option<&Ty>) -> Ty {
        match &e.kind {
            ExprKind::Int(_) => Ty::Int,
            ExprKind::Float(_) => Ty::Float,
            ExprKind::Str(_) => Ty::Str,
            ExprKind::Bool(_) => Ty::Truth,
            ExprKind::Nothing => match expected {
                Some(t) if t.can_be_nothing() => t.clone(),
                _ => Ty::Nothing,
            },
            ExprKind::Name(n) => self.name(e, n, ctx),
            ExprKind::Field(target, f, fpos) => self.field(e, target, f, *fpos, ctx),
            ExprKind::Method(target, m, args, mpos) => self.method_call(e, target, m, args, *mpos, ctx),
            ExprKind::MethodRef(target, m, mpos) => {
                let t = self.expr(target, ctx, None);
                match t {
                    Ty::Class(c) => match self.info.method(c, m).cloned() {
                        Some(sig) => Ty::Hook(Rc::new(HookSig {
                            ret: sig.ret,
                            params: sig.params.into_iter().map(|(n, t)| (t, Some(n))).collect(),
                        })),
                        None => {
                            let cn = self.info.classes[c].name.clone();
                            self.err(*mpos, format!("{cn} has no method called {m}."));
                            Ty::Error
                        }
                    },
                    Ty::Error => Ty::Error,
                    other => {
                        self.err(
                            *mpos,
                            format!("{} is {}, which has no method called {m}.", describe(target), self.a(&other)),
                        );
                        Ty::Error
                    }
                }
            }
            ExprKind::Call(name, args) => self.call(e, name, args, ctx),
            ExprKind::Index(target, index) => {
                let t = self.expr(target, ctx, None);
                match t {
                    Ty::Array(el) => {
                        let i = self.expr(index, ctx, Some(&Ty::Int));
                        if i != Ty::Int && i != Ty::Error {
                            self.err(
                                index.pos,
                                format!("an array's items are numbered: {}[0]. This is {}.", describe(target), self.a(&i)),
                            );
                        }
                        *el
                    }
                    Ty::Map(v) => {
                        let k = self.expr(index, ctx, Some(&Ty::Str));
                        if k != Ty::Str && k != Ty::Error {
                            self.err(
                                index.pos,
                                format!("a map's keys are text: {}[\"apple\"]. This is {}.", describe(target), self.a(&k)),
                            );
                        }
                        *v
                    }
                    Ty::CArray(cols) => {
                        let i = self.expr(index, ctx, Some(&Ty::Int));
                        if i != Ty::Int && i != Ty::Error {
                            self.err(
                                index.pos,
                                format!(
                                    "a cArray's rows are found by their id, a whole number: {}[1]. This is {}.",
                                    describe(target),
                                    self.a(&i)
                                ),
                            );
                        }
                        Ty::Row(cols)
                    }
                    Ty::Str => {
                        self.expr(index, ctx, None);
                        self.err(e.pos, "text can't be picked apart with [ ] yet.");
                        Ty::Error
                    }
                    Ty::Error => {
                        self.expr(index, ctx, None);
                        Ty::Error
                    }
                    other => {
                        self.expr(index, ctx, None);
                        self.err(
                            e.pos,
                            format!("{} is {}, so it has no items to pick with [ ].", describe(target), self.a(&other)),
                        );
                        Ty::Error
                    }
                }
            }
            ExprKind::Len(inner) => {
                let t = self.expr(inner, ctx, None);
                match t {
                    Ty::Str | Ty::Array(_) | Ty::Map(_) | Ty::CArray(_) | Ty::Error => Ty::Int,
                    other => {
                        self.err(
                            e.pos,
                            format!("# counts text, arrays, maps and cArrays, but this is {}.", self.a(&other)),
                        );
                        Ty::Int
                    }
                }
            }
            ExprKind::Neg(inner) => {
                let t = self.expr(inner, ctx, expected);
                if !t.is_number() && t != Ty::Error {
                    self.err(e.pos, format!("only numbers can be made negative, but this is {}.", self.a(&t)));
                    return Ty::Error;
                }
                t
            }
            ExprKind::Not(inner) => {
                let t = self.expr(inner, ctx, Some(&Ty::Truth));
                self.expect_truth(inner, &t, "not");
                Ty::Truth
            }
            ExprKind::Bin(op, l, r, opos) => self.binary(*op, l, r, *opos, ctx),
            ExprKind::Either(c, a, b) => {
                let ct = self.expr(c, ctx, Some(&Ty::Truth));
                self.expect_truth(c, &ct, "either");
                let at = self.expr(a, ctx, expected);
                let bt = self.expr(b, ctx, expected.or(Some(&at)));
                if self.fits(&bt, &at) {
                    if at == Ty::Nothing {
                        bt
                    } else {
                        at
                    }
                } else if self.fits(&at, &bt) {
                    bt
                } else {
                    self.err(
                        b.pos,
                        format!(
                            "both answers of either must be the same type, but these are {} and {}.",
                            self.a(&at),
                            self.a(&bt)
                        ),
                    );
                    Ty::Error
                }
            }
            ExprKind::List { items, keys } => self.list(e, items, keys.as_deref(), ctx, expected),
            ExprKind::New { class, class_pos, inits } => {
                let Some(&c) = self.info.class_index.get(class) else {
                    for (_, _, v) in inits {
                        self.expr(v, ctx, None);
                    }
                    let s = suggest(class, self.class_names.clone());
                    self.err(*class_pos, format!("there's no class called {class}.{s}"));
                    return Ty::Error;
                };
                if self.info.classes[c].is_object {
                    self.err(
                        *class_pos,
                        format!("{class} is an object: there's only ever one, and it's ready to use as {class}."),
                    );
                }
                for (f, fpos, v) in inits {
                    match self.info.field(c, f).map(|x| x.ty.clone()) {
                        Some(ft) => {
                            let got = self.expr(v, ctx, Some(&ft));
                            self.expect_fits(v, &got, &ft, &format!("{f} is {}", self.a(&ft)));
                        }
                        None => {
                            self.expr(v, ctx, None);
                            let fields: Vec<String> = self.all_fields(c);
                            let s = suggest(f, fields);
                            self.err(*fpos, format!("{class} has no field called {f}.{s}"));
                        }
                    }
                }
                Ty::Class(c)
            }
            ExprKind::NewHook(body) => {
                let Some(Ty::Hook(sig)) = expected.cloned() else {
                    self.err(
                        e.pos,
                        "Lust can't tell what this hook takes. Put it where a hook is expected: hook int(int n) triple = new hook { … };",
                    );
                    return Ty::Error;
                };
                let mut names = Vec::new();
                for (i, (_, n)) in sig.params.iter().enumerate() {
                    match n {
                        Some(n) => names.push(n.clone()),
                        None => {
                            self.err(
                                e.pos,
                                "name what the hook takes, where its type is written, so the body can use it: hook(string item)",
                            );
                            names.push(format!("value{}", i + 1));
                        }
                    }
                }
                self.info.hook_params.insert(e.id, names.clone());
                let mut inner = ctx.clone();
                inner.ret = Some(sig.ret.clone());
                inner.in_try = false;
                inner.in_hook = true;
                inner.scopes.push(HashMap::new());
                inner.hook_base = inner.scopes.len() - 1;
                for (n, (t, _)) in names.iter().zip(&sig.params) {
                    if self.lookup(ctx, n).is_some() {
                        self.err(
                            e.pos,
                            format!("{n} is already a variable here, so the hook's {n} would hide it. Rename one of them."),
                        );
                    }
                    inner.scopes.last_mut().unwrap().insert(
                        n.clone(),
                        Local {
                            ty: t.clone(),
                            readonly: false,
                            pos: e.pos,
                        },
                    );
                }
                self.block(body, &mut inner);
                if sig.ret != Ty::Void && !returns(body) {
                    self.err(e.pos, format!("this hook must return {} on every path.", self.a(&sig.ret)));
                }
                Ty::Hook(sig)
            }
            ExprKind::Like(inner, class, cpos) => {
                let t = self.expr(inner, ctx, None);
                let Some(&c) = self.info.class_index.get(class) else {
                    let s = suggest(class, self.class_names.clone());
                    self.err(*cpos, format!("there's no class called {class}.{s}"));
                    return Ty::Truth;
                };
                if !matches!(t, Ty::Class(_) | Ty::Error | Ty::Nothing) {
                    self.err(
                        e.pos,
                        format!("like asks what kind of thing something is, but this is {}.", self.a(&t)),
                    );
                }
                let _ = c;
                Ty::Truth
            }
            ExprKind::Convert(inner, conv, cpos) => {
                let t = self.expr(inner, ctx, None);
                if t == Ty::Error {
                    return match conv {
                        Conv::Text => Ty::Str,
                        Conv::Int => Ty::Int,
                        Conv::Float => Ty::Float,
                        Conv::IsNumber => Ty::Truth,
                        Conv::Lines => Ty::Array(Box::new(Ty::Str)),
                    };
                }
                match conv {
                    Conv::Text => {
                        if !matches!(t, Ty::Int | Ty::Float | Ty::Truth | Ty::Str | Ty::State(_)) {
                            self.err(
                                *cpos,
                                format!(
                                    ".text() works on numbers, truths, text and state options, but this is {}.",
                                    self.a(&t)
                                ),
                            );
                        }
                        Ty::Str
                    }
                    Conv::Int => {
                        match t {
                            Ty::Float | Ty::Str => {}
                            Ty::Int => self.err(*cpos, "this is already an int."),
                            _ => self.err(*cpos, format!(".int() works on floats and text, but this is {}.", self.a(&t))),
                        }
                        Ty::Int
                    }
                    Conv::Float => {
                        match t {
                            Ty::Int | Ty::Str => {}
                            Ty::Float => self.err(*cpos, "this is already a float."),
                            _ => self.err(*cpos, format!(".float() works on ints and text, but this is {}.", self.a(&t))),
                        }
                        Ty::Float
                    }
                    Conv::IsNumber | Conv::Lines => {
                        if t != Ty::Str {
                            let n = if *conv == Conv::IsNumber { "isNumber" } else { "lines" };
                            self.err(*cpos, format!(".{n}() works on text, but this is {}.", self.a(&t)));
                        }
                        if *conv == Conv::IsNumber {
                            Ty::Truth
                        } else {
                            Ty::Array(Box::new(Ty::Str))
                        }
                    }
                }
            }
        }
    }

    fn all_fields(&self, c: usize) -> Vec<String> {
        let mut out = Vec::new();
        let mut cur = Some(c);
        while let Some(i) = cur {
            out.extend(self.info.classes[i].fields.iter().map(|f| f.name.clone()));
            cur = self.info.classes[i].parent;
        }
        out
    }

    fn name(&mut self, e: &Expr, n: &str, ctx: &mut Ctx) -> Ty {
        if let Some(l) = self.lookup(ctx, n) {
            self.info.names.insert(e.id, NameRes::Local);
            return l.ty;
        }
        if let Some(c) = ctx.class {
            if let Some(f) = self.info.field(c, n) {
                let ty = f.ty.clone();
                self.info.names.insert(e.id, NameRes::Field);
                return ty;
            }
        }
        if let Some(&c) = self.info.class_index.get(n) {
            if self.info.classes[c].is_object {
                self.info.names.insert(e.id, NameRes::Object);
                return Ty::Class(c);
            }
            self.err(
                e.pos,
                format!("{n} is a class. Make one with new {n}, or ask about one with is x like {n}?"),
            );
            return Ty::Error;
        }
        if let Some(&s) = self.info.state_index.get(n) {
            self.info.names.insert(e.id, NameRes::StateList);
            self.info.options.insert(e.id, (s, String::new()));
            return Ty::StateList(s);
        }
        if let Some(sig) = self.info.methods.get(n).cloned() {
            self.info.names.insert(e.id, NameRes::TopMethod);
            return Ty::Hook(Rc::new(HookSig {
                ret: sig.ret,
                params: sig.params.into_iter().map(|(n, t)| (t, Some(n))).collect(),
            }));
        }
        if let Some(c) = ctx.class {
            if let Some(sig) = self.info.method(c, n).cloned() {
                self.info.names.insert(e.id, NameRes::OwnMethod);
                return Ty::Hook(Rc::new(HookSig {
                    ret: sig.ret,
                    params: sig.params.into_iter().map(|(n, t)| (t, Some(n))).collect(),
                }));
            }
        }
        if ctx.ret.is_some() && !ctx.in_hook && self.top_vars.contains_key(n) {
            self.err(
                e.pos,
                format!("{n} belongs to the top of the file, and methods can't see those. Pass it in, or keep it in an object."),
            );
            return Ty::Error;
        }
        let mut known: Vec<String> = ctx.scopes.iter().flat_map(|s| s.keys().cloned()).collect();
        if let Some(c) = ctx.class {
            known.extend(self.all_fields(c));
        }
        let s = suggest(n, known);
        if s.is_empty() {
            self.err(
                e.pos,
                format!("there's nothing called {n} here. Declare it first, like int {n} = 0;"),
            );
        } else {
            self.err(e.pos, format!("there's nothing called {n} here.{s}"));
        }
        Ty::Error
    }

    fn field(&mut self, e: &Expr, target: &Expr, f: &str, fpos: Pos, ctx: &mut Ctx) -> Ty {
        let t = self.expr(target, ctx, None);
        // GameState.ActiveGame.Combat: walking down a state's options.
        if let Some((s, path)) = self.info.options.get(&target.id).cloned() {
            let full = if path.is_empty() { f.to_string() } else { format!("{path}.{f}") };
            let st = &self.info.states[s];
            if st.options.contains(&full) {
                self.info.options.insert(e.id, (s, full));
                return Ty::State(s);
            }
            let sn = st.name.clone();
            let here: Vec<String> = st
                .options
                .iter()
                .filter(|o| {
                    let parent = o.rsplit_once('.').map(|(p, _)| p).unwrap_or("");
                    parent == path
                })
                .map(|o| o.rsplit('.').next().unwrap().to_string())
                .collect();
            let where_ = if path.is_empty() { sn.clone() } else { format!("{sn}.{path}") };
            let s2 = suggest(f, here.clone());
            self.err(
                fpos,
                format!("{f} isn't an option of {where_}.{s2} The options are {}.", here.join(", ")),
            );
            return Ty::Error;
        }
        match t {
            Ty::Class(c) => match self.info.field(c, f) {
                Some(fi) => fi.ty.clone(),
                None => {
                    let cn = self.info.classes[c].name.clone();
                    if self.info.method(c, f).is_some() {
                        self.err(
                            fpos,
                            format!(
                                "{f} is a method of {cn}. Methods are called with : and ( ): {}:{f}()",
                                describe(target)
                            ),
                        );
                    } else {
                        let s = suggest(f, self.all_fields(c));
                        self.err(fpos, format!("{cn} has no field called {f}.{s}"));
                    }
                    Ty::Error
                }
            },
            Ty::Row(cols) => {
                if f == "id" {
                    return Ty::Int;
                }
                match cols.iter().find(|(n, _)| n == f) {
                    Some((_, t)) => t.clone(),
                    None => {
                        let names: Vec<String> = cols.iter().map(|(n, _)| n.clone()).collect();
                        let s = suggest(f, names.clone());
                        self.err(
                            fpos,
                            format!("this cArray has no column called {f}.{s} Its columns are id, {}.", names.join(", ")),
                        );
                        Ty::Error
                    }
                }
            }
            Ty::Error => Ty::Error,
            Ty::Nothing => {
                self.err(e.pos, "nothing has no fields.");
                Ty::Error
            }
            other => {
                if ["text", "int", "float", "isNumber", "lines"].contains(&f) {
                    self.err(fpos, format!("add () to turn it: {}.{f}()", describe(target)));
                } else {
                    self.err(fpos, format!("{} is {}, which has no fields.", describe(target), self.a(&other)));
                }
                Ty::Error
            }
        }
    }

    fn args(&mut self, args: &[Expr], params: &[(String, Ty)], what: &str, pos: Pos, ctx: &mut Ctx) {
        if args.len() != params.len() {
            let list: Vec<String> = params.iter().map(|(n, t)| format!("{} {n}", self.show(t))).collect();
            let takes = if params.is_empty() {
                "nothing".to_string()
            } else {
                list.join(", ")
            };
            self.err(
                pos,
                format!("{what} takes {}, but gets {}. It takes: {takes}.", params.len(), args.len()),
            );
            for a in args {
                self.expr(a, ctx, None);
            }
            return;
        }
        for (a, (n, t)) in args.iter().zip(params) {
            let got = self.expr(a, ctx, Some(t));
            self.expect_fits(a, &got, t, &format!("{n} is {}", self.a(t)));
        }
    }

    fn hook_params(sig: &HookSig) -> Vec<(String, Ty)> {
        sig.params
            .iter()
            .enumerate()
            .map(|(i, (t, n))| (n.clone().unwrap_or_else(|| format!("value {}", i + 1)), t.clone()))
            .collect()
    }

    fn call(&mut self, e: &Expr, name: &str, args: &[Expr], ctx: &mut Ctx) -> Ty {
        if BUILTINS.contains(&name) {
            self.info.calls.insert(e.id, CallRes::Builtin);
            let (params, ret): (Vec<(&str, Option<Ty>)>, Ty) = match name {
                "print" => (vec![("value", None)], Ty::Void),
                "random" => (vec![("low", Some(Ty::Int)), ("high", Some(Ty::Int))], Ty::Int),
                "ask" => (vec![("question", Some(Ty::Str))], Ty::Str),
                "readText" => (vec![("file", Some(Ty::Str))], Ty::Str),
                "writeText" => (vec![("file", Some(Ty::Str)), ("text", Some(Ty::Str))], Ty::Void),
                "wait" => (vec![("seconds", Some(Ty::Float))], Ty::Void),
                _ => unreachable!(),
            };
            if args.len() != params.len() {
                self.err(
                    e.pos,
                    format!(
                        "{name} takes {} value{}, but gets {}.",
                        params.len(),
                        if params.len() == 1 { "" } else { "s" },
                        args.len()
                    ),
                );
                for a in args {
                    self.expr(a, ctx, None);
                }
                return ret;
            }
            for (a, (pn, pt)) in args.iter().zip(&params) {
                match pt {
                    Some(t) => {
                        let got = self.expr(a, ctx, Some(t));
                        self.expect_fits(a, &got, t, &format!("{name}'s {pn} is {}", self.a(t)));
                    }
                    None => {
                        let got = self.expr(a, ctx, None);
                        if got == Ty::Void {
                            self.err(
                                a.pos,
                                format!("{} doesn't give anything back, so there's nothing to print.", describe(a)),
                            );
                        }
                    }
                }
            }
            return ret;
        }
        // A hook kept in a variable.
        if let Some(l) = self.lookup(ctx, name) {
            if let Ty::Hook(sig) = &l.ty {
                self.info.calls.insert(e.id, CallRes::LocalHook);
                self.info.decl_types.insert(e.id, l.ty.clone());
                let params = Self::hook_params(sig);
                self.args(args, &params, name, e.pos, ctx);
                return sig.ret.clone();
            }
            if l.ty != Ty::Error {
                self.err(e.pos, format!("{name} is {}, so it can't be called.", self.a(&l.ty)));
            }
            return Ty::Error;
        }
        if let Some(c) = ctx.class {
            if let Some(sig) = self.info.method(c, name).cloned() {
                self.info.calls.insert(e.id, CallRes::Own);
                self.args(args, &sig.params, name, e.pos, ctx);
                return sig.ret;
            }
            if let Some(f) = self.info.field(c, name) {
                if let Ty::Hook(sig) = f.ty.clone() {
                    self.info.calls.insert(e.id, CallRes::FieldHook);
                    let params = Self::hook_params(&sig);
                    self.args(args, &params, name, e.pos, ctx);
                    return sig.ret.clone();
                }
            }
        }
        if let Some(sig) = self.info.methods.get(name).cloned() {
            self.info.calls.insert(e.id, CallRes::Top);
            self.args(args, &sig.params, name, e.pos, ctx);
            return sig.ret;
        }
        for a in args {
            self.expr(a, ctx, None);
        }
        let mut known: Vec<String> = self.info.methods.keys().cloned().collect();
        if let Some(c) = ctx.class {
            let mut cur = Some(c);
            while let Some(i) = cur {
                known.extend(self.info.classes[i].methods.keys().cloned());
                cur = self.info.classes[i].parent;
            }
        }
        let s = suggest(name, known);
        self.err(e.pos, format!("there's no method called {name}.{s}"));
        Ty::Error
    }

    fn method_call(&mut self, e: &Expr, target: &Expr, m: &str, args: &[Expr], mpos: Pos, ctx: &mut Ctx) -> Ty {
        let t = self.expr(target, ctx, None);
        let builtin: Option<(Vec<(String, Ty)>, Ty)> = match (&t, m) {
            (Ty::Array(el), "inject") => Some((vec![("item".into(), (**el).clone())], Ty::Void)),
            (Ty::Array(_), "remove") => Some((vec![("position".into(), Ty::Int)], Ty::Void)),
            (Ty::Map(_), "has") => Some((vec![("key".into(), Ty::Str)], Ty::Truth)),
            (Ty::Map(_), "remove") => Some((vec![("key".into(), Ty::Str)], Ty::Void)),
            (Ty::CArray(_), "remove") => Some((vec![("id".into(), Ty::Int)], Ty::Void)),
            (Ty::CArray(_), "has") => Some((vec![("id".into(), Ty::Int)], Ty::Truth)),
            (Ty::CArray(cols), "inject") => {
                if args.len() != 1 {
                    self.err(mpos, "inject takes one row, like bag:inject([\"map\", 1]);");
                    return Ty::Void;
                }
                self.info.calls.insert(e.id, CallRes::Builtin);
                self.row(&args[0], cols, ctx);
                return Ty::Void;
            }
            _ => None,
        };
        if let Some((params, ret)) = builtin {
            self.info.calls.insert(e.id, CallRes::Builtin);
            self.args(args, &params, m, e.pos, ctx);
            return ret;
        }
        match t {
            Ty::Class(c) => {
                if let Some(sig) = self.info.method(c, m).cloned() {
                    self.info.calls.insert(e.id, CallRes::Own);
                    self.args(args, &sig.params, m, mpos, ctx);
                    return sig.ret;
                }
                if let Some(Ty::Hook(sig)) = self.info.field(c, m).map(|f| f.ty.clone()) {
                    self.info.calls.insert(e.id, CallRes::FieldHook);
                    let params = Self::hook_params(&sig);
                    self.args(args, &params, m, mpos, ctx);
                    return sig.ret.clone();
                }
                for a in args {
                    self.expr(a, ctx, None);
                }
                let cn = self.info.classes[c].name.clone();
                let mut known = Vec::new();
                let mut cur = Some(c);
                while let Some(i) = cur {
                    known.extend(self.info.classes[i].methods.keys().cloned());
                    cur = self.info.classes[i].parent;
                }
                let s = suggest(m, known);
                self.err(mpos, format!("{cn} has no method called {m}.{s}"));
                Ty::Error
            }
            Ty::Error => {
                for a in args {
                    self.expr(a, ctx, None);
                }
                Ty::Error
            }
            other => {
                for a in args {
                    self.expr(a, ctx, None);
                }
                let what = match other {
                    Ty::Array(_) => " Arrays have inject and remove.",
                    Ty::Map(_) => " Maps have has and remove.",
                    Ty::CArray(_) => " cArrays have inject, remove and has.",
                    _ => "",
                };
                self.err(
                    mpos,
                    format!("{} is {}, which has no method called {m}.{what}", describe(target), self.a(&other)),
                );
                Ty::Error
            }
        }
    }

    /// One cArray row: its columns' values, optionally with its id first.
    fn row(&mut self, item: &Expr, cols: &Rc<Vec<(String, Ty)>>, ctx: &mut Ctx) {
        let ExprKind::List { items, keys: None } = &item.kind else {
            let t = self.expr(item, ctx, None);
            if t != Ty::Error {
                self.err(item.pos, "a cArray row is written in [ ], one value per column: [\"map\", 1]");
            }
            return;
        };
        self.info.types.insert(item.id, Ty::Row(cols.clone()));
        let with_id = items.len() == cols.len() + 1;
        if items.len() != cols.len() && !with_id {
            let names: Vec<String> = cols.iter().map(|(n, _)| n.clone()).collect();
            self.err(
                item.pos,
                format!(
                    "a row needs one value for each column ({}), or an id first and then those.",
                    names.join(", ")
                ),
            );
            for i in items {
                self.expr(i, ctx, None);
            }
            return;
        }
        let mut wanted: Vec<(String, Ty)> = cols.iter().cloned().collect();
        if with_id {
            wanted.insert(0, ("id".into(), Ty::Int));
        }
        for (v, (n, t)) in items.iter().zip(&wanted) {
            let got = self.expr(v, ctx, Some(t));
            self.expect_fits(v, &got, t, &format!("{n} is {}", self.a(t)));
        }
    }

    fn list(&mut self, e: &Expr, items: &[Expr], keys: Option<&[Expr]>, ctx: &mut Ctx, expected: Option<&Ty>) -> Ty {
        match expected {
            Some(Ty::Array(el)) if keys.is_none() => {
                for i in items {
                    let got = self.expr(i, ctx, Some(el));
                    self.expect_fits(i, &got, el, &format!("every item is {}", self.a(el)));
                }
                return Ty::Array(el.clone());
            }
            Some(Ty::Map(v)) if keys.is_some() || items.is_empty() => {
                for (k, i) in keys.unwrap_or(&[]).iter().zip(items) {
                    let kt = self.expr(k, ctx, Some(&Ty::Str));
                    if kt != Ty::Str && kt != Ty::Error {
                        self.err(k.pos, format!("a map's keys are text, but this is {}.", self.a(&kt)));
                    }
                    let got = self.expr(i, ctx, Some(v));
                    self.expect_fits(i, &got, v, &format!("every value is {}", self.a(v)));
                }
                if let Some(ks) = keys {
                    let mut seen = Vec::new();
                    for k in ks {
                        if let ExprKind::Str(s) = &k.kind {
                            if seen.contains(s) {
                                self.err(k.pos, format!("\"{s}\" is already a key in this map."));
                            }
                            seen.push(s.clone());
                        }
                    }
                }
                return Ty::Map(v.clone());
            }
            Some(Ty::CArray(cols)) if keys.is_none() => {
                for i in items {
                    self.row(i, cols, ctx);
                }
                return Ty::CArray(cols.clone());
            }
            _ => {}
        }
        if let Some(ks) = keys {
            let mut vt = Ty::Error;
            for (k, i) in ks.iter().zip(items) {
                self.expr(k, ctx, Some(&Ty::Str));
                let t = self.expr(i, ctx, None);
                if vt == Ty::Error {
                    vt = t;
                }
            }
            if let Some(exp) = expected {
                if !matches!(exp, Ty::Error) {
                    self.err(e.pos, format!("this is a map, but {} is expected here.", self.a(exp)));
                    return Ty::Error;
                }
            }
            return Ty::Map(Box::new(vt));
        }
        if items.is_empty() {
            if expected.map(|t| *t == Ty::Error).unwrap_or(false) {
                return Ty::Error;
            }
            self.err(
                e.pos,
                "Lust can't tell what this empty list holds. Give it a type where it's declared: int array scores = [];",
            );
            return Ty::Error;
        }
        let mut el = self.expr(&items[0], ctx, None);
        for i in &items[1..] {
            let t = self.expr(i, ctx, Some(&el));
            if el == Ty::Int && t == Ty::Float {
                el = Ty::Float;
            } else if !self.fits(&t, &el) {
                self.err(
                    i.pos,
                    format!(
                        "every item in an array is the same type: this is {}, but the first is {}.",
                        self.a(&t),
                        self.a(&el)
                    ),
                );
            }
        }
        if let Some(exp) = expected {
            if !matches!(exp, Ty::Error | Ty::Array(_)) {
                self.err(e.pos, format!("this is a list, but {} is expected here.", self.a(exp)));
                return Ty::Error;
            }
        }
        Ty::Array(Box::new(el))
    }

    fn binary(&mut self, op: BinOp, l: &Expr, r: &Expr, opos: Pos, ctx: &mut Ctx) -> Ty {
        match op {
            BinOp::And | BinOp::Or => {
                let lt = self.expr(l, ctx, Some(&Ty::Truth));
                let rt = self.expr(r, ctx, Some(&Ty::Truth));
                self.expect_truth(l, &lt, op.symbol());
                self.expect_truth(r, &rt, op.symbol());
                Ty::Truth
            }
            BinOp::Eq | BinOp::Ne | BinOp::Same | BinOp::NotSame => {
                let lt = self.expr(l, ctx, None);
                let rt = self.expr(r, ctx, Some(&lt));
                let ok = self.fits(&lt, &rt) || self.fits(&rt, &lt) || (lt.is_number() && rt.is_number());
                if !ok {
                    let fix = if (lt.is_number() && rt == Ty::Str) || (rt.is_number() && lt == Ty::Str) {
                        " Turn one into the other first, with .text() or .int()."
                    } else {
                        ""
                    };
                    self.err(
                        opos,
                        format!(
                            "{} compares two things of the same type, but these are {} and {}.{fix}",
                            op.symbol(),
                            self.a(&lt),
                            self.a(&rt)
                        ),
                    );
                }
                if let (Ty::StateList(_), _) | (_, Ty::StateList(_)) = (&lt, &rt) {
                    self.err(opos, "compare with one of the state's options, like is mood == Mood.Happy?");
                }
                Ty::Truth
            }
            BinOp::Lt | BinOp::Gt | BinOp::Le | BinOp::Ge => {
                let lt = self.expr(l, ctx, None);
                let rt = self.expr(r, ctx, None);
                for (t, x) in [(&lt, l), (&rt, r)] {
                    if !t.is_number() && *t != Ty::Error {
                        self.err(x.pos, format!("{} compares numbers, but this is {}.", op.symbol(), self.a(t)));
                    }
                }
                Ty::Truth
            }
            BinOp::Add => {
                let lt = self.expr(l, ctx, None);
                let rt = self.expr(r, ctx, None);
                match (&lt, &rt) {
                    (Ty::Str, Ty::Str) => Ty::Str,
                    (Ty::Int, Ty::Int) => Ty::Int,
                    (a, b) if a.is_number() && b.is_number() => Ty::Float,
                    (Ty::Error, _) | (_, Ty::Error) => Ty::Error,
                    (Ty::Str, other) | (other, Ty::Str) => {
                        let which = if lt == Ty::Str { r } else { l };
                        let fix = if other.is_number() || *other == Ty::Truth || matches!(other, Ty::State(_)) {
                            format!(" Use {}.text() to turn it into text.", describe(which))
                        } else {
                            String::new()
                        };
                        self.err(opos, format!("can't add {} to a string.{fix}", self.a(other)));
                        Ty::Error
                    }
                    _ => {
                        self.err(
                            opos,
                            format!("+ works on numbers and text, but these are {} and {}.", self.a(&lt), self.a(&rt)),
                        );
                        Ty::Error
                    }
                }
            }
            BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod => {
                let lt = self.expr(l, ctx, None);
                let rt = self.expr(r, ctx, None);
                if lt == Ty::Error || rt == Ty::Error {
                    return Ty::Error;
                }
                if !lt.is_number() || !rt.is_number() {
                    self.err(
                        opos,
                        format!(
                            "{} works on numbers, but these are {} and {}.",
                            op.symbol(),
                            self.a(&lt),
                            self.a(&rt)
                        ),
                    );
                    return Ty::Error;
                }
                if lt == Ty::Int && rt == Ty::Int {
                    if op == BinOp::Div {
                        if let ExprKind::Int(0) = r.kind {
                            self.err(r.pos, "dividing a whole number by zero doesn't work.");
                        }
                    }
                    Ty::Int
                } else {
                    Ty::Float
                }
            }
        }
    }
}

/// Does this block always reach a return?
pub fn returns(stmts: &[Stmt]) -> bool {
    stmts.iter().any(|s| match s {
        Stmt::Return(..) => true,
        Stmt::If {
            branches,
            otherwise: Some(o),
            ..
        } => branches.iter().all(|(_, b)| returns(b)) && returns(o),
        Stmt::Try { body, handler, .. } => returns(body) && returns(handler),
        Stmt::Loop {
            kind: LoopKind::Forever,
            body,
            ..
        } => !has_stop(body),
        _ => false,
    })
}

fn has_stop(stmts: &[Stmt]) -> bool {
    stmts.iter().any(|s| match s {
        Stmt::Stop(_) => true,
        Stmt::If { branches, otherwise, .. } => {
            branches.iter().any(|(_, b)| has_stop(b)) || otherwise.as_ref().map(|o| has_stop(o)).unwrap_or(false)
        }
        Stmt::Try { body, handler, .. } => has_stop(body) || has_stop(handler),
        _ => false,
    })
}
