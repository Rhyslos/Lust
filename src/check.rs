//! Checks a program before it runs: every name known, every type fitting, every
//! method giving back what it promises. Also records what the Lua writer needs to know.

use std::collections::HashMap;
use std::rc::Rc;

use crate::ast::*;
use crate::diag::Diags;
use crate::parser::BUILTINS;
use crate::types::{HookSig, Names, TableTy, Ty};

pub struct StateInfo {
    pub name: String,
    /// Every option's full path, in the order written: "MainMenu", "ActiveGame", "ActiveGame.Combat".
    pub options: Vec<String>,
}

#[allow(dead_code)]
pub struct RouteInfo {
    pub method: String,
    pub path: String,
    pub handler: String,
    /// The handler's values: each one from the path (by name), or the request's body.
    pub params: Vec<(String, Ty, bool)>,
    pub ret: Ty,
}

#[allow(dead_code)]
pub struct DbInfo {
    pub name: String,
    pub path: String,
    pub tables: Vec<(String, Rc<Vec<(String, Ty)>>)>,
}

#[derive(Clone, Debug)]
pub enum SaveItem {
    /// A variable at the top of the file, and its type.
    Var(Ty),
    /// An object, by class.
    Object(usize),
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
    pub is_node: bool,
    #[allow(dead_code)]
    pub is_schema: bool,
    pub has_view: bool,
    /// A schema's fields with no starting value: they must always be filled in.
    pub required: Vec<String>,
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
    /// Standard library calls, by expression.
    pub libs: HashMap<usize, crate::stdlib::Lib>,
    /// RoomOne:open(), :close() and :isOpen(), by expression.
    pub node_ops: HashMap<usize, &'static str>,
    /// LootSpawner:create(Item), by expression: the assembly and the class.
    pub creates: HashMap<usize, (String, usize)>,
    /// Each assembly's recipes: class and the recipe's `new`.
    pub assemblies: HashMap<String, Vec<(usize, usize)>>,
    pub databases: Vec<DbInfo>,
    pub db_index: HashMap<String, usize>,
    /// Names inside :where( ) and :orderBy( ) that are a table's columns.
    pub columns: HashMap<usize, String>,
    /// Calls on tables and queries: inject, where, orderBy and the rest.
    pub table_ops: HashMap<usize, &'static str>,
    /// Every route the program answers.
    pub routes: Vec<RouteInfo>,
    /// Named styles: style card = [ … ];
    pub styles: std::collections::HashSet<String>,
    /// App:open(), by expression: the assembly whose view opens.
    pub view_opens: HashMap<usize, usize>,
    /// A piece in a loop over a map or cMap: the loop keys that tell its pieces apart.
    pub piece_keys: HashMap<usize, Vec<String>>,
    /// Where native methods are declared: their bodies are outside Lust.
    pub natives: std::collections::HashSet<Pos>,
    /// Saves, by name: what each one holds.
    pub saves: HashMap<String, Vec<(String, SaveItem)>>,
    pub states: Vec<StateInfo>,
    pub classes: Vec<ClassInfo>,
    pub methods: HashMap<String, MethodSig>,
    pub class_index: HashMap<String, usize>,
    pub state_index: HashMap<String, usize>,
}

impl Info {
    /// Does this class have a state of its own (state is …; in its has)? Things its methods
    /// make belong to that state.
    pub fn stateful(&self, c: usize) -> bool {
        self.field(c, "state").is_some()
    }

    /// If this is an object's own state (its `state` field), the object it belongs to:
    /// None for `this`, Some(target) for `player.state`.
    pub fn state_owner<'e>(&self, target: &'e Expr, class: Option<usize>) -> Option<Option<&'e Expr>> {
        match &target.kind {
            ExprKind::Name(n) if n == "state" && self.names.get(&target.id) == Some(&NameRes::Field) => {
                class.filter(|&c| self.stateful(c)).map(|_| None)
            }
            ExprKind::Field(t, f, _) if f == "state" => match self.ty(t) {
                Ty::Class(c) if self.stateful(c) => Some(Some(&**t)),
                _ => None,
            },
            _ => None,
        }
    }

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
    /// Variables holding things this method made while its object is in a state.
    made: std::collections::HashSet<String>,
    /// Inside a view, where pieces of the screen may go.
    in_view: bool,
}

pub struct Checker<'a> {
    pub info: Info,
    diags: &'a mut Diags,
    top_vars: HashMap<String, Pos>,
    class_names: Vec<String>,
    state_names: Vec<String>,
    /// Set just before resolving a type that may carry a key-maker (a variable's or a field's).
    maker_ok: bool,
    /// The keys of the map and cMap loops being checked, innermost last.
    loop_keys: Vec<String>,
    /// How many menus and canvases the piece being checked is inside.
    menus: usize,
    canvases: usize,
}

pub fn check(prog: &Program, diags: &mut Diags, is_script: bool) -> Info {
    let mut c = Checker {
        info: Info::default(),
        diags,
        top_vars: HashMap::new(),
        class_names: vec![],
        state_names: vec![],
        maker_ok: false,
        loop_keys: vec![],
        menus: 0,
        canvases: 0,
    };
    c.run(prog, is_script);
    c.info
}

fn suggest(name: &str, candidates: impl IntoIterator<Item = String>) -> String {
    let lower = name.to_lowercase();
    let mut best: Option<(usize, String)> = None;
    // In a fixed order, so the same mistake always gets the same suggestion.
    let mut candidates: Vec<String> = candidates.into_iter().collect();
    candidates.sort();
    candidates.dedup();
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
                        is_node: c.is_node,
                        is_schema: c.is_schema,
                        has_view: c.view.is_some(),
                        required: vec![],
                        parent: None,
                        fields: vec![],
                        methods: HashMap::new(),
                    });
                }
                Item::Assembly(a) => {
                    if self.name_taken(&a.name, a.pos) {
                        continue;
                    }
                    self.info.assemblies.insert(a.name.clone(), vec![]);
                }
                Item::Route(_) => {}
                Item::Style(st) => {
                    if self.info.styles.contains(&st.name) {
                        self.err(st.pos, format!("there's already a style called {}.", st.name));
                    } else if crate::native_ui::COLOURS.contains(&st.name.as_str()) {
                        self.err(
                            st.pos,
                            format!("{} is one of Lust's colours, so a style can't use that name.", st.name),
                        );
                    }
                    self.info.styles.insert(st.name.clone());
                }
                Item::Database(d) => {
                    if self.name_taken(&d.name, d.pos) {
                        continue;
                    }
                    self.info.db_index.insert(d.name.clone(), self.info.databases.len());
                    self.info.databases.push(DbInfo {
                        name: d.name.clone(),
                        path: d.path.clone(),
                        tables: vec![],
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
                if c.is_schema {
                    if let Some((_, pos)) = &c.parent {
                        self.err(*pos, "a schema can't be like a class. It's only the shape of some data.");
                    }
                    if let Some(m) = c.methods.first() {
                        self.err(
                            m.pos,
                            "a schema only holds fields: it's the shape of some data, so it has no methods.",
                        );
                    }
                }
                if c.is_node {
                    for m in &c.methods {
                        if matches!(m.name.as_str(), "open" | "close" | "isOpen") {
                            self.err(
                                m.pos,
                                format!(
                                    "every node has {} already: Lust opens and closes nodes. Use onOpen and onClose to do things then.",
                                    m.name
                                ),
                            );
                        }
                        if matches!(m.name.as_str(), "onOpen" | "onClose") && (!m.params.is_empty() || m.ret.is_some()) {
                            self.err(
                                m.pos,
                                format!("{} takes nothing and gives nothing back: method {}() {{ … }}", m.name, m.name),
                            );
                        }
                    }
                }
                if let (Some((_, pos)), true) = (&c.parent, c.is_node) {
                    self.err(*pos, "a node can't be like a class. It stands on its own, like an object.");
                    continue;
                }
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
                    Some(t) => {
                        self.maker_ok = true;
                        self.resolve(t)
                    }
                    None => {
                        // state is GameState.Idle;
                        let mut ctx = Ctx {
                            class: None,
                            ret: None,
                            scopes: vec![HashMap::new()],
                            in_try: false,
                            in_hook: false,
                            hook_base: 0,
                            made: Default::default(),
                            in_view: false,
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
        // Database tables: cMaps whose columns hold plain values.
        for item in &prog.items {
            let Item::Database(d) = item else { continue };
            let Some(&di) = self.info.db_index.get(&d.name) else { continue };
            let mut tables = Vec::new();
            for (ty, name, pos) in &d.tables {
                if ty.key_maker().is_some() {
                    self.err(*pos, "a table's keys come from the database, so a table has no key-maker.");
                }
                self.maker_ok = true;
                let t = self.resolve(ty);
                let Ty::CMap(cols) = t else { continue };
                for (cn, ct) in cols.iter() {
                    if !matches!(ct, Ty::Int | Ty::Float | Ty::Str | Ty::Truth | Ty::State(_) | Ty::Error) {
                        self.err(
                            *pos,
                            format!(
                                "a table's columns hold numbers, text, bools or state options, but {cn} is {}.",
                                self.a(ct)
                            ),
                        );
                    }
                }
                if tables.iter().any(|(n, _): &(String, Rc<Vec<(String, Ty)>>)| n == name) {
                    self.err(*pos, format!("{} already has a table called {name}.", d.name));
                    continue;
                }
                tables.push((name.clone(), cols));
            }
            self.info.databases[di].tables = tables;
        }
        // Assembly recipes: each one a `new`, checked like a field's starting value.
        for item in &prog.items {
            let Item::Assembly(a) = item else { continue };
            if !self.info.assemblies.contains_key(&a.name) {
                continue;
            }
            let mut made: Vec<(usize, usize)> = Vec::new();
            for (class, cpos, e) in &a.recipes {
                let mut ctx = Ctx {
                    class: None,
                    ret: None,
                    scopes: vec![HashMap::new()],
                    in_try: false,
                    in_hook: false,
                    hook_base: 0,
                    made: Default::default(),
                    in_view: false,
                };
                if let Ty::Class(c) = self.expr(e, &mut ctx, None) {
                    if made.iter().any(|(x, _)| *x == c) {
                        self.err(
                            *cpos,
                            format!("{} already has a recipe for {class}. An assembly has one recipe per class.", a.name),
                        );
                    } else {
                        made.push((c, e.id));
                    }
                }
            }
            self.info.assemblies.insert(a.name.clone(), made);
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
                            made: Default::default(),
                            in_view: false,
                        };
                        let got = self.expr(d, &mut ctx, Some(&want));
                        self.expect_fits(d, &got, &want, &format!("{} is {}", f.name, self.a(&want)));
                        if let Some(t) = &f.ty {
                            self.key_maker(t, &want, &mut ctx);
                        }
                    }
                    None if c.is_schema => self.info.classes[ci].required.push(f.name.clone()),
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
        // Routes: each answered by a method, whose values come from the path and the body.
        for item in &prog.items {
            let Item::Route(r) = item else { continue };
            for (method, path, handler, pos) in &r.routes {
                let Some(sig) = self.info.methods.get(handler).cloned() else {
                    self.err(*pos, format!("there's no method called {handler} to answer {method} {path}."));
                    continue;
                };
                if self.info.routes.iter().any(|x| x.method == *method && x.path == *path) {
                    self.err(*pos, format!("{method} {path} is answered twice."));
                    continue;
                }
                let names: Vec<String> = path.split('/').filter_map(|s| s.strip_prefix(':')).map(|s| s.to_string()).collect();
                let mut params = Vec::new();
                let mut body = 0;
                for (pn, pt) in &sig.params {
                    let from_path = names.contains(pn);
                    if from_path {
                        if !matches!(pt, Ty::Int | Ty::Float | Ty::Str) {
                            self.err(
                                *pos,
                                format!("{handler}'s {pn} comes from the path, so it's an int, a float or a string."),
                            );
                        }
                    } else {
                        body += 1;
                        if let Some(p) = crate::stdlib::json_problem(pt, &|t| self.a(t)) {
                            self.err(
                                *pos,
                                format!("{handler}'s {pn} isn't in the path, so it comes from the request's body as JSON, but {p}"),
                            );
                        }
                    }
                    params.push((pn.clone(), pt.clone(), from_path));
                }
                if body > 1 {
                    self.err(*pos, format!("{handler} takes more than one value that isn't in the path. Only one comes from the request's body: put them in a schema."));
                }
                for n in &names {
                    if !sig.params.iter().any(|(p, _)| p == n) {
                        self.err(
                            *pos,
                            format!("the path has :{n}, so {handler} needs a value called {n}: method {handler}(int {n}) {{ … }}"),
                        );
                    }
                }
                if sig.ret != Ty::Void {
                    if let Some(p) = crate::stdlib::json_problem(&sig.ret, &|t| self.a(t)) {
                        self.err(*pos, format!("{handler}'s answer goes back as JSON, but {p}"));
                    }
                }
                self.info.routes.push(RouteInfo {
                    method: method.clone(),
                    path: path.clone(),
                    handler: handler.clone(),
                    params,
                    ret: sig.ret.clone(),
                });
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
            made: Default::default(),
            in_view: false,
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
                    if let (Some(v), Some(ci)) = (&c.view, ci) {
                        self.view_body(v, ci);
                    }
                }
                Item::Style(st) => {
                    let mut ctx = Ctx {
                        class: None,
                        ret: None,
                        scopes: vec![HashMap::new()],
                        in_try: false,
                        in_hook: false,
                        hook_base: 0,
                        made: Default::default(),
                        in_view: false,
                    };
                    self.eval_styles(&st.items, &mut ctx);
                    self.check_styles(&st.items, "style");
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
        } else if self.info.assemblies.contains_key(name) {
            Some("an assembly")
        } else if self.info.db_index.contains_key(name) {
            Some("a database")
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
        let maker_ok = std::mem::take(&mut self.maker_ok);
        match t {
            TypeRef::Named(n, pos) => match n.as_str() {
                "int" => Ty::Int,
                "float" => Ty::Float,
                "string" => Ty::Str,
                "bool" => Ty::Truth,
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
                        .chain(["int", "float", "string", "bool"].iter().map(|s| s.to_string()))
                        .collect();
                    let s = suggest(n, known);
                    self.err(*pos, format!("Lust doesn't have a type called {n}.{s} The types are int, float, string, bool, arrays, maps, cMaps, hooks, classes and states."));
                    Ty::Error
                }
            },
            TypeRef::Array(e) => Ty::Array(Box::new(self.resolve(e))),
            TypeRef::Map(e) => Ty::Map(Box::new(self.resolve(e))),
            TypeRef::CMap(cols) => {
                let resolved: Vec<(String, Ty)> = cols.iter().map(|c| (c.name.clone(), self.resolve(&c.ty))).collect();
                if let Some(key) = cols.first() {
                    if !matches!(resolved[0].1, Ty::Int | Ty::Str | Ty::Error) {
                        self.err(
                            key.pos,
                            format!("a cMap's first column is its key, so it has to be an int or a string: cMap(int id, …) or cMap(string name, …). {} is {}.", key.name, self.a(&resolved[0].1)),
                        );
                    }
                    if let (Some(m), false) = (&key.maker, maker_ok) {
                        self.err(
                            m.pos,
                            "a key-maker goes where a cMap is made: cMap(int id = rollId, string item) bag = [];",
                        );
                    }
                } else {
                    self.err(Pos::default(), "a cMap needs at least its key column: cMap(int id, string item)");
                }
                Ty::CMap(Rc::new(resolved))
            }
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
            Ty::Array(_) | Ty::Map(_) | Ty::CMap(_) => "[]".into(),
            Ty::State(s) => format!("{}.{}", self.info.states[*s].name, self.info.states[*s].options[0]),
            _ => "nothing".into(),
        }
    }

    /// A view's body: checked like a method of its assembly, where pieces of the screen may go.
    fn view_body(&mut self, v: &ViewDecl, class: usize) {
        let mut ctx = Ctx {
            class: Some(class),
            ret: None,
            scopes: vec![HashMap::new()],
            in_try: false,
            in_hook: false,
            hook_base: 0,
            made: Default::default(),
            in_view: true,
        };
        self.eval_styles(&v.settings, &mut ctx);
        self.check_styles(&v.settings, "view");
        self.block_in(&v.body, &mut ctx, true);
    }

    /// One piece of a view.
    #[allow(clippy::too_many_arguments)]
    fn element(
        &mut self,
        kind: &str,
        arg: &Option<Expr>,
        style: &[StyleItem],
        children: &[Stmt],
        action: &Option<Vec<Stmt>>,
        enter: &Option<Vec<Stmt>>,
        choices: &Option<Expr>,
        pos: Pos,
        ctx: &mut Ctx,
    ) {
        if !ctx.in_view {
            self.err(pos, format!("{kind} is a piece of a screen, so it goes inside a view {{ }}."));
            return;
        }
        match (kind, arg) {
            ("piece", Some(a)) => {
                if !self.loop_keys.is_empty() {
                    self.info.piece_keys.insert(a.id, self.loop_keys.clone());
                }
                let t = self.expr(a, ctx, None);
                if let (Ty::Class(c), ExprKind::New { class, class_pos, .. }) = (&t, &a.kind) {
                    if !self.info.classes[*c].has_view {
                        self.err(
                            *class_pos,
                            format!("{class} has no view, so it can't be a piece of a screen. Give it one: view {{ … }}"),
                        );
                    }
                }
            }
            ("text" | "button", Some(a)) => {
                let t = self.expr(a, ctx, Some(&Ty::Str));
                if t != Ty::Str && t != Ty::Error {
                    let fix = if matches!(t, Ty::Int | Ty::Float | Ty::Truth) {
                        format!(" Use {}.text() to turn it into text.", describe(a))
                    } else {
                        String::new()
                    };
                    self.err(a.pos, format!("{kind} shows text, but this is {}.{fix}", self.a(&t)));
                }
            }
            ("input" | "textarea" | "checkbox" | "slider" | "dropdown" | "dialog" | "tabs", Some(a)) => {
                let t = self.expr(a, ctx, None);
                let (want, example) = match kind {
                    "input" => ("text", "input name;"),
                    "textarea" => ("text", "textarea notes;"),
                    "checkbox" => ("a bool", "checkbox done;"),
                    "dropdown" => ("text or a state", "dropdown size in [\"Small\", \"Large\"];"),
                    "dialog" => ("a bool", "dialog confirming { … }"),
                    "tabs" => ("text: the name of the tab that shows", "tabs page { tab \"Home\" { … } }"),
                    _ => ("a number", "slider volume [from 0, to 100];"),
                };
                let fits = match kind {
                    "input" | "textarea" | "tabs" => t == Ty::Str,
                    "checkbox" | "dialog" => t == Ty::Truth,
                    "dropdown" => matches!(t, Ty::Str | Ty::State(_)),
                    _ => matches!(t, Ty::Int | Ty::Float),
                };
                if kind == "dropdown" {
                    match (&t, choices) {
                        (Ty::Str, Some(c)) => {
                            let ct = self.expr(c, ctx, None);
                            if !matches!(&ct, Ty::Array(e) if **e == Ty::Str) && ct != Ty::Error {
                                self.err(
                                    c.pos,
                                    format!("a dropdown chooses from a string array, but this is {}.", self.a(&ct)),
                                );
                            }
                        }
                        (Ty::Str, None) => self.err(
                            a.pos,
                            format!(
                                "a dropdown of text needs what to choose from: dropdown {} in [\"Small\", \"Large\"];",
                                describe(a)
                            ),
                        ),
                        (Ty::State(s), Some(c)) => {
                            let name = self.info.states[*s].name.clone();
                            self.err(
                                c.pos,
                                format!(
                                    "{} is a {name}, so its dropdown chooses from {name}'s options. Leave out in …",
                                    describe(a)
                                ),
                            );
                        }
                        _ => {}
                    }
                }
                if !fits && t != Ty::Error && kind == "dialog" {
                    self.err(
                        a.pos,
                        format!(
                            "a dialog is open while a bool is true, but {} is {}: {example}",
                            describe(a),
                            self.a(&t)
                        ),
                    );
                } else if !fits && t != Ty::Error {
                    self.err(
                        a.pos,
                        format!("{kind} changes {want}, but {} is {}: {example}", describe(a), self.a(&t)),
                    );
                }
                // It changes what it's bound to, so that has to be something that stays.
                let local = matches!(a.kind, ExprKind::Name(_)) && self.info.names.get(&a.id) == Some(&NameRes::Local);
                if local {
                    self.err(
                        a.pos,
                        format!(
                            "{} is a variable of the view, made again each time it's drawn, so what's typed would be lost. Bind the {kind} to a field.",
                            describe(a)
                        ),
                    );
                } else if !matches!(a.kind, ExprKind::Name(_) | ExprKind::Field(..) | ExprKind::Index(..)) {
                    let an = if kind == "input" { "an" } else { "a" };
                    self.err(a.pos, format!("{an} {kind} changes a field, so it needs one: {example}"));
                }
                ctx.scopes.push(HashMap::new());
                ctx.scopes.last_mut().unwrap().insert(
                    crate::parser::VALUE.to_string(),
                    Local {
                        ty: t,
                        readonly: true,
                        pos: a.pos,
                    },
                );
            }
            ("menu", Some(a)) | ("item", Some(a)) => {
                let t = self.expr(a, ctx, Some(&Ty::Str));
                if t != Ty::Str && t != Ty::Error {
                    let an = if kind == "item" { "an" } else { "a" };
                    self.err(a.pos, format!("{an} {kind} shows text, but this is {}.", self.a(&t)));
                }
                if kind == "item" && self.menus == 0 {
                    self.err(pos, "an item goes inside a menu: menu \"File\" { item \"Open\" { … } }");
                }
                if kind == "item" && action.is_none() {
                    self.err(pos, "an item needs what it does: item \"Open\" { … }");
                }
            }
            ("menu", None) => self.err(pos, "a menu needs its name: menu \"File\" { item \"Open\" { … } }"),
            ("tabs", None) => ctx.scopes.push(HashMap::new()),
            ("tab" | "branch", Some(a)) => {
                let t = self.expr(a, ctx, Some(&Ty::Str));
                if t != Ty::Str && t != Ty::Error {
                    self.err(
                        a.pos,
                        format!("a {kind} shows its name, which is text, but this is {}.", self.a(&t)),
                    );
                }
            }
            ("tab", None) => self.err(pos, "a tab needs its name: tab \"Home\" { … }"),
            ("branch", None) => self.err(pos, "a branch needs its name: branch \"Weapons\" { … }"),
            ("progress", Some(a)) => {
                let t = self.expr(a, ctx, Some(&Ty::Float));
                if !matches!(t, Ty::Int | Ty::Float | Ty::Error) {
                    self.err(
                        a.pos,
                        format!("progress shows a number, but this is {}: progress done [to total];", self.a(&t)),
                    );
                }
            }
            ("progress", None) => self.err(pos, "progress needs the number it shows: progress done [to total];"),
            ("item", None) => self.err(pos, "an item needs its words: item \"Open\" { … }"),
            ("divider", None) if self.menus == 0 => self.err(pos, "a divider is a line between items, so it goes inside a menu."),
            ("box" | "circle" | "line", None) if self.canvases == 0 => self.err(
                pos,
                format!("a {kind} is drawn in a canvas: canvas [width 300, height 200] {{ {kind} [x 10, y 10, …]; }}"),
            ),
            ("dialog", None) => {
                self.err(pos, "a dialog needs the bool that opens it: dialog confirming { … }");
                ctx.scopes.push(HashMap::new());
            }
            ("input" | "textarea" | "checkbox" | "slider" | "dropdown", None) => {
                let an = if kind == "input" { "an" } else { "a" };
                self.err(pos, format!("{an} {kind} needs the field it changes: {kind} name;"));
                ctx.scopes.push(HashMap::new());
            }
            ("image", Some(a)) => {
                let t = self.expr(a, ctx, Some(&Ty::Str));
                if t != Ty::Str && t != Ty::Error {
                    self.err(
                        a.pos,
                        format!(
                            "image takes the picture's file name, but this is {}: image \"cat.png\";",
                            self.a(&t)
                        ),
                    );
                }
            }
            ("image", None) => self.err(pos, "image needs the picture's file: image \"cat.png\";"),
            ("every", Some(a)) => {
                let t = self.expr(a, ctx, Some(&Ty::Float));
                if !matches!(t, Ty::Int | Ty::Float | Ty::Error) {
                    self.err(
                        a.pos,
                        format!(
                            "every takes how many seconds to wait, but this is {}: every 1.0 {{ … }}",
                            self.a(&t)
                        ),
                    );
                }
                if action.is_none() {
                    self.err(pos, "every runs its { } again and again: every 1.0 { seconds = seconds + 1; }");
                }
            }
            ("every", None) => self.err(pos, "every needs how many seconds to wait: every 1.0 { … }"),
            ("key", Some(a)) => {
                let t = self.expr(a, ctx, Some(&Ty::Str));
                if t != Ty::Str && t != Ty::Error {
                    self.err(
                        a.pos,
                        format!("key takes the keys as text, but this is {}: key \"ctrl+s\" {{ … }}", self.a(&t)),
                    );
                } else if let ExprKind::Str(spec) = &a.kind {
                    if let Err(why) = crate::native_ui::parse_shortcut(spec) {
                        self.err(a.pos, why);
                    }
                }
                if action.is_none() {
                    self.err(pos, "key runs its { } when the keys are pressed: key \"ctrl+s\" { save(); }");
                }
            }
            ("key", None) => self.err(pos, "key needs which keys: key \"ctrl+s\" { save(); }"),
            ("text", None) => self.err(pos, "text needs something to show: text \"Hello\";"),
            ("button", None) => self.err(pos, "a button needs its words: button \"Start\" { … }"),
            (_, Some(a)) => self.err(
                a.pos,
                format!(
                    "a {kind} doesn't take a value, only [styles]{}.",
                    if kind == "space" { "" } else { " and what's inside it" }
                ),
            ),
            _ => {}
        }
        self.eval_styles(style, ctx);
        self.check_styles(style, kind);
        if !children.is_empty() {
            ctx.scopes.push(HashMap::new());
            let menu = kind == "menu";
            self.menus += menu as usize;
            let canvas = kind == "canvas";
            self.canvases += canvas as usize;
            // A dialog's own pieces are its, not inside a menu around it.
            let outer = if kind == "dialog" { std::mem::take(&mut self.menus) } else { 0 };
            for s in children {
                self.stmt(s, ctx);
            }
            self.menus += outer;
            self.menus -= menu as usize;
            self.canvases -= canvas as usize;
            ctx.scopes.pop();
        }
        let bound = matches!(kind, "input" | "textarea" | "checkbox" | "slider" | "dropdown" | "dialog" | "tabs");
        if let Some(a) = enter {
            let was = std::mem::replace(&mut ctx.in_view, false);
            ctx.scopes.push(HashMap::new());
            for s in a {
                self.stmt(s, ctx);
            }
            ctx.scopes.pop();
            ctx.in_view = was;
        }
        let _ = choices;
        if let Some(a) = action {
            if !matches!(kind, "button" | "every" | "key" | "item" | "box" | "circle") && !bound {
                self.err(pos, format!("a {kind} doesn't do anything when clicked, so it has no {{ }}."));
            }
            let was = std::mem::replace(&mut ctx.in_view, false);
            ctx.scopes.push(HashMap::new());
            for s in a {
                self.stmt(s, ctx);
            }
            ctx.scopes.pop();
            ctx.in_view = was;
        }
        if bound {
            ctx.scopes.pop();
        }
    }

    /// Works out the values in [ ], which can use fields and variables like any expression.
    fn eval_styles(&mut self, items: &[StyleItem], ctx: &mut Ctx) {
        for it in items {
            match &it.value {
                StyleValue::Expr(e) => {
                    self.expr(e, ctx, None);
                }
                StyleValue::Nested(n) => self.eval_styles(n, ctx),
                _ => {}
            }
        }
    }

    /// The words in a view's [ ]. `kind` is the piece they're on, or "view" for the window's own.
    fn check_styles(&mut self, items: &[StyleItem], kind: &str) {
        const NUMS: &[&str] = &[
            "pad",
            "padX",
            "padY",
            "padTop",
            "padRight",
            "padBottom",
            "padLeft",
            "gap",
            "size",
            "weight",
            "grow",
            "minWidth",
            "maxWidth",
            "minHeight",
            "maxHeight",
            "borderWidth",
            "opacity",
            "from",
            "to",
            "step",
            "x",
            "y",
            "toX",
            "toY",
            "radius",
            "thick",
            "columns",
            "cell",
        ];
        const FLAGS: &[&str] = &[
            "tiny", "small", "large", "huge", "bold", "muted", "strike", "nowrap", "shadow", "pill", "left", "center", "right", "top",
            "middle", "bottom", "spread", "subtle", "cover", "down", "open", "animate",
        ];
        const COLOURED: &[&str] = &["color", "background"];
        let window = kind == "view";
        for it in items {
            let n = it.name.as_str();
            let named = self.info.styles.contains(n);
            let colour_word = crate::native_ui::COLOURS.contains(&n);
            let ok_word = |w: &str| crate::native_ui::colour(w).is_some();
            let number = |me: &mut Self, v: &StyleValue, what: &str| match v {
                StyleValue::Expr(e) => {
                    let t = me.info.ty(e);
                    if !matches!(t, Ty::Int | Ty::Float | Ty::Error) {
                        me.err(e.pos, format!("{n} takes a number, like {what}. This is {}.", me.a(&t)));
                    }
                }
                _ => me.err(it.pos, format!("{n} takes a number, like {what}.")),
            };
            match (n, &it.value) {
                ("title", StyleValue::Expr(_)) if window => {}
                ("disabled", StyleValue::Flag) if !window => {}
                ("disabled", StyleValue::Expr(e)) if !window => {
                    let t = self.info.ty(e);
                    if !matches!(t, Ty::Truth | Ty::Error) {
                        self.err(
                            e.pos,
                            format!("disabled takes a bool, like disabled not ready. This is {}.", self.a(&t)),
                        );
                    }
                }
                ("minWidth" | "minHeight", StyleValue::Expr(_)) if window => {}
                ("dark", StyleValue::Flag) if window => {}
                ("dark", StyleValue::Expr(e)) if window => {
                    let t = self.info.ty(e);
                    if !matches!(t, Ty::Truth | Ty::Error) {
                        self.err(e.pos, format!("dark takes a bool, like dark darkMode. This is {}.", self.a(&t)));
                    }
                }
                ("dark", StyleValue::Nested(inner)) if !window => {
                    let inner = inner.clone();
                    self.check_styles(&inner, kind);
                }
                ("dark", _) if !window => self.err(
                    it.pos,
                    "on a piece, dark: holds the styles it has in dark mode: dark:[color blue3]. Dark mode itself is turned on in the view's [ ]: view [dark].",
                ),
                ("font", v) => match v {
                    StyleValue::Expr(e) if matches!(self.info.ty(e), Ty::Str | Ty::Error) => {}
                    _ => self.err(it.pos, "font takes a font file: font \"fonts/Lora.ttf\"."),
                },
                ("tip", v) if !window => match v {
                    StyleValue::Expr(e) if matches!(self.info.ty(e), Ty::Str | Ty::Error) => {}
                    _ => self.err(
                        it.pos,
                        "tip takes text, shown when the mouse rests on the piece: tip \"Saves your work\".",
                    ),
                },
                ("placeholder", v) if !window => match v {
                    StyleValue::Expr(e) if matches!(self.info.ty(e), Ty::Str | Ty::Error) => {}
                    _ => self.err(it.pos, "placeholder takes text: placeholder \"Search…\"."),
                },
                ("width" | "height", StyleValue::Expr(_)) if window => {}
                ("width" | "height", StyleValue::Expr(_) | StyleValue::Word(_)) => {
                    if let StyleValue::Word(w) = &it.value {
                        if w != "fill" {
                            self.err(it.pos, format!("{n} is a number, or fill: {n} 200, {n} fill."));
                        }
                    }
                }
                ("width" | "height", StyleValue::Flag) => {}
                ("background", _) if window || !window => match &it.value {
                    StyleValue::Word(w) if ok_word(w) => {}
                    StyleValue::Expr(_) => {}
                    _ => self.err(it.pos, format!("{n} takes a colour: {n} blue, or {n} \"#2f9e44\".")),
                },
                (n2, v) if NUMS.contains(&n2) && !window => match v {
                    StyleValue::Flag if matches!(n2, "pad" | "gap" | "grow") => {}
                    other => number(self, other, &format!("{n2} {}", if n2 == "opacity" { "0.5" } else { "12" })),
                },
                ("round", StyleValue::Flag | StyleValue::Expr(_)) if !window => {}
                ("border", StyleValue::Flag) if !window => {}
                ("border", StyleValue::Word(w)) if !window && ok_word(w) => {}
                ("border", StyleValue::Expr(_)) if !window => {}
                (c, v) if COLOURED.contains(&c) && !window => match v {
                    StyleValue::Word(w) if ok_word(w) => {}
                    StyleValue::Expr(_) => {}
                    _ => self.err(it.pos, format!("{c} takes a colour: {c} blue, or {c} \"#2f9e44\".")),
                },
                ("hover", StyleValue::Nested(inner)) if !window => {
                    let inner = inner.clone();
                    self.check_styles(&inner, kind);
                }
                (f, StyleValue::Flag) if !window && (FLAGS.contains(&f) || colour_word || named) => {}
                _ => {
                    let mut known: Vec<String> = if window {
                        vec![
                            "title".into(),
                            "width".into(),
                            "height".into(),
                            "minWidth".into(),
                            "minHeight".into(),
                            "background".into(),
                            "font".into(),
                            "dark".into(),
                        ]
                    } else {
                        NUMS.iter()
                            .chain(FLAGS)
                            .chain(COLOURED)
                            .chain(
                                [
                                    "width",
                                    "height",
                                    "round",
                                    "border",
                                    "hover",
                                    "placeholder",
                                    "disabled",
                                    "font",
                                    "tip",
                                    "dark",
                                ]
                                .iter(),
                            )
                            .map(|s| s.to_string())
                            .collect()
                    };
                    if !window {
                        known.extend(self.info.styles.iter().cloned());
                        known.extend(crate::native_ui::COLOURS.iter().map(|s| s.to_string()));
                    }
                    let s = suggest(n, known.clone());
                    if window {
                        self.err(
                            it.pos,
                            format!("a window's view has title, width, height, minWidth, minHeight, background, font and dark.{s}"),
                        );
                    } else if FLAGS.contains(&n) || colour_word || named {
                        self.err(it.pos, format!("{n} is a word on its own, without a value."));
                    } else {
                        self.err(it.pos, format!("Lust has no style called {n}.{s}"));
                    }
                }
            }
            // Values are worked out like any expression, in the view's own place.
            let _ = (&it.value, n);
        }
    }

    fn method_body(&mut self, m: &MethodDecl, class: Option<usize>) {
        let sig = match class {
            Some(c) => self.info.classes[c].methods.get(&m.name).cloned(),
            None => self.info.methods.get(&m.name).cloned(),
        };
        let Some(sig) = sig else { return };
        if m.native {
            // Its body is in Rust (or the host's Lua), so what goes in and out has to be something
            // both sides know.
            let plain = |t: &Ty| matches!(t, Ty::Int | Ty::Float | Ty::Str | Ty::Truth | Ty::Class(_) | Ty::Void | Ty::Error);
            for (p, (_, ty)) in m.params.iter().zip(&sig.params) {
                if !plain(ty) {
                    self.err(
                        p.pos,
                        format!(
                            "a native method takes ints, floats, text, bools and things made from classes, but {} is {}.",
                            p.name,
                            self.a(ty)
                        ),
                    );
                }
            }
            if !plain(&sig.ret) {
                self.err(
                    m.pos,
                    format!(
                        "a native method gives back an int, a float, text, a bool or a thing made from a class, but {} gives back {}.",
                        m.name,
                        self.a(&sig.ret)
                    ),
                );
            }
            self.info.natives.insert(m.pos);
            return;
        }
        let mut ctx = Ctx {
            class,
            ret: Some(sig.ret.clone()),
            scopes: vec![HashMap::new()],
            in_try: false,
            in_hook: false,
            hook_base: 0,
            made: Default::default(),
            in_view: false,
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
                    prev.pos.row()
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

    /// Does this class have a state of its own (state is …; in its has)?
    pub fn stateful(&self, c: usize) -> bool {
        self.info.field(c, "state").is_some()
    }

    /// Is this a new thing, made in a method of a class with a state, so it belongs to that state?
    fn makes_owned(&self, e: &Expr, ctx: &Ctx) -> bool {
        let in_state = ctx.class.map(|c| self.stateful(c)).unwrap_or(false);
        in_state && (matches!(e.kind, ExprKind::New { .. }) || self.info.creates.contains_key(&e.id))
    }

    /// Storing something made in a state somewhere that outlasts the state needs keep.
    fn store_check(&mut self, value: &Expr, place: &str, inject: bool, ctx: &Ctx) {
        let made = match &value.kind {
            ExprKind::Name(n) => ctx.made.contains(n),
            _ => self.makes_owned(value, ctx),
        };
        if !made {
            return;
        }
        let Some(c) = ctx.class else { return };
        let cn = self.info.classes[c].name.clone();
        let _ = inject;
        let (start, how) = match &value.kind {
            ExprKind::Name(n) => (n.clone(), format!("keep {n} in {place};")),
            ExprKind::New { class, .. } => (
                format!("this new {class}"),
                format!("{class} thing = new {class} {{ … }}; keep thing in {place};"),
            ),
            _ => ("this".to_string(), format!("keep it in {place};")),
        };
        self.err(
            value.pos,
            format!("{start} is made while this {cn} is in a state, so it ends when the state does. To store it, keep it: {how}"),
        );
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
                self.maker_ok = true;
                let want = self.resolve(ty);
                self.key_maker(ty, &want, ctx);
                let got = self.expr(value, ctx, Some(&want));
                self.expect_fits(value, &got, &want, &format!("{name} is {}", self.a(&want)));
                self.info.decl_types.insert(value.id, want.clone());
                self.declare(ctx, name, want, *pos, false);
                if self.makes_owned(value, ctx) {
                    ctx.made.insert(name.clone());
                } else {
                    ctx.made.remove(name);
                }
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
                let to_local = matches!(&target.kind, ExprKind::Name(_)) && self.info.names.get(&target.id) == Some(&NameRes::Local);
                if to_local {
                    if let ExprKind::Name(n) = &target.kind {
                        if self.makes_owned(value, ctx) {
                            ctx.made.insert(n.clone());
                        } else {
                            ctx.made.remove(n);
                        }
                    }
                } else {
                    self.store_check(value, &describe(target), false, ctx);
                }
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
                // Pieces in a loop over keys are told apart by the key, not their turn.
                let keyed = match kind {
                    LoopKind::Each { vars, source } => match (self.info.ty(source), vars.first()) {
                        (Ty::Map(_), Some(v)) => Some(v.1.clone()),
                        (Ty::CMap(cols), Some(v)) if matches!(cols[0].1, Ty::Int | Ty::Str) => Some(v.1.clone()),
                        _ => None,
                    },
                    _ => None,
                };
                if let Some(k) = &keyed {
                    self.loop_keys.push(k.clone());
                }
                for s in body {
                    self.stmt(s, ctx);
                }
                if keyed.is_some() {
                    self.loop_keys.pop();
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
            Stmt::Element {
                kind,
                arg,
                style,
                children,
                action,
                enter,
                choices,
                pos,
            } => self.element(kind, arg, style, children, action, enter, choices, *pos, ctx),
            Stmt::SaveDecl { name, items, pos } => {
                let top = ctx.class.is_none() && ctx.ret.is_none() && !ctx.in_hook;
                if !top {
                    self.err(*pos, "a save is set up at the top of the file: save gameSession = [Farm, player];");
                    return;
                }
                if self.info.saves.contains_key(name) {
                    self.err(*pos, format!("there's already a save called {name}."));
                    return;
                }
                let mut list = Vec::new();
                for (n, p) in items {
                    if let Some(&c) = self.info.class_index.get(n) {
                        let ci = &self.info.classes[c];
                        if ci.is_node {
                            self.err(
                                *p,
                                format!("{n} is a node, which opens and closes, so it can't be saved. Save what's in it instead."),
                            );
                        } else if !ci.is_object {
                            self.err(
                                *p,
                                format!("{n} is a class. Save things made from it, like a variable holding one."),
                            );
                        } else {
                            list.push((n.clone(), SaveItem::Object(c)));
                        }
                        continue;
                    }
                    match self.lookup(ctx, n).map(|l| l.ty.clone()) {
                        Some(t) => {
                            if let Some(p2) = crate::stdlib::json_problem(&t, &|t| self.a(t)) {
                                self.err(
                                    *p,
                                    format!(
                                        "{p2} Saves hold numbers, text, bools, states, arrays, maps, cMaps and things made from classes."
                                    ),
                                );
                            } else {
                                list.push((n.clone(), SaveItem::Var(t)));
                            }
                        }
                        None => self.err(*p, format!("there's nothing called {n} to save. Declare it first, above the save.")),
                    }
                }
                self.info.saves.insert(name.clone(), list);
            }
            Stmt::Save(name, pos) | Stmt::Load(name, pos) => {
                let word = if matches!(s, Stmt::Save(..)) { "save" } else { "load" };
                match self.info.saves.get(name).cloned() {
                    None => self.err(
                        *pos,
                        format!("there's no save called {name}. Set one up first: save {name} = [Farm, player];"),
                    ),
                    Some(list) => {
                        let top = ctx.class.is_none() && ctx.ret.is_none() && !ctx.in_hook;
                        if !top {
                            if let Some((v, _)) = list.iter().find(|(_, i)| matches!(i, SaveItem::Var(_))) {
                                self.err(*pos, format!("{name} holds {v}, which only the top of the file can see. {word} it from there, or keep {v} in an object."));
                            }
                        }
                    }
                }
            }
            Stmt::Keep { value, into, pos } => {
                let t = self.expr(value, ctx, None);
                if !matches!(t, Ty::Class(_) | Ty::Error) {
                    self.err(
                        value.pos,
                        format!(
                            "only things made from classes are kept. This is {}, which is copied wherever it goes.",
                            self.a(&t)
                        ),
                    );
                }
                if let ExprKind::Name(n) = &value.kind {
                    ctx.made.remove(n);
                }
                if let Some(into) = into {
                    let place = self.expr(into, ctx, None);
                    match &place {
                        Ty::Array(el) => {
                            if !self.fits(&t, el) {
                                self.err(
                                    into.pos,
                                    format!("{} holds {}, so {} can't go in it.", describe(into), self.a(el), self.a(&t)),
                                );
                            }
                        }
                        Ty::Error => {}
                        _ => {
                            self.writable(into, ctx, *pos);
                            if !matches!(&into.kind, ExprKind::Name(_) | ExprKind::Field(..) | ExprKind::Index(..)) {
                                self.err(
                                    into.pos,
                                    "keep puts it in an array, a field or a variable: keep sword in player.bag;",
                                );
                            } else if !self.fits(&t, &place) {
                                self.err(
                                    into.pos,
                                    format!("{} is {}, so {} can't go in it.", describe(into), self.a(&place), self.a(&t)),
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    fn each(&mut self, vars: &[(TypeRef, String, Pos)], source: &Expr, ctx: &mut Ctx) {
        let src = self.expr(source, ctx, None);
        let declared: Vec<Ty> = vars.iter().map(|(t, _, _)| self.resolve(t)).collect();
        let wanted: Option<Vec<Ty>> = match &src {
            Ty::Array(e) => Some(vec![(**e).clone()]),
            Ty::Map(v) => Some(vec![Ty::Str, (**v).clone()]),
            Ty::CMap(cols) => Some(cols.iter().map(|(_, t)| t.clone()).collect()),
            Ty::Table(t) | Ty::Query(t) => Some(t.cols.iter().map(|(_, t)| t.clone()).collect()),
            Ty::StateList(s) => Some(vec![Ty::State(*s)]),
            Ty::Error => None,
            other => {
                self.err(
                    source.pos,
                    format!(
                        "a loop can go through an array, a map, a cMap or a state's options, but this is {}.",
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
                    Ty::CMap(cols) => {
                        let c: Vec<String> = cols.iter().map(|(n, t)| format!("{} {n}", self.show(t))).collect();
                        format!("loop ({} in {}) {{ }}", c.join(", "), describe(source))
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
                if let Ty::TableRow(tt) = self.info.ty(t) {
                    if tt.cols.first().map(|c| &c.0) == Some(f) {
                        self.err(
                            *fp,
                            format!(
                                "{f} is {}'s key, so it can't be changed. Remove the row and add it again with the new {f}.",
                                tt.name
                            ),
                        );
                    }
                }
                if let Ty::Row(cols) = self.info.ty(t) {
                    if cols.first().map(|c| &c.0) == Some(f) {
                        self.err(
                            *fp,
                            format!("{f} is this cMap's key, so it can't be changed. Remove the row and add it again with the new {f}."),
                        );
                    }
                }
                if self.info.options.contains_key(&target.id) {
                    self.err(pos, "a state's option can't be changed.");
                }
            }
            ExprKind::Index(t, _) => {
                if matches!(self.info.ty(t), Ty::CMap(_) | Ty::Table(_)) {
                    self.err(
                        pos,
                        "a whole row can't be replaced. Change its columns instead: bag[1].amount = 5;, or add a new row with :inject.",
                    );
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
            (Ty::CMap(a), Ty::CMap(b)) => a == b,
            // A table's rows, or a query's, copied into a cMap.
            (Ty::Table(t) | Ty::Query(t), Ty::CMap(b)) => t.cols == *b,
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
            ExprKind::Random(lo, hi) => {
                let a = self.expr(lo, ctx, None);
                let b = self.expr(hi, ctx, None);
                for (x, t) in [(lo, &a), (hi, &b)] {
                    if !matches!(t, Ty::Int | Ty::Float | Ty::Error) {
                        self.err(x.pos, format!("Random picks between two numbers, but this is {}.", self.a(t)));
                    }
                }
                if a == Ty::Float || b == Ty::Float || expected == Some(&Ty::Float) {
                    Ty::Float
                } else {
                    Ty::Int
                }
            }
            ExprKind::Saved(name) => {
                if !self.info.saves.contains_key(name) {
                    self.err(
                        e.pos,
                        format!("there's no save called {name}. Set one up first: save {name} = [Farm, player];"),
                    );
                }
                Ty::Truth
            }
            ExprKind::Nothing => match expected {
                Some(t) if t.can_be_nothing() => t.clone(),
                _ => Ty::Nothing,
            },
            ExprKind::Name(n) => self.name(e, n, ctx),
            ExprKind::Field(target, f, fpos) => self.field(e, target, f, *fpos, ctx),
            ExprKind::Method(target, m, args, mpos) => self.method_call(e, target, m, args, *mpos, ctx, expected),
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
                    Ty::Table(tt) => {
                        let (kn, kt) = tt.cols[0].clone();
                        let i = self.expr(index, ctx, Some(&kt));
                        if i != kt && i != Ty::Error && kt != Ty::Error {
                            self.err(
                                index.pos,
                                format!(
                                    "{}'s rows are found by their {kn}, which is {}. This is {}.",
                                    tt.name,
                                    self.a(&kt),
                                    self.a(&i)
                                ),
                            );
                        }
                        Ty::TableRow(tt)
                    }
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
                    Ty::CMap(cols) => {
                        let (kn, kt) = cols[0].clone();
                        let i = self.expr(index, ctx, Some(&kt));
                        if i != kt && i != Ty::Error && kt != Ty::Error {
                            let ex = if kt == Ty::Str { "[\"tea\"]" } else { "[1]" };
                            self.err(
                                index.pos,
                                format!(
                                    "this cMap's rows are found by their {kn}, which is {}: {}{ex}. This is {}.",
                                    self.a(&kt),
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
                    Ty::Str | Ty::Array(_) | Ty::Map(_) | Ty::CMap(_) | Ty::Table(_) | Ty::Query(_) | Ty::Error => Ty::Int,
                    other => {
                        self.err(
                            e.pos,
                            format!("# counts text, arrays, maps and cMaps, but this is {}.", self.a(&other)),
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
                let missing: Vec<String> = self.info.classes[c]
                    .required
                    .iter()
                    .filter(|r| !inits.iter().any(|(f, _, _)| f == *r))
                    .cloned()
                    .collect();
                if !missing.is_empty() {
                    let ex: Vec<String> = missing.iter().map(|m| format!("{m} = …")).collect();
                    self.err(
                        *class_pos,
                        format!(
                            "{class} needs {} filled in: new {class} {{ {} }}",
                            missing.join(" and "),
                            ex.join(", ")
                        ),
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
                        Conv::Upper | Conv::Lower | Conv::Trim => Ty::Str,
                    };
                }
                match conv {
                    Conv::Text => {
                        if !matches!(t, Ty::Int | Ty::Float | Ty::Truth | Ty::Str | Ty::State(_)) {
                            self.err(
                                *cpos,
                                format!(
                                    ".text() works on numbers, bools, text and state options, but this is {}.",
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
                    Conv::Upper | Conv::Lower | Conv::Trim => {
                        if t != Ty::Str {
                            let n = match conv {
                                Conv::Upper => "upper",
                                Conv::Lower => "lower",
                                _ => "trim",
                            };
                            self.err(*cpos, format!(".{n}() works on text, but this is {}.", self.a(&t)));
                        }
                        Ty::Str
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
        if self.info.assemblies.contains_key(n) {
            self.err(
                e.pos,
                format!("{n} is an assembly. Make things with it like this: {n}:create(Item)"),
            );
            return Ty::Error;
        }
        if let Some(&d) = self.info.db_index.get(n) {
            let first = self.info.databases[d]
                .tables
                .first()
                .map(|t| t.0.clone())
                .unwrap_or_else(|| "items".into());
            self.err(e.pos, format!("{n} is a database. Use its tables: {n}.{first}"));
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

    /// Calls on tables and queries: Shop.items:inject([…]), :where(price < 3.0), :orderBy(name) and so on.
    #[allow(clippy::too_many_arguments)]
    fn table_call(&mut self, e: &Expr, t: &Ty, tt: &Rc<TableTy>, m: &str, args: &[Expr], mpos: Pos, ctx: &mut Ctx) -> Ty {
        let is_table = matches!(t, Ty::Table(_));
        let query = Ty::Query(tt.clone());
        let n = tt.name.clone();
        let fail = |me: &mut Self, ctx: &mut Ctx, msg: String| {
            for a in args {
                me.expr(a, ctx, None);
            }
            me.err(mpos, msg);
            Ty::Error
        };
        let op: &'static str = match (m, is_table, args.len()) {
            ("inject", true, 1) => "inject",
            ("has", true, 1) => "has",
            ("remove", true, 1) => "removeKey",
            ("remove", false, 0) => "removeAll",
            ("where", _, 1) => "where",
            ("orderBy", _, 1) => "orderBy",
            ("reverse", _, 0) => "reverse",
            ("first", _, 1) => "first",
            ("count", _, 0) => "count",
            ("inject" | "has", false, _) => {
                return fail(self, ctx, format!(":{m} works on the table itself: {n}:{m}(…). This is a query."));
            }
            ("remove", true, 0) => {
                return fail(
                    self,
                    ctx,
                    format!("remove takes a key: {n}:remove(…). To remove several rows, pick them first: {n}:where(…):remove()."),
                );
            }
            ("remove", false, _) => {
                return fail(
                    self,
                    ctx,
                    "a query's remove takes nothing: it removes the rows the query picked.".into(),
                )
            }
            ("inject" | "has" | "where" | "orderBy" | "first", _, _) => return fail(self, ctx, format!(":{m} takes one value.")),
            ("reverse" | "count", _, _) => return fail(self, ctx, format!(":{m} takes nothing in its ( ).")),
            _ => {
                let what = if is_table {
                    "Tables have inject, has, remove, where, orderBy, reverse, first and count."
                } else {
                    "Queries have where, orderBy, reverse, first, count and remove."
                };
                return fail(self, ctx, format!("{n} has no method called {m}. {what}"));
            }
        };
        self.info.table_ops.insert(e.id, op);
        self.info.calls.insert(e.id, CallRes::Builtin);
        match op {
            "inject" => {
                self.row(&args[0], &tt.cols, ctx);
                if let ExprKind::List { items, .. } = &args[0].kind {
                    if items.len() + 1 == tt.cols.len() && tt.cols[0].1 == Ty::Str {
                        self.err(
                            args[0].pos,
                            format!("{n}'s key ({}) is text, so each new row needs it first.", tt.cols[0].0),
                        );
                    }
                }
                Ty::Void
            }
            "has" | "removeKey" => {
                let (kn, kt) = tt.cols[0].clone();
                let got = self.expr(&args[0], ctx, Some(&kt));
                self.expect_fits(&args[0], &got, &kt, &format!("{n}'s key, {kn}, is {}", self.a(&kt)));
                if op == "has" {
                    Ty::Truth
                } else {
                    Ty::Void
                }
            }
            "where" => {
                ctx.scopes.push(
                    tt.cols
                        .iter()
                        .map(|(c, t)| {
                            (
                                c.clone(),
                                Local {
                                    ty: t.clone(),
                                    readonly: true,
                                    pos: mpos,
                                },
                            )
                        })
                        .collect(),
                );
                let got = self.expr(&args[0], ctx, Some(&Ty::Truth));
                ctx.scopes.pop();
                self.expect_truth(&args[0], &got, "where");
                let names: Vec<String> = tt.cols.iter().map(|(c, _)| c.clone()).collect();
                self.mark_columns(&args[0], &names);
                if let Err((pos, msg)) = self.sql_ok(&args[0]) {
                    self.err(pos, msg);
                }
                query
            }
            "orderBy" => {
                match &args[0].kind {
                    ExprKind::Name(c) if tt.cols.iter().any(|(x, _)| x == c) => {
                        self.info.columns.insert(args[0].id, c.clone());
                        let ct = tt.cols.iter().find(|(x, _)| x == c).unwrap().1.clone();
                        self.info.types.insert(args[0].id, ct);
                    }
                    _ => {
                        let cols: Vec<String> = tt.cols.iter().map(|(c, _)| c.clone()).collect();
                        self.err(
                            args[0].pos,
                            format!("orderBy takes the name of one of {n}'s columns: {}.", cols.join(", ")),
                        );
                    }
                }
                query
            }
            "reverse" => query,
            "first" => {
                let got = self.expr(&args[0], ctx, Some(&Ty::Int));
                self.expect_fits(&args[0], &got, &Ty::Int, "first's count is an int");
                query
            }
            "count" => Ty::Int,
            _ => Ty::Void,
        }
    }

    /// Inside :where( ), the names that are the table's columns.
    fn mark_columns(&mut self, e: &Expr, cols: &[String]) {
        let mut found = Vec::new();
        let names = &self.info.names;
        crate::rust::walk_expr(e, &mut |x| {
            if let ExprKind::Name(n) = &x.kind {
                if cols.contains(n) && names.get(&x.id).map(|r| *r == NameRes::Local).unwrap_or(false) {
                    found.push((x.id, n.clone()));
                }
            }
        });
        self.info.columns.extend(found);
    }

    pub fn mentions_column(info: &Info, e: &Expr) -> bool {
        let mut found = false;
        crate::rust::walk_expr(e, &mut |x| {
            if info.columns.contains_key(&x.id) {
                found = true;
            }
        });
        found
    }

    /// Can this part of a :where( ) become SQL? Parts that don't use the columns are worked out by
    /// Lust first, so anything goes there.
    fn sql_ok(&self, e: &Expr) -> Result<(), (Pos, String)> {
        if !Self::mentions_column(&self.info, e) {
            return Ok(());
        }
        let bad = || {
            Err((
                e.pos,
                "where can use a table's columns with maths, comparisons, &&, ||, not, # and :contains, :startsWith and :endsWith. Work anything else out first, in a variable.".to_string(),
            ))
        };
        match &e.kind {
            ExprKind::Name(_) => Ok(()),
            ExprKind::Bin(_, l, r, _) => {
                self.sql_ok(l)?;
                self.sql_ok(r)
            }
            ExprKind::Not(x) | ExprKind::Neg(x) => self.sql_ok(x),
            ExprKind::Len(x) if self.info.ty(x) == Ty::Str => self.sql_ok(x),
            ExprKind::Method(t, m, args, _)
                if matches!(m.as_str(), "contains" | "startsWith" | "endsWith") && self.info.ty(t) == Ty::Str =>
            {
                self.sql_ok(t)?;
                if args.iter().any(|a| Self::mentions_column(&self.info, a)) {
                    return bad();
                }
                Ok(())
            }
            _ => bad(),
        }
    }

    /// LootSpawner:create(Item)
    fn create_call(&mut self, e: &Expr, asm: &str, recipes: &[(usize, usize)], m: &str, args: &[Expr], mpos: Pos) -> Ty {
        let makes: Vec<String> = recipes.iter().map(|(c, _)| self.info.classes[*c].name.clone()).collect();
        let example = makes.first().cloned().unwrap_or_else(|| "Item".into());
        if m != "create" {
            self.err(mpos, format!("an assembly makes things with create: {asm}:create({example})"));
            return Ty::Error;
        }
        let class = match args {
            [a] => match &a.kind {
                ExprKind::Name(n) => Some((n.clone(), a.pos)),
                _ => None,
            },
            _ => None,
        };
        let Some((class, cpos)) = class else {
            self.err(mpos, format!("create takes the name of the class to make: {asm}:create({example})"));
            return Ty::Error;
        };
        match recipes.iter().find(|(c, _)| self.info.classes[*c].name == class) {
            Some(&(c, _)) => {
                self.info.creates.insert(e.id, (asm.to_string(), c));
                self.info.calls.insert(e.id, CallRes::Builtin);
                Ty::Class(c)
            }
            None => {
                let list = if makes.is_empty() {
                    "nothing yet".to_string()
                } else {
                    makes.join(", ")
                };
                self.err(cpos, format!("{asm} has no recipe for {class}. It makes {list}."));
                Ty::Error
            }
        }
    }

    /// A call to the standard library, once it's known which one: checks the arguments.
    fn lib_call(&mut self, e: &Expr, lib: crate::stdlib::Lib, args: &[Expr], m: &str, mpos: Pos, ctx: &mut Ctx) -> Ty {
        let params: Vec<(String, Ty)> = lib.params.iter().map(|(n, t)| (n.to_string(), t.clone())).collect();
        self.info.calls.insert(e.id, CallRes::Builtin);
        self.args(args, &params, m, mpos, ctx);
        let ret = lib.ret.clone();
        self.info.libs.insert(e.id, lib);
        ret
    }

    /// Maths:sqrt(2.0), Files:read("notes.txt"), Json:read(text) and the rest.
    #[allow(clippy::too_many_arguments)]
    fn object_call(&mut self, e: &Expr, obj: &str, m: &str, args: &[Expr], mpos: Pos, ctx: &mut Ctx, expected: Option<&Ty>) -> Ty {
        use crate::stdlib::Lib;
        if obj == "Json" {
            let one = |me: &mut Self, ctx: &mut Ctx| -> Option<Ty> {
                if args.len() != 1 {
                    me.err(mpos, format!("Json:{m} takes one value, but gets {}.", args.len()));
                    for a in args {
                        me.expr(a, ctx, None);
                    }
                    return None;
                }
                Some(me.expr(&args[0], ctx, if m == "read" { Some(&Ty::Str) } else { None }))
            };
            match m {
                "write" | "pretty" => {
                    let Some(t) = one(self, ctx) else { return Ty::Str };
                    if let Some(p) = crate::stdlib::json_problem(&t, &|t| self.a(t)) {
                        self.err(
                            args[0].pos,
                            format!("{p} JSON holds numbers, text, bools, states, arrays, maps, cMaps and things made from classes."),
                        );
                    } else if t == Ty::Void || t == Ty::Nothing {
                        self.err(args[0].pos, "there's nothing here to turn into JSON.");
                    }
                    self.info.calls.insert(e.id, CallRes::Builtin);
                    self.info.libs.insert(
                        e.id,
                        Lib {
                            func: format!("json_{m}"),
                            params: vec![],
                            ret: Ty::Str,
                        },
                    );
                    return Ty::Str;
                }
                "read" => {
                    let got = one(self, ctx);
                    if let Some(g) = got {
                        if g != Ty::Str && g != Ty::Error {
                            self.err(args[0].pos, format!("Json:read reads text, but this is {}.", self.a(&g)));
                        }
                    }
                    let Some(want) = expected.cloned().filter(|t| !matches!(t, Ty::Void | Ty::Nothing)) else {
                        self.err(
                            mpos,
                            "Json:read needs to know what it's reading, so give it a place to go: Player p = Json:read(text);",
                        );
                        return Ty::Error;
                    };
                    if let Some(p) = crate::stdlib::json_problem(&want, &|t| self.a(t)) {
                        self.err(
                            mpos,
                            format!("{p} JSON holds numbers, text, bools, states, arrays, maps, cMaps and things made from classes."),
                        );
                        return Ty::Error;
                    }
                    if let Ty::Class(c) = want {
                        if self.info.classes[c].is_object {
                            self.err(mpos, format!("{} is an object, and there's only ever one, so it can't be read from JSON. Read into its fields instead.", self.info.classes[c].name));
                            return Ty::Error;
                        }
                    }
                    self.info.calls.insert(e.id, CallRes::Builtin);
                    self.info.libs.insert(
                        e.id,
                        Lib {
                            func: "json_read".into(),
                            params: vec![],
                            ret: want.clone(),
                        },
                    );
                    return want;
                }
                _ => {}
            }
        } else {
            let types: Vec<Ty> = args.iter().map(|a| self.expr(a, ctx, None)).collect();
            if let Some(found) = crate::stdlib::object_method(obj, m, &types) {
                match found {
                    Ok(lib) => {
                        self.info.calls.insert(e.id, CallRes::Builtin);
                        if args.len() != lib.params.len() {
                            self.err(
                                e.pos,
                                format!(
                                    "{obj}:{m} takes {} value{}, but gets {}.",
                                    lib.params.len(),
                                    if lib.params.len() == 1 { "" } else { "s" },
                                    args.len()
                                ),
                            );
                        } else {
                            for ((a, got), (pn, pt)) in args.iter().zip(&types).zip(&lib.params) {
                                self.expect_fits(a, got, pt, &format!("{obj}:{m}'s {pn} is {}", self.a(pt)));
                            }
                        }
                        let ret = lib.ret.clone();
                        self.info.libs.insert(e.id, lib);
                        return ret;
                    }
                    Err(msg) => {
                        self.err(mpos, msg);
                        return Ty::Error;
                    }
                }
            }
        }
        if obj == "Json" {
            for a in args {
                self.expr(a, ctx, None);
            }
        }
        let s = suggest(
            m,
            crate::stdlib::listing(obj)
                .replace(" and ", ", ")
                .split(", ")
                .map(|x| x.trim_start_matches("and ").to_string())
                .collect::<Vec<String>>(),
        );
        self.err(
            mpos,
            format!("{obj} has no method called {m}.{s} It has {}.", crate::stdlib::listing(obj)),
        );
        Ty::Error
    }

    fn field(&mut self, e: &Expr, target: &Expr, f: &str, fpos: Pos, ctx: &mut Ctx) -> Ty {
        if let ExprKind::Name(n) = &target.kind {
            if crate::stdlib::OBJECTS.contains(&n.as_str()) {
                if n == "Maths" && f == "pi" {
                    self.info.libs.insert(
                        e.id,
                        crate::stdlib::Lib {
                            func: "maths_pi".into(),
                            params: vec![],
                            ret: Ty::Float,
                        },
                    );
                    return Ty::Float;
                }
                self.err(
                    fpos,
                    format!(
                        "{n} has no field called {f}. Its methods are called with : like {n}:{}(…).",
                        crate::stdlib::listing(n).split(',').next().unwrap_or("")
                    ),
                );
                return Ty::Error;
            }
        }
        if let ExprKind::Name(n) = &target.kind {
            if let Some(&d) = self.info.db_index.get(n) {
                let db = &self.info.databases[d];
                return match db.tables.iter().position(|(tn, _)| tn == f) {
                    Some(t) => Ty::Table(Rc::new(TableTy {
                        db: d,
                        table: t,
                        name: format!("{}.{f}", crate::ast::shown(n)),
                        cols: db.tables[t].1.clone(),
                    })),
                    None => {
                        let names: Vec<String> = db.tables.iter().map(|t| t.0.clone()).collect();
                        let s = suggest(f, names.clone());
                        self.err(
                            fpos,
                            format!("{n} has no table called {f}.{s} Its tables are {}.", names.join(", ")),
                        );
                        Ty::Error
                    }
                };
            }
        }
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
            Ty::TableRow(tt) => match tt.cols.iter().find(|(n, _)| n == f) {
                Some((_, t)) => t.clone(),
                None => {
                    let names: Vec<String> = tt.cols.iter().map(|(n, _)| n.clone()).collect();
                    let s = suggest(f, names.clone());
                    self.err(
                        fpos,
                        format!("{} has no column called {f}.{s} Its columns are {}.", tt.name, names.join(", ")),
                    );
                    Ty::Error
                }
            },
            Ty::Row(cols) => match cols.iter().find(|(n, _)| n == f) {
                Some((_, t)) => t.clone(),
                None => {
                    let names: Vec<String> = cols.iter().map(|(n, _)| n.clone()).collect();
                    let s = suggest(f, names.clone());
                    self.err(
                        fpos,
                        format!("this cMap has no column called {f}.{s} Its columns are {}.", names.join(", ")),
                    );
                    Ty::Error
                }
            },
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
                "ask" => (vec![("question", Some(Ty::Str))], Ty::Str),
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
        if name == "random" {
            self.err(e.pos, "Lust writes it Random[1, 6], with the lowest and highest number in [ ].");
            return Ty::Int;
        }
        if name == "readText" || name == "writeText" {
            let new = if name == "readText" {
                "Files:read(\"notes.txt\")"
            } else {
                "Files:write(\"notes.txt\", text)"
            };
            self.err(e.pos, format!("{name} is part of Files now: {new}"));
            return Ty::Error;
        }
        let s = suggest(name, known);
        self.err(e.pos, format!("there's no method called {name}.{s}"));
        Ty::Error
    }

    fn method_call(&mut self, e: &Expr, target: &Expr, m: &str, args: &[Expr], mpos: Pos, ctx: &mut Ctx, expected: Option<&Ty>) -> Ty {
        if let ExprKind::Name(n) = &target.kind {
            if crate::stdlib::OBJECTS.contains(&n.as_str()) {
                return self.object_call(e, n, m, args, mpos, ctx, expected);
            }
            if let Some(&c) = self.info.class_index.get(n) {
                if self.info.classes[c].has_view && m == "open" {
                    if !args.is_empty() {
                        self.err(mpos, "open takes nothing in its ( ): App:open();");
                    }
                    self.info.view_opens.insert(e.id, c);
                    self.info.calls.insert(e.id, CallRes::Builtin);
                    return Ty::Void;
                }
                if m == "open" && !self.info.classes[c].is_node {
                    self.err(
                        mpos,
                        format!("{n} has no view, so it can't open a window. Give it one: view [title \"{n}\"] {{ … }}"),
                    );
                    return Ty::Error;
                }
            }
            if let Some(recipes) = self.info.assemblies.get(n).cloned() {
                return self.create_call(e, n, &recipes, m, args, mpos);
            }
        }
        let t = self.expr(target, ctx, None);
        if let Ty::Class(c) = t {
            if self.info.classes[c].is_node && matches!(m, "open" | "close" | "isOpen") {
                if !args.is_empty() {
                    self.err(mpos, format!("{m} takes nothing in its ( )."));
                }
                let op: &'static str = match m {
                    "open" => "open",
                    "close" => "close",
                    _ => "isOpen",
                };
                self.info.node_ops.insert(e.id, op);
                self.info.calls.insert(e.id, CallRes::Builtin);
                return if op == "isOpen" { Ty::Truth } else { Ty::Void };
            }
        }
        if let Ty::Table(tt) | Ty::Query(tt) = &t {
            let tt = tt.clone();
            return self.table_call(e, &t, &tt, m, args, mpos, ctx);
        }
        // bag:sortBy(amount) and players:sortBy(score): a column's or a field's name, in order.
        if m == "sortBy" {
            let names: Option<Vec<(String, Ty)>> = match &t {
                Ty::CMap(cols) => Some(cols.to_vec()),
                Ty::Array(el) => match &**el {
                    Ty::Class(c) => Some(
                        self.all_fields(*c)
                            .into_iter()
                            .map(|f| {
                                let ty = self.info.field(*c, &f).map(|x| x.ty.clone()).unwrap_or(Ty::Error);
                                (f, ty)
                            })
                            .collect(),
                    ),
                    _ => None,
                },
                _ => None,
            };
            if let Some(names) = names {
                let what = if matches!(t, Ty::CMap(_)) { "column" } else { "field" };
                let pick = match args {
                    [a] => match &a.kind {
                        ExprKind::Name(n) => names.iter().find(|(x, _)| x == n).cloned().map(|x| (a, x)),
                        _ => None,
                    },
                    _ => None,
                };
                let shown: Vec<String> = names
                    .iter()
                    .filter(|(_, t)| matches!(t, Ty::Int | Ty::Float | Ty::Str))
                    .map(|(n, _)| n.clone())
                    .collect();
                match pick {
                    Some((a, (n, ty))) if matches!(ty, Ty::Int | Ty::Float | Ty::Str) => {
                        self.info.columns.insert(a.id, n);
                        self.info.table_ops.insert(e.id, "sortBy");
                        self.info.calls.insert(e.id, CallRes::Builtin);
                    }
                    Some((_, (n, _))) => self.err(args[0].pos, format!("sortBy puts numbers or text in order, but {n} is neither.")),
                    None => self.err(
                        mpos,
                        format!("sortBy takes the name of a {what} with numbers or text: {}.", shown.join(", ")),
                    ),
                }
                return Ty::Void;
            }
        }
        let builtin: Option<(Vec<(String, Ty)>, Ty)> = match (&t, m) {
            (Ty::CMap(_), "reverse") => Some((vec![], Ty::Void)),
            (Ty::Array(el), "inject") => Some((vec![("item".into(), (**el).clone())], Ty::Void)),
            (Ty::Array(_), "remove") => Some((vec![("position".into(), Ty::Int)], Ty::Void)),
            (Ty::Map(_), "has") => Some((vec![("key".into(), Ty::Str)], Ty::Truth)),
            (Ty::Map(_), "remove") => Some((vec![("key".into(), Ty::Str)], Ty::Void)),
            (Ty::CMap(cols), "remove") => Some((vec![cols[0].clone()], Ty::Void)),
            (Ty::CMap(cols), "has") => Some((vec![cols[0].clone()], Ty::Truth)),
            (Ty::CMap(cols), "inject") => {
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
            if m == "inject" && matches!(t, Ty::Array(_)) && args.len() == 1 {
                self.store_check(&args[0], &describe(target), true, ctx);
            }
            return ret;
        }
        if let Some(found) = crate::stdlib::value_method(&t, m) {
            return match found {
                Ok(lib) => self.lib_call(e, lib, args, m, mpos, ctx),
                Err(msg) => {
                    for a in args {
                        self.expr(a, ctx, None);
                    }
                    self.err(mpos, msg);
                    Ty::Error
                }
            };
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
                    Ty::Array(_) => " Arrays have inject, remove, has, find, join, sort and reverse.",
                    Ty::Map(_) => " Maps have has and remove.",
                    Ty::CMap(_) => " cMaps have inject, remove and has.",
                    Ty::Str => {
                        " Text has contains, startsWith, endsWith, find, replace, split, slice, matches, findAll, replaceAll and capture, and .upper(), .lower() and .trim()."
                    }
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

    /// One cMap row: a value for every column, or for every column but the key.
    fn row(&mut self, item: &Expr, cols: &Rc<Vec<(String, Ty)>>, ctx: &mut Ctx) {
        let ExprKind::List { items, keys: None } = &item.kind else {
            let t = self.expr(item, ctx, None);
            if t != Ty::Error {
                self.err(item.pos, "a cMap row is written in [ ], one value per column: [1, \"map\", 1]");
            }
            return;
        };
        self.info.types.insert(item.id, Ty::Row(cols.clone()));
        let with_key = items.len() == cols.len();
        if !with_key && items.len() + 1 != cols.len() {
            let names: Vec<String> = cols.iter().map(|(n, _)| n.clone()).collect();
            self.err(
                item.pos,
                format!(
                    "a row needs one value for each column ({}). The key ({}) can be left out when Lust can make it.",
                    names.join(", "),
                    names[0]
                ),
            );
            for i in items {
                self.expr(i, ctx, None);
            }
            return;
        }
        let wanted: Vec<(String, Ty)> = if with_key {
            cols.iter().cloned().collect()
        } else {
            cols[1..].to_vec()
        };
        for (v, (n, t)) in items.iter().zip(&wanted) {
            let got = self.expr(v, ctx, Some(t));
            self.expect_fits(v, &got, t, &format!("{n} is {}", self.a(t)));
        }
    }

    /// Checks a cMap type's key-maker: a hook that takes nothing and gives a key.
    fn key_maker(&mut self, t: &TypeRef, ty: &Ty, ctx: &mut Ctx) {
        let (Some(m), Ty::CMap(cols)) = (t.key_maker(), ty) else { return };
        let want = Ty::Hook(Rc::new(HookSig {
            ret: cols[0].1.clone(),
            params: vec![],
        }));
        let got = self.expr(m, ctx, Some(&want));
        if !self.fits(&got, &want) {
            let kt = self.show(&cols[0].1);
            self.err(
                m.pos,
                format!(
                    "a key-maker is a method or hook that takes nothing and gives back {}: {kt} {} = makeKey. This is {}.",
                    self.a(&cols[0].1),
                    cols[0].0,
                    self.a(&got)
                ),
            );
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
            Some(Ty::CMap(cols)) if keys.is_none() => {
                for i in items {
                    self.row(i, cols, ctx);
                }
                return Ty::CMap(cols.clone());
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
