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
pub const DB_RUNTIME: &str = include_str!("native_db.rs");
pub const WEB_RUNTIME: &str = include_str!("native_web.rs");
pub const NET_RUNTIME: &str = include_str!("native_net.rs");
pub const DLG_RUNTIME: &str = include_str!("native_dlg.rs");
pub const TEXT_RUNTIME: &str = include_str!("native_text.rs");
pub const UI_RUNTIME: &str = include_str!("native_ui.rs");

fn uses(info: &Info, funcs: &[&str]) -> bool {
    info.libs.values().any(|l| funcs.contains(&l.func.as_str()))
}

fn serves(info: &Info) -> bool {
    info.libs.values().any(|l| l.func == "web_serve")
}

struct W<'a> {
    info: &'a Info,
    /// Writing a library for another Rust program, not a program of its own.
    library: bool,
    /// Row structs for cMaps, by their columns.
    rows: Vec<Vec<(String, Ty)>>,
    tmp: usize,
    /// The class whose method is being written; `this` is in scope.
    class: Option<usize>,
    /// What the method or hook being written gives back.
    current_ret: Option<Ty>,
    /// A key-maker for the cMap about to be written.
    maker: Option<String>,
    /// Writing a node's starting values: what's made belongs to the node.
    owner_label: Option<String>,
    /// Inside a view: the names of the pieces new pieces go into.
    view_parent: Vec<String>,
    named_styles: std::collections::HashMap<String, Vec<StyleItem>>,
}

/// `library`: a Rust library for another Rust program to use (lust build --library), instead of a
/// program of its own.
pub fn write(prog: &Program, info: &Info, files: &[String], library: bool) -> String {
    let file = &files[0];
    // What native methods ask of the program using the library: one function each.
    let mut host_fns: Vec<String> = Vec::new();
    let mut w = W {
        info,
        library,
        rows: vec![],
        tmp: 0,
        class: None,
        current_ret: None,
        maker: None,
        owner_label: None,
        view_parent: vec![],
        named_styles: prog
            .items
            .iter()
            .filter_map(|i| match i {
                Item::Style(s) => Some((s.name.clone(), s.items.clone())),
                _ => None,
            })
            .collect(),
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
        defs.push_str(&format!(
            "#[derive(Default)]\nstruct O{root} {{\n    class: u32,\n    ended: Option<String>,\n{fields}}}\n"
        ));
        defs.push_str(&format!(
            "impl rt::HasClass for O{root} {{ fn class_id(&self) -> u32 {{ self.class }} fn ended(&self) -> Option<&str> {{ self.ended.as_deref() }} }}\nimpl rt::Endable for O{root} {{ fn end(&mut self, msg: String) {{ let class = self.class; *self = O{root} {{ class, ended: Some(msg), ..Default::default() }}; }} }}\n"
        ));
        let arms: Vec<String> = family
            .iter()
            .map(|&c| format!("ID_{c} => {:?}.to_string(),", crate::ast::shown(&info.classes[c].name)))
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
        w.owner_label = c.is_node.then(|| format!("{:?}", crate::ast::shown(&c.name)));
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
        w.owner_label = None;
        defs.push_str(&format!("fn init{ci}(this: &Rc<RefCell<O{root}>>) {{\n{init}}}\n"));
        if c.is_node {
            // Nodes open and close: opening starts them fresh, closing ends what they made.
            let name = crate::ast::shown(&c.name);
            let on_open = if c.methods.iter().any(|m| m.name == "onOpen") {
                format!("d{root}_onOpen(&o); ")
            } else {
                String::new()
            };
            let on_close = if c.methods.iter().any(|m| m.name == "onClose") {
                format!("d{root}_onClose(&o); ")
            } else {
                String::new()
            };
            let closed = format!("{name} is closed. Open it first: {name}:open();");
            let ended = format!("this was made in {name}, and ended when it closed. To hold onto something made in a node, keep it.");
            defs.push_str(&format!(
                "fn open{ci}() {{ let o = obj{ci}(); if o.borrow().ended.is_none() {{ rt::fail({:?}.into()); }} *o.borrow_mut() = O{root} {{ class: ID_{ci}, ..Default::default() }}; init{ci}(&o); {on_open}}}\n",
                format!("{name} is already open.")
            ));
            defs.push_str(&format!(
                "fn close{ci}() {{ let o = obj{ci}(); if o.borrow().ended.is_some() {{ rt::fail({:?}.into()); }} {on_close}rt::end_owned(&o, {ended:?}); rt::Endable::end(&mut *o.borrow_mut(), {closed:?}.to_string()); }}\n",
                format!("{name} is already closed.")
            ));
        }
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
            if m.native {
                let name = format!("{}_{}", snake(&c.name), snake(&m.name));
                let mut decl = vec!["&self".to_string(), format!("this: &{}", ident(&c.name))];
                let mut args = vec![format!("&{}(this.clone())", ident(&c.name))];
                for (p, (_, t)) in m.params.iter().zip(&sig.params) {
                    decl.push(format!("{}: {}", snake(&p.name), w.api_ty(t)));
                    args.push(w.to_api(&format!("v_{}", p.name), t));
                }
                host_fns.push(format!("    fn {name}({}){};\n", decl.join(", "), w.api_ret(&sig.ret)));
                let shown = format!("{}:{}", crate::ast::shown(&c.name), m.name);
                b.push_str(&w.native_body(&name, &shown, &args, &sig.ret));
            }
            w.stmts(&m.body, &mut b, 1);
            if sig.ret != Ty::Void && !m.native {
                b.push_str("    rt::unreachable()\n");
            }
            let mut all = vec![format!("this: &Rc<RefCell<O{root}>>")];
            all.extend(params);
            defs.push_str(&format!("fn m{ci}_{}({}){ret} {{\n{b}}}\n", m.name, all.join(", ")));
        }
        if let Some(v) = &c.view {
            // The window's settings, and the view: a tree of nodes Lust draws.
            w.current_ret = None;
            let s = w.styles(&v.settings);
            defs.push_str(&format!(
                "fn settings{ci}(this: &Rc<RefCell<O{root}>>) -> Vec<(String, rt::ui::SV)> {{\n    {s}\n}}\n"
            ));
            // The window's settings ride along, so a title can change with the data.
            let mut b = format!(
                "    let _depth = rt::Depth::enter();\n    let mut p0 = rt::ui::Node::new(\"view\");\n    p0.style = settings{ci}(this);\n"
            );
            w.view_parent.push("p0".into());
            w.stmts(&v.body, &mut b, 1);
            w.view_parent.pop();
            b.push_str("    p0\n");
            defs.push_str(&format!(
                "fn view{ci}(this: &Rc<RefCell<O{root}>>, actions: &mut Vec<rt::ui::Action>) -> rt::ui::Node {{\n{b}}}\n"
            ));
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

    if crate::stdlib::uses_json(info) {
        let j = w.class_json();
        defs.push_str(&j);
    }

    // Assembly recipes, each a function that makes one thing.
    for item in &prog.items {
        if let Item::Assembly(a) = item {
            w.class = None;
            for (_, _, made) in &a.recipes {
                let Ty::Class(c) = info.ty(made) else { continue };
                let root = w.root(c);
                let v = w.expr(made);
                defs.push_str(&format!(
                    "fn a_{}_{c}() -> Option<Rc<RefCell<O{root}>>> {{\n    {v}\n}}\n",
                    ident(&a.name)
                ));
            }
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
            if m.native {
                let name = snake(&m.name);
                let mut decl = vec!["&self".to_string()];
                let mut args = Vec::new();
                for (p, (_, t)) in m.params.iter().zip(&sig.params) {
                    decl.push(format!("{}: {}", snake(&p.name), w.api_ty(t)));
                    args.push(w.to_api(&format!("v_{}", p.name), t));
                }
                host_fns.push(format!("    fn {name}({}){};\n", decl.join(", "), w.api_ret(&sig.ret)));
                b.push_str(&w.native_body(&name, crate::ast::shown(&m.name), &args, &sig.ret));
            }
            w.stmts(&m.body, &mut b, 1);
            if sig.ret != Ty::Void && !m.native {
                b.push_str("    rt::unreachable()\n");
            }
            defs.push_str(&format!(
                "fn t_{}({}){} {{\n{b}}}\n",
                ident(&m.name),
                params.join(", "),
                w.ret(&sig.ret)
            ));
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
        let make = if c.is_node {
            let name = crate::ast::shown(&c.name);
            format!(
                "Rc::new(RefCell::new(O{root} {{ class: ID_{ci}, ended: Some({:?}.to_string()), ..Default::default() }}))",
                format!("{name} is closed. Open it first: {name}:open();")
            )
        } else {
            format!("new{ci}()")
        };
        objects.push_str(&format!("    {{ let o = {make}; OBJ{ci}.with(|x| *x.borrow_mut() = Some(o)); }}\n"));
    }
    // States aren't types in Rust: options are &'static str.

    w.class = None;
    // Routes: one function that finds the route for a request and runs its method.
    if !info.routes.is_empty() {
        let mut arms = String::new();
        let mut patterns: Vec<String> = Vec::new();
        for r in &info.routes {
            if !patterns.contains(&r.path) {
                patterns.push(r.path.clone());
            }
        }
        for pat in &patterns {
            let mut inner = String::new();
            for r in info.routes.iter().filter(|r| &r.path == pat) {
                let mut lets = String::new();
                let mut args = Vec::new();
                for (i, (pn, pt, from_path)) in r.params.iter().enumerate() {
                    let v = if *from_path {
                        format!("rt::web_{}(&p, {pn:?})", crate::lua::kind(pt))
                    } else {
                        format!("rt::web_body(body, &{})", w.jdec(pt))
                    };
                    lets.push_str(&format!("let a{i} = {v}; "));
                    args.push(format!("a{i}"));
                }
                let call = format!("t_{}({})", ident(&r.handler), args.join(", "));
                let run = if r.ret == Ty::Void {
                    format!("{call}; None")
                } else {
                    format!("let r = {call}; Some(({})(&r, None))", w.jenc(&r.ret))
                };
                inner.push_str(&format!(
                    "        if method == {:?} {{ return rt::web_run(&|| {{ {lets}{run} }}); }}\n        allowed.push({:?});\n",
                    r.method, r.method
                ));
            }
            arms.push_str(&format!("    if let Some(p) = rt::web_match({pat:?}, path) {{\n{inner}    }}\n"));
        }
        defs.push_str(&format!(
            "fn answer(method: &str, path: &str, body: &str) -> (u16, String) {{\n    let mut allowed: Vec<&str> = Vec::new();\n{arms}    rt::web_unanswered(method, path, &allowed)\n}}\n"
        ));
        body.push_str("    rt::web_routes(answer);\n");
    }
    // Databases open first, so everything can use their tables.
    for (i, d) in info.databases.iter().enumerate() {
        defs.push_str(&format!(
            "thread_local! {{ static DB{i}: RefCell<Option<Rc<rt::db::Db>>> = RefCell::new(None); }}\nfn db{i}() -> Rc<rt::db::Db> {{ DB{i}.with(|d| d.borrow().clone().expect(\"database\")) }}\n"
        ));
        let tables: Vec<String> = d
            .tables
            .iter()
            .map(|(tn, cols)| {
                let cs: Vec<String> = cols.iter().map(|(c, t)| format!("({c:?}, {:?})", crate::lua::kind(t))).collect();
                format!("({tn:?}, &[{}][..])", cs.join(", "))
            })
            .collect();
        body.push_str(&format!(
            "    {{ let d = rt::db::open({:?}, &[{}]); DB{i}.with(|x| *x.borrow_mut() = Some(d)); }}\n",
            d.path,
            tables.join(", ")
        ));
    }
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

    // Native methods: their bodies are in the program that uses this as a library.
    let mut host = String::new();
    if !host_fns.is_empty() && library {
        let vis = "pub ";
        host.push_str(&format!(
            "/// What this Lust code's native methods need from the program using it: implement it,\n/// then hand it over with set_host before start.\n{vis}trait Host {{\n{}}}\nthread_local! {{ static HOST: RefCell<Option<Rc<dyn Host>>> = RefCell::new(None); }}\n",
            host_fns.concat()
        ));
        if library {
            host.push_str("/// Gives the Lust code the program's side of its native methods. Call it before start.\npub fn set_host(h: impl Host + 'static) {\n    HOST.with(|x| *x.borrow_mut() = Some(Rc::new(h)));\n}\n");
        }
        host.push_str(&format!(
            "fn host() -> Rc<dyn Host> {{\n    HOST.with(|x| x.borrow().clone()).unwrap_or_else(|| rt::fail({:?}.into()))\n}}\n",
            if library {
                "a native method was called before set_host gave Lust the program's side of it."
            } else {
                "native methods only work in a library (lust build --library), used by the program that has their bodies."
            }
        ));
    }
    let files_list = files.iter().map(|f| format!("{f:?}")).collect::<Vec<_>>().join(", ");
    let ending = if library {
        let api = w.library_api(prog, &class_decls);
        format!(
            "{host}{api}\nconst FILES: &[&str] = &[{files_list}];\n\n/// Runs the Lust code's own start: makes its objects, then runs the code written outside\n/// classes and methods. Call it once, before anything else.\npub fn start() -> Result<(), String> {{\n    rt::catch(FILES, || {{ rt::started(); program() }})\n}}\n\nfn program() {{\n{body}}}\n"
        )
    } else {
        format!("{host}\nfn program() {{\n{body}}}\n\nfn main() {{\n    rt::main(program, &[{files_list}]);\n}}\n")
    };
    let what = if library { "a Rust library" } else { "a program" };
    format!(
        "// Written by Lust from {file}, as {what}. It's rewritten on every build, so don't edit it.\n#![allow(warnings)]\nuse std::cell::RefCell;\nuse std::rc::Rc;\n\nmod rt {{\n{RUNTIME}\n{db}}}\n\n{rows}{defs}{ending}",
        db = {
            let mut m = format!("pub const UI_PROGRAM: bool = {};\n", views(info));
            if !info.databases.is_empty() {
                m.push_str(&format!("pub mod db {{\nuse super::*;\nuse std::rc::Rc;\n{DB_RUNTIME}\n}}\n"));
            }
            if serves(info) {
                m.push_str(&format!("pub mod web {{\nuse super::*;\n{WEB_RUNTIME}\n}}\n"));
            }
            if uses(info, &["files_chooseOpen", "files_chooseSave", "files_chooseFolder"]) {
                m.push_str(&format!("pub mod dlg {{\nuse super::*;\n{DLG_RUNTIME}\n}}\n"));
            }
            if uses(info, &["web_get", "web_post"]) {
                m.push_str(&format!("pub mod net {{\nuse super::*;\n{NET_RUNTIME}\n}}\n"));
            }
            if views(info) {
                m.push_str(&format!("pub mod ui {{\nuse super::*;\n{UI_RUNTIME}\n}}\n"));
            }
            if uses(info, &["text_matches", "text_findAll", "text_replaceAll", "text_capture"]) {
                m.push_str(&format!("pub mod pat {{\nuse super::*;\n{TEXT_RUNTIME}\n}}\n"));
            }
            m
        }
    )
}

/// What a library with components gives the engine: values, kinds and schemas, the same kinds as
/// Solis's own (reflect::Value and FieldKind, without assets and colours), so its adapter is a
/// one-to-one match.
const COMPONENT_API: &str = r#"
/// One field's value.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Bool(bool),
    Int(i64),
    Float(f64),
    Text(String),
    /// A vec3 is three numbers, a rotation four (x, y, z, w).
    List(Vec<Value>),
    /// An entity's 64-bit ID, or none.
    Entity(Option<u64>),
}

/// What kind of value a field holds.
#[derive(Debug, Clone, PartialEq)]
pub enum FieldKind {
    Bool,
    Int,
    Float,
    Text,
    /// One of a state's options, by name.
    Choice(Vec<String>),
    Vec3,
    Rotation,
    Entity,
}

/// One field of a component.
#[derive(Debug, Clone, PartialEq)]
pub struct FieldInfo {
    pub name: String,
    pub kind: FieldKind,
    pub default: Value,
}

/// A component's name and fields, in the order they're written.
#[derive(Debug, Clone, PartialEq)]
pub struct Schema {
    pub name: String,
    pub fields: Vec<FieldInfo>,
}

/// A component's values, by field name.
pub type Data = std::collections::BTreeMap<String, Value>;

fn as_bool(v: &Value) -> Option<bool> {
    match v {
        Value::Bool(b) => Some(*b),
        _ => None,
    }
}
fn as_int(v: &Value) -> Option<i64> {
    match v {
        Value::Int(n) => Some(*n),
        _ => None,
    }
}
fn as_num(v: &Value) -> Option<f64> {
    match v {
        Value::Float(x) => Some(*x),
        Value::Int(n) => Some(*n as f64),
        _ => None,
    }
}
fn as_text(v: &Value) -> Option<String> {
    match v {
        Value::Text(t) => Some(t.clone()),
        _ => None,
    }
}
fn as_choice(v: &Value, options: &[&'static str]) -> Option<&'static str> {
    match v {
        Value::Text(t) => options.iter().copied().find(|o| o == t),
        _ => None,
    }
}
fn as_floats(v: &Value, n: usize) -> Option<Vec<f64>> {
    match v {
        Value::List(items) if items.len() == n => items.iter().map(as_num).collect(),
        _ => None,
    }
}
fn as_vec3(v: &Value) -> Option<Vec3> {
    as_floats(v, 3).map(|f| rt::vec3(f[0], f[1], f[2]))
}
fn as_rotation(v: &Value) -> Option<Rotation> {
    as_floats(v, 4).map(|f| Rotation { x: f[0] as f32, y: f[1] as f32, z: f[2] as f32, w: f[3] as f32 })
}
fn as_entity(v: &Value) -> Option<u64> {
    match v {
        Value::Entity(e) => Some(e.unwrap_or(0)),
        _ => None,
    }
}
"#;

/// What a native method says when it's run without a program to give its body.
pub fn native_alone(shown: &str) -> String {
    format!("{shown} is native: its body is in the Rust program that uses this Lust as a library (lust build --library), so it can't run on its own.")
}

/// A Lust name as Rust writes names of functions and fields: moveTo becomes move_to.
fn snake(name: &str) -> String {
    let mut out = String::new();
    for (i, ch) in crate::ast::shown(name).chars().enumerate() {
        if ch.is_ascii_uppercase() {
            if i > 0 {
                out.push('_');
            }
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push(ch);
        }
    }
    const KEYWORDS: &[&str] = &[
        "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum", "extern", "false", "fn", "for", "if", "impl",
        "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref", "return", "static", "struct", "super", "trait", "true", "type",
        "unsafe", "use", "where", "while", "box", "final", "macro", "override", "priv", "typeof", "unsized", "virtual", "yield", "try",
    ];
    if KEYWORDS.contains(&out.as_str()) {
        format!("r#{out}")
    } else {
        out
    }
}

/// Whether the program has anything to do with screens.
fn views(info: &Info) -> bool {
    !info.view_opens.is_empty() || info.classes.iter().any(|c| c.has_view)
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
        // Things made in a hook may belong to the object's state.
        if let ExprKind::New { .. } = e.kind {
            found = true;
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
            Stmt::Keep { value, into, .. } => {
                walk_expr(value, f);
                if let Some(i) = into {
                    walk_expr(i, f);
                }
            }
            Stmt::Element {
                arg,
                style,
                children,
                action,
                enter,
                choices,
                ..
            } => {
                for a in [arg, choices].into_iter().flatten() {
                    walk_expr(a, f);
                }
                walk_styles(style, f);
                walk_stmts(children, f);
                if let Some(a) = enter {
                    walk_stmts(a, f);
                }
                if let Some(a) = action {
                    walk_stmts(a, f);
                }
            }
            Stmt::Try { body, handler, .. } => {
                walk_stmts(body, f);
                walk_stmts(handler, f);
            }
            _ => {}
        }
    }
}

fn walk_styles(items: &[StyleItem], f: &mut dyn FnMut(&Expr)) {
    for it in items {
        match &it.value {
            StyleValue::Expr(e) => walk_expr(e, f),
            StyleValue::Nested(n) => walk_styles(n, f),
            _ => {}
        }
    }
}

pub fn walk_expr(e: &Expr, f: &mut dyn FnMut(&Expr)) {
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
        ExprKind::Index(a, b) | ExprKind::Bin(_, a, b, _) | ExprKind::Random(a, b) => {
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
            Ty::Vec3 => "rt::Vec3".into(),
            Ty::Rot => "rt::Rotation".into(),
            // An entity's 64-bit ID; nothing is 0, which Solis never uses.
            Ty::Entity => "u64".into(),
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
            Ty::Table(_) | Ty::Query(_) => "rt::db::Q".into(),
            Ty::TableRow(_) => "rt::db::RowRef".into(),
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

    // ----- a library's side that the program using it sees -----

    /// The Rust the program using a library sees. Every class gets a type of its own (so native
    /// methods can name them), and what the first file passes gets made, read, changed and called.
    /// Each call gives back Lust's failure as text instead of stopping the program.
    fn library_api(&mut self, prog: &Program, class_decls: &[&ClassDecl]) -> String {
        let info = self.info;
        let exported = |n: &str| prog.passes.iter().any(|(p, _)| p == n);
        fn plain(t: &Ty) -> bool {
            match t {
                Ty::Array(e) => plain(e) && !matches!(**e, Ty::Array(_)),
                _ => matches!(
                    t,
                    Ty::Int | Ty::Float | Ty::Str | Ty::Truth | Ty::Vec3 | Ty::Rot | Ty::Entity | Ty::Class(_)
                ),
            }
        }
        // Vectors and rotations are the runtime's own types, handed out as they are.
        let mut out = String::from("\npub use rt::{Rotation, Vec3};\n");
        for (ci, c) in info.classes.iter().enumerate() {
            let root = self.root(ci);
            let w = ident(&c.name);
            let shown = crate::ast::shown(&c.name).to_string();
            let an = if shown.starts_with(['A', 'E', 'I', 'O', 'U']) { "An" } else { "A" };
            out.push_str(&format!(
                "\n/// {an} {shown} from Lust.\n#[derive(Clone)]\npub struct {w}(Rc<RefCell<O{root}>>);\n"
            ));
            // Classes with native methods belong to the program using the library, so it always
            // sees all of them. Otherwise, what the first file passes.
            let owned = class_decls.iter().any(|d| d.name == c.name && d.methods.iter().any(|m| m.native)) || c.is_component;
            if !exported(&c.name) && !owned {
                continue;
            }
            let mut items = String::new();
            if c.is_object {
                items.push_str(&format!(
                    "    /// The one {shown}. start must have run first.\n    pub fn get() -> {w} {{\n        {w}(obj{ci}())\n    }}\n"
                ));
            } else if !c.is_node && !class_decls.iter().any(|d| d.name == c.name && d.is_schema) {
                items.push_str(&format!(
                    "    /// A new {shown}, with its fields' starting values.\n    pub fn new() -> Result<{w}, String> {{\n        rt::catch(FILES, || {w}(new{ci}()))\n    }}\n"
                ));
            }
            let mut seen: Vec<String> = Vec::new();
            let mut methods: Vec<(usize, String, crate::check::MethodSig)> = Vec::new();
            let mut cur = Some(ci);
            while let Some(k) = cur {
                let mut names: Vec<&String> = info.classes[k].methods.keys().collect();
                names.sort();
                for m in names {
                    if !methods.iter().any(|(_, n, _)| n == m) {
                        methods.push((k, m.clone(), info.classes[k].methods[m].clone()));
                    }
                }
                cur = info.classes[k].parent;
            }
            let mut cur = Some(ci);
            while let Some(k) = cur {
                for f in &info.classes[k].fields {
                    if !plain(&f.ty) || seen.contains(&f.name) || methods.iter().any(|(_, n, _)| *n == f.name) {
                        continue;
                    }
                    seen.push(f.name.clone());
                    let field = self.fname(k, &f.name);
                    let ty = self.api_ty(&f.ty);
                    let get = snake(&f.name);
                    let set = snake(&format!("set{}{}", f.name[..1].to_uppercase(), &f.name[1..]));
                    items.push_str(&format!(
                        "    pub fn {get}(&self) -> Result<{ty}, String> {{\n        rt::catch(FILES, || {{ let v = self.0.borrow().{field}.clone(); {} }})\n    }}\n",
                        self.to_api("v", &f.ty)
                    ));
                    items.push_str(&format!(
                        "    pub fn {set}(&self, v: {ty}) -> Result<(), String> {{\n        rt::catch(FILES, || {{ let v = {}; self.0.borrow_mut().{field} = v; }})\n    }}\n",
                        self.from_api("v", &f.ty)
                    ));
                }
                cur = info.classes[k].parent;
            }
            for (_, m, sig) in &methods {
                if !sig.params.iter().all(|(_, t)| plain(t)) || !(sig.ret == Ty::Void || plain(&sig.ret)) {
                    continue;
                }
                let mut decl = vec!["&self".to_string()];
                let mut args = vec!["&self.0".to_string()];
                for (p, t) in &sig.params {
                    decl.push(format!("{}: {}", snake(p), self.api_ty(t)));
                    args.push(self.from_api(&snake(p), t));
                }
                let ret = self.api_ty(&sig.ret);
                items.push_str(&format!(
                    "    pub fn {}({}) -> Result<{ret}, String> {{\n        rt::catch(FILES, || {{ let out = d{root}_{m}({}); {} }})\n    }}\n",
                    snake(m),
                    decl.join(", "),
                    args.join(", "),
                    self.to_api("out", &sig.ret)
                ));
            }
            if c.is_component {
                items.push_str(&self.component_api(ci, &shown, &w));
            }
            out.push_str(&format!("impl {w} {{\n{items}}}\n"));
        }
        // Components: what the engine needs to keep, show and run them.
        let comps: Vec<String> = info.classes.iter().filter(|c| c.is_component).map(|c| ident(&c.name)).collect();
        if !comps.is_empty() {
            out.push_str(COMPONENT_API);
            out.push_str(&format!(
                "\n/// Every component's schema, for the inspector.\npub fn components() -> Vec<Schema> {{\n    vec![{}]\n}}\n",
                comps.iter().map(|c| format!("{c}::schema()")).collect::<Vec<_>>().join(", ")
            ));
        }
        for item in &prog.items {
            let Item::Method(m) = item else { continue };
            if !exported(&m.name) {
                continue;
            }
            let sig = info.methods[&m.name].clone();
            if !sig.params.iter().all(|(_, t)| plain(t)) || !(sig.ret == Ty::Void || plain(&sig.ret)) {
                continue;
            }
            let mut decl = Vec::new();
            let mut args = Vec::new();
            for (p, t) in &sig.params {
                decl.push(format!("{}: {}", snake(p), self.api_ty(t)));
                args.push(self.from_api(&snake(p), t));
            }
            let ret = self.api_ty(&sig.ret);
            out.push_str(&format!(
                "\npub fn {}({}) -> Result<{ret}, String> {{\n    rt::catch(FILES, || {{ let out = t_{}({}); {} }})\n}}\n",
                snake(&m.name),
                decl.join(", "),
                ident(&m.name),
                args.join(", "),
                self.to_api("out", &sig.ret)
            ));
        }
        out
    }

    /// A type as the program using a library sees it: plain Rust, and classes by name.
    fn api_ty(&mut self, t: &Ty) -> String {
        match t {
            Ty::Int => "i64".into(),
            Ty::Float => "f64".into(),
            Ty::Str => "String".into(),
            Ty::Truth => "bool".into(),
            Ty::Class(c) => format!("Option<{}>", ident(&self.info.classes[*c].name)),
            Ty::Vec3 => "Vec3".into(),
            Ty::Rot => "Rotation".into(),
            Ty::Entity => "Option<u64>".into(),
            Ty::Array(e) => format!("Vec<{}>", self.api_ty(e)),
            Ty::Void => "()".into(),
            other => self.ty(other),
        }
    }

    fn api_ret(&mut self, t: &Ty) -> String {
        if *t == Ty::Void {
            String::new()
        } else {
            format!(" -> {}", self.api_ty(t))
        }
    }

    /// A component's name, schema, and its fields as values, both ways.
    fn component_api(&mut self, ci: usize, shown: &str, w: &str) -> String {
        let fields: Vec<(String, Ty)> = self.info.classes[ci]
            .fields
            .iter()
            .filter(|f| f.name != "me")
            .map(|f| (f.name.clone(), f.ty.clone()))
            .collect();
        let mut kinds = Vec::new();
        let mut out_vals = Vec::new();
        let mut in_vals = Vec::new();
        for (n, t) in &fields {
            let f = self.fname(ci, n);
            let (kind, to, from) = match t {
                Ty::Truth => ("FieldKind::Bool".to_string(), format!("Value::Bool(o.{f})"), "as_bool(v)".to_string()),
                Ty::Int => ("FieldKind::Int".into(), format!("Value::Int(o.{f})"), "as_int(v)".into()),
                Ty::Float => ("FieldKind::Float".into(), format!("Value::Float(o.{f})"), "as_num(v)".into()),
                Ty::Str => ("FieldKind::Text".into(), format!("Value::Text(o.{f}.clone())"), "as_text(v)".into()),
                Ty::State(s) => {
                    let opts: Vec<String> = self.info.states[*s].options.iter().map(|o| format!("{o:?}")).collect();
                    (
                        format!("FieldKind::Choice(vec![{}])", opts.iter().map(|o| format!("{o}.to_string()")).collect::<Vec<_>>().join(", ")),
                        format!("Value::Text(o.{f}.to_string())"),
                        format!("as_choice(v, &[{}])", opts.join(", ")),
                    )
                }
                Ty::Vec3 => (
                    "FieldKind::Vec3".into(),
                    format!("Value::List(vec![Value::Float(o.{f}.x as f64), Value::Float(o.{f}.y as f64), Value::Float(o.{f}.z as f64)])"),
                    "as_vec3(v)".into(),
                ),
                Ty::Rot => (
                    "FieldKind::Rotation".into(),
                    format!("Value::List(vec![Value::Float(o.{f}.x as f64), Value::Float(o.{f}.y as f64), Value::Float(o.{f}.z as f64), Value::Float(o.{f}.w as f64)])"),
                    "as_rotation(v)".into(),
                ),
                Ty::Entity => ("FieldKind::Entity".into(), format!("Value::Entity(Some(o.{f}).filter(|e| *e != 0))"), "as_entity(v)".into()),
                _ => continue,
            };
            kinds.push(format!(
                "FieldInfo {{ name: {n:?}.into(), kind: {kind}, default: d[{n:?}].clone() }}"
            ));
            out_vals.push(format!(
                "        d.insert({n:?}.into(), {to});
"
            ));
            in_vals.push(format!(
                "        if let Some(x) = data.get({n:?}).and_then(|v| {from}) {{
            self.0.borrow_mut().{f} = x;
        }}
"
            ));
        }
        format!(
            "    /// The component's name, as the engine knows it.\n    pub const NAME: &'static str = {shown:?};\n\n    /// Its fields, their kinds and their starting values, in order.\n    pub fn schema() -> Schema {{\n        let d = {w}::new().map(|c| c.to_data()).unwrap_or_default();\n        Schema {{ name: {shown:?}.into(), fields: vec![{}] }}\n    }}\n\n    /// Its fields' values, by name.\n    pub fn to_data(&self) -> Data {{\n        let o = self.0.borrow();\n        let mut d = Data::new();\n{}        d\n    }}\n\n    /// Sets the fields `data` has. A missing value, or one of the wrong kind, is left as it is.\n    pub fn set_data(&self, data: &Data) {{\n{}    }}\n\n    /// A new one with these values, and the starting values for the rest.\n    pub fn from_data(data: &Data) -> Result<{w}, String> {{\n        let c = {w}::new()?;\n        c.set_data(data);\n        Ok(c)\n    }}\n",
            kinds.join(", "),
            out_vals.concat(),
            in_vals.concat()
        )
    }

    /// A Lust value handed out: things made from classes get their class's own Rust type.
    fn to_api(&self, code: &str, t: &Ty) -> String {
        match t {
            Ty::Class(c) => format!("{code}.clone().map({})", ident(&self.info.classes[*c].name)),
            // An array goes out as a Vec of its items, copied.
            Ty::Array(e) => format!(
                "rt::acopy(&{code}).into_iter().map(|x| {}).collect::<Vec<_>>()",
                self.to_api("x", e)
            ),
            Ty::Entity => format!("Some({code}).filter(|e| *e != 0)"),
            _ => code.to_string(),
        }
    }

    /// A value coming in from the program using the library.
    fn from_api(&self, code: &str, t: &Ty) -> String {
        match t {
            Ty::Class(_) => format!("{code}.map(|w| w.0)"),
            Ty::Array(e) => format!("rt::arr({code}.into_iter().map(|x| {}).collect())", self.from_api("x", e)),
            Ty::Entity => format!("{code}.unwrap_or(0)"),
            _ => code.to_string(),
        }
    }

    /// A native method's body: the program using the library does the work. A program of its own
    /// has no such program, so it stops with the same message lust run gives.
    fn native_body(&self, name: &str, shown: &str, args: &[String], ret: &Ty) -> String {
        if !self.library {
            return format!("    rt::fail({:?}.into());\n", native_alone(shown));
        }
        let call = format!("host().{name}({})", args.join(", "));
        if *ret == Ty::Void {
            format!("    {call};\n")
        } else {
            format!("    let r = {call};\n    return {};\n", self.from_api("r", ret))
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
                        // A table or query goes through a copy of its rows, as a cMap.
                        let st = match self.info.ty(source) {
                            Ty::Table(t) | Ty::Query(t) => Ty::CMap(t.cols.clone()),
                            x => x,
                        };
                        match st {
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
            Stmt::Element {
                kind,
                arg,
                style,
                children,
                action,
                enter,
                choices,
                ..
            } => {
                let parent = self.view_parent.last().cloned().unwrap_or_else(|| "p0".into());
                if let Some(Expr {
                    kind: ExprKind::New { class, inits, .. },
                    id,
                    ..
                }) = arg.as_ref().filter(|_| kind == "piece")
                {
                    // A piece: the same one each time the view is drawn, given its values again.
                    let Some(&ci) = self.info.class_index.get(class) else { return };
                    let v = format!("p{}", self.fresh());
                    let key = match self.info.piece_keys.get(id) {
                        Some(keys) => format!(
                            "Some(format!(\"{}\", {}))",
                            vec!["{:?}"; keys.len()].join("|"),
                            keys.iter().map(|k| format!("v_{k}")).collect::<Vec<_>>().join(", ")
                        ),
                        None => "None".into(),
                    };
                    out.push_str(&format!(
                        "{pad}{{\n{pad}    let pc = rt::ui::piece(Rc::as_ptr(this) as *const () as usize, {id}, {key}, new{ci});\n"
                    ));
                    for (f, _, val) in inits {
                        let want = self.info.field(ci, f).map(|x| x.ty.clone()).unwrap_or(Ty::Error);
                        let vc = self.coerced(val, &want);
                        let fname = self.field_of(ci, f);
                        out.push_str(&format!("{pad}    {{ let v = {vc}; pc.borrow_mut().{fname} = v; }}\n"));
                    }
                    out.push_str(&format!("{pad}    let mut {v} = view{ci}(&pc, actions);\n"));
                    out.push_str(&format!(
                        "{pad}    {v}.kind = \"piece\".to_string();\n{pad}    {v}.text = {:?}.to_string();\n",
                        crate::ast::shown(class)
                    ));
                    let s = self.styles(style);
                    out.push_str(&format!("{pad}    {v}.style = {s};\n"));
                    out.push_str(&format!("{pad}    {parent}.children.push({v});\n{pad}}}\n"));
                    return;
                }
                let v = format!("p{}", self.fresh());
                out.push_str(&format!("{pad}{{\n{pad}    let mut {v} = rt::ui::Node::new({kind:?});\n"));
                let bound = matches!(
                    kind.as_str(),
                    "input" | "textarea" | "checkbox" | "slider" | "dropdown" | "dialog" | "tabs"
                );
                match arg {
                    Some(a) if kind == "every" || kind == "progress" => {
                        let secs = self.coerced(a, &Ty::Float);
                        out.push_str(&format!("{pad}    {v}.value = Some(rt::ui::SV::Num({secs}));\n"));
                    }
                    Some(a) if bound => {
                        // What the input shows: the value it's bound to, as it is now.
                        let shown = self.expr(a);
                        if kind == "dropdown" {
                            // What it chooses from: its list, or its state's options.
                            let opts = match (self.info.ty(a), choices) {
                                (Ty::State(s), _) => {
                                    let o: Vec<String> = self.info.states[s].options.iter().map(|o| format!("{o:?}.to_string()")).collect();
                                    format!("vec![{}]", o.join(", "))
                                }
                                (_, Some(c)) => format!("rt::acopy(&{})", self.expr(c)),
                                _ => "vec![]".into(),
                            };
                            out.push_str(&format!("{pad}    {v}.options = {opts};\n"));
                        }
                        let sv = match self.info.ty(a) {
                            Ty::State(_) => format!("rt::ui::SV::Text({shown}.to_string())"),
                            Ty::Str => format!("rt::ui::SV::Text({shown})"),
                            Ty::Truth => format!("rt::ui::SV::Truth({shown})"),
                            _ => format!("rt::ui::SV::Num(({shown}) as f64)"),
                        };
                        out.push_str(&format!("{pad}    {v}.value = Some({sv});\n"));
                    }
                    Some(a) => {
                        let t = self.coerced(a, &Ty::Str);
                        out.push_str(&format!("{pad}    {v}.text = {t};\n"));
                    }
                    None => {}
                }
                if !style.is_empty() {
                    let s = self.styles(style);
                    out.push_str(&format!("{pad}    {v}.style = {s};\n"));
                }
                self.view_parent.push(v.clone());
                self.stmts(children, out, depth + 1);
                self.view_parent.pop();
                if let Some(a) = action {
                    let given = if bound { arg.as_ref() } else { None };
                    self.view_action(a, given, &format!("{v}.action"), out, depth);
                }
                if let Some(a) = enter {
                    self.view_action(a, None, &format!("{v}.enter"), out, depth);
                }
                out.push_str(&format!("{pad}    {parent}.children.push({v});\n{pad}}}\n"));
            }
            Stmt::SaveDecl { .. } => {}
            Stmt::Save(name, _) => {
                let list = self.info.saves.get(name).cloned().unwrap_or_default();
                let mut parts = Vec::new();
                for (n, item) in &list {
                    let (t, v) = match item {
                        crate::check::SaveItem::Var(t) => (t.clone(), format!("v_{n}")),
                        crate::check::SaveItem::Object(c) => (Ty::Class(*c), format!("Some(obj{c}())")),
                    };
                    let enc = self.jenc(&t);
                    parts.push(format!("({:?}, ({enc})(&{v}, Some(1)))", crate::ast::shown(n)));
                }
                out.push_str(&format!(
                    "{pad}rt::save_write({name:?}, rt::jw_fields(vec![{}], Some(0)));\n",
                    parts.join(", ")
                ));
            }
            Stmt::Load(name, _) => {
                let list = self.info.saves.get(name).cloned().unwrap_or_default();
                out.push_str(&format!("{pad}{{\n{pad}    let d = rt::save_read({name:?});\n"));
                // Everything is read first, so a save that doesn't fit changes nothing.
                for (i, (n, item)) in list.iter().enumerate() {
                    let t = match item {
                        crate::check::SaveItem::Var(t) => t.clone(),
                        crate::check::SaveItem::Object(c) => Ty::Class(*c),
                    };
                    let dec = self.jdec(&t);
                    out.push_str(&format!(
                        "{pad}    let t{i} = rt::save_get(&d, {name:?}, {:?}, &{dec});\n",
                        crate::ast::shown(n)
                    ));
                }
                for (i, (n, item)) in list.iter().enumerate() {
                    match item {
                        crate::check::SaveItem::Var(_) => out.push_str(&format!("{pad}    v_{n} = t{i};\n")),
                        crate::check::SaveItem::Object(c) => {
                            let mut copy = String::new();
                            for (decl, f, _) in crate::stdlib::json_fields(self.info, *c) {
                                let fname = self.fname(decl, &f);
                                copy.push_str(&format!("{{ let x = t.borrow().{fname}.clone(); o.borrow_mut().{fname} = x; }} "));
                            }
                            out.push_str(&format!("{pad}    {{ let t = rt::obj(&t{i}); let o = obj{c}(); {copy}}}\n"));
                        }
                    }
                }
                out.push_str(&format!("{pad}}}\n"));
            }
            Stmt::Keep { value, into, .. } => {
                let v = self.expr(value);
                out.push_str(&format!("{pad}{{\n{pad}    let k = {v};\n{pad}    rt::keep(&k);\n"));
                if let Some(into) = into {
                    let inner = format!("{pad}    ");
                    if matches!(self.info.ty(into), Ty::Array(_)) {
                        let t = self.expr(into);
                        out.push_str(&format!("{inner}rt::ainject(&{t}, k);\n"));
                    } else {
                        self.assign(into, "k".into(), out, &inner);
                    }
                }
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
        if let Some(owner) = self.info.state_owner(target, self.class) {
            // An object's own state: things made in the old one may end.
            let (o, c) = match owner {
                Some(t) => {
                    let Ty::Class(c) = self.info.ty(t) else { return };
                    (format!("rt::obj(&{})", self.expr(t)), c)
                }
                None => ("this.clone()".to_string(), self.class.unwrap()),
            };
            let sf = self.field_of(c, "state");
            out.push_str(&format!(
                "{pad}{{ let v = {v}; let o = {o}; o.borrow_mut().{sf} = v; rt::set_state(&o, v); }}\n"
            ));
            return;
        }
        match &target.kind {
            ExprKind::Name(n) => match self.info.names.get(&target.id) {
                Some(NameRes::Field) => {
                    let c = self.class.unwrap();
                    let f = self.field_of(c, n);
                    out.push_str(&format!("{pad}{{ let v = {v}; this.borrow_mut().{f} = v; }}\n"));
                }
                _ => out.push_str(&format!("{pad}v_{n} = {v};\n")),
            },
            // pos.x = 3: a vec3 is given a new one with the new x.
            ExprKind::Field(t, f, _) if self.info.ty(t) == Ty::Vec3 => {
                let tc = self.expr(t);
                let i = match f.as_str() {
                    "x" => 0,
                    "y" => 1,
                    _ => 2,
                };
                let t = t.clone();
                self.assign(&t, format!("rt::vwith({tc}, {i}, {v})"), out, pad);
            }
            ExprKind::Field(t, f, _) => {
                let tc = self.expr(t);
                match self.info.ty(t) {
                    Ty::Row(_) => out.push_str(&format!("{pad}{{ let v = {v}; let o = {tc}; o.borrow_mut().f_{f} = v; }}\n")),
                    Ty::TableRow(tt) => {
                        let ct = tt.cols.iter().find(|(c, _)| c == f).map(|x| x.1.clone()).unwrap_or(Ty::Error);
                        let k = crate::lua::kind(&ct);
                        out.push_str(&format!(
                            "{pad}{{ let v = {v}; rt::db::r_set(&{tc}, {f:?}, rt::db::sv_{k}(v)); }}\n"
                        ));
                    }
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

    /// An expression as a value. A table or a query used as a value is a copy of its rows, as a cMap.
    fn expr(&mut self, e: &Expr) -> String {
        let c = self.raw(e);
        match self.info.ty(e) {
            Ty::Table(t) | Ty::Query(t) => {
                let dec = self.row_decoder(&t);
                format!("rt::db::q_cmap(&{c}, &{dec})")
            }
            _ => c,
        }
    }

    /// A closure turning a table's row, as the database gives it, into a cMap row.
    fn row_decoder(&mut self, t: &crate::types::TableTy) -> String {
        let ri = self.row_index(&t.cols);
        let kt = self.ty(&t.cols[0].1);
        let fields: Vec<String> = t
            .cols
            .iter()
            .enumerate()
            .map(|(i, (n, ct))| format!("f_{n}: {}", self.from_sql(ct, &format!("&r[{i}]"))))
            .collect();
        format!(
            "(|r: &[rt::db::SqlVal]| -> ({kt}, Row{ri}) {{ let row = Row{ri} {{ {} }}; (row.f_{}.clone(), row) }})",
            fields.join(", "),
            t.cols[0].0
        )
    }

    /// A value from the database, as the Lust type.
    fn from_sql(&self, t: &Ty, v: &str) -> String {
        match t {
            Ty::State(s) => {
                let opts: Vec<String> = self.info.states[*s].options.iter().map(|o| format!("{o:?}")).collect();
                format!("rt::db::v_state({v}, &[{}])", opts.join(", "))
            }
            other => format!("rt::db::v_{}({v})", crate::lua::kind(other)),
        }
    }

    /// A Lust value going into the database.
    fn to_sql(&mut self, e: &Expr, want: &Ty) -> String {
        let c = self.coerced(e, want);
        let k = match want {
            Ty::Error | Ty::Nothing | Ty::Void => crate::lua::kind(&self.info.ty(e)),
            w => crate::lua::kind(w),
        };
        format!("rt::db::sv_{k}({c})")
    }

    /// Where a table or query is wanted as itself, to ask the database.
    fn raw(&mut self, e: &Expr) -> String {
        if let Some(op) = self.info.table_ops.get(&e.id).copied() {
            return self.table_op(e, op);
        }
        if let ExprKind::Saved(name) = &e.kind {
            return format!("rt::save_exists({name:?})");
        }
        if let Some(lib) = self.info.libs.get(&e.id) {
            return self.lib(e, lib);
        }
        if let Some(c) = self.info.view_opens.get(&e.id) {
            return format!(
                "{{ let o = new{c}(); let s = settings{c}(&o); let mut build = || -> Result<(rt::ui::Node, Vec<rt::ui::Action>), String> {{ let mut actions: Vec<rt::ui::Action> = Vec::new(); let mut out = None; if let Some(e) = rt::attempt(|| out = Some(view{c}(&o, &mut actions))) {{ return Err(e); }} Ok((out.unwrap(), actions)) }}; if let Err(e) = rt::ui::show(&s, &mut build) {{ rt::fail(e); }} }}"
            );
        }
        if let Some(op) = self.info.node_ops.get(&e.id) {
            let ExprKind::Method(t, ..) = &e.kind else { return "()".into() };
            let Ty::Class(c) = self.info.ty(t) else { return "()".into() };
            return match *op {
                "open" => format!("open{c}()"),
                "close" => format!("close{c}()"),
                _ => format!("obj{c}().borrow().ended.is_none()"),
            };
        }
        if let Some((asm, c)) = self.info.creates.get(&e.id) {
            let f = format!("a_{}_{c}()", ident(asm));
            return self.owned(f);
        }
        match &e.kind {
            ExprKind::Int(v) => format!("({v}i64)"),
            ExprKind::Float(v) => format!("({v:?}f64)"),
            ExprKind::Str(s) => format!("{s:?}.to_string()"),
            ExprKind::Bool(b) => b.to_string(),
            ExprKind::Nothing if self.info.ty(e) == Ty::Entity => "0u64".into(),
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
                        format!(
                            "{{ let f: Rc<dyn Fn({types}) -> {r}> = Rc::new(move |{params}| t_{}({args})); Some(f) }}",
                            ident(n)
                        )
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
            ExprKind::Field(t, f, _) if self.info.ty(t) == Ty::Vec3 => format!("({}.{f} as f64)", self.expr(t)),
            ExprKind::Field(t, f, _) => {
                if let Some((_, path)) = self.info.options.get(&e.id) {
                    return format!("{path:?}");
                }
                if let (Ty::Table(tt), ExprKind::Name(_)) = (self.info.ty(e), &t.kind) {
                    let cols: Vec<String> = tt.cols.iter().map(|(c, _)| format!("{c:?}")).collect();
                    return format!(
                        "rt::db::table(db{}(), {f:?}, {:?}, &[{}], {})",
                        tt.db,
                        tt.name,
                        cols.join(", "),
                        tt.cols[0].1 == Ty::Str
                    );
                }
                if let Ty::TableRow(tt) = self.info.ty(t) {
                    let r = self.expr(t);
                    let ct = tt.cols.iter().find(|(c, _)| c == f).map(|x| x.1.clone()).unwrap_or(Ty::Error);
                    return self.from_sql(&ct, &format!("&rt::db::r_get(&{r}, {f:?})"));
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
                        (Ty::Vec3 | Ty::Rot, _) => {
                            let p = if tt == Ty::Vec3 { "v" } else { "q" };
                            let mut a = vec![tc];
                            for x in args {
                                let want = if self.info.ty(x).is_number() { Ty::Float } else { self.info.ty(x) };
                                a.push(self.coerced(x, &want));
                            }
                            format!("rt::{p}{m}({})", a.join(", "))
                        }
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
                        (Ty::CMap(_), "reverse") => format!("rt::creverse(&{tc})"),
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
                    format!("t_{}({a})", ident(name))
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
            ExprKind::Index(t, i) if matches!(self.info.ty(t), Ty::Table(_)) => {
                let Ty::Table(tt) = self.info.ty(t) else { return "()".into() };
                let tc = self.raw(t);
                let k = self.to_sql(i, &tt.cols[0].1);
                format!("rt::db::t_get(&{tc}, {k})")
            }
            ExprKind::Index(t, i) => {
                let tc = self.expr(t);
                let ic = self.expr(i);
                match self.info.ty(t) {
                    Ty::Map(_) => format!("rt::mget(&{tc}, &{ic})"),
                    Ty::CMap(_) => format!("rt::cget(&{tc}, {ic})"),
                    _ => format!("rt::aget(&{tc}, {ic})"),
                }
            }
            ExprKind::Len(inner) if matches!(self.info.ty(inner), Ty::Table(_) | Ty::Query(_)) => {
                format!("rt::db::q_count(&{})", self.raw(inner))
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
            ExprKind::Neg(inner) if self.info.ty(inner) == Ty::Vec3 => format!("rt::vneg({})", self.expr(inner)),
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
                t @ (Ty::Vec3 | Ty::Rot) => {
                    let vs: Vec<String> = items.iter().map(|i| self.coerced(i, &Ty::Float)).collect();
                    format!("rt::{}({})", if t == Ty::Vec3 { "vec3" } else { "rot" }, vs.join(", "))
                }
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
                self.owned(s)
            }
            ExprKind::Saved(_) => "false".into(),
            ExprKind::Random(lo, hi) => {
                if self.info.ty(e) == Ty::Float {
                    format!("rt::frandom({}, {})", self.coerced(lo, &Ty::Float), self.coerced(hi, &Ty::Float))
                } else {
                    format!("rt::random({}, {})", self.expr(lo), self.expr(hi))
                }
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
                    Conv::Upper => format!("rt::text_upper({c})"),
                    Conv::Lower => format!("rt::text_lower({c})"),
                    Conv::Trim => format!("rt::text_trim({c})"),
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

    /// Calls on tables and queries.
    fn table_op(&mut self, e: &Expr, op: &str) -> String {
        let ExprKind::Method(t, _, args, _) = &e.kind else {
            return "()".into();
        };
        if op == "sortBy" {
            let col = self.info.columns.get(&args[0].id).cloned().unwrap_or_default();
            let c = self.expr(t);
            return match self.info.ty(t) {
                Ty::CMap(cols) => {
                    let ri = self.row_index(&cols);
                    format!("rt::csort(&{c}, |r: &Row{ri}| r.f_{col}.clone())")
                }
                Ty::Array(el) => {
                    let Ty::Class(k) = *el else { return "()".into() };
                    let root = self.root(k);
                    let f = self.field_of(k, &col);
                    format!("rt::asort_by(&{c}, |x: &Option<Rc<RefCell<O{root}>>>| {{ let o = rt::obj(x); let v = o.borrow().{f}.clone(); v }})")
                }
                _ => "()".into(),
            };
        }
        let (Ty::Table(tt) | Ty::Query(tt)) = self.info.ty(t) else {
            return "()".into();
        };
        let q = self.raw(t);
        match op {
            "inject" => {
                let ExprKind::List { items, .. } = &args[0].kind else {
                    return "()".into();
                };
                let cols: Vec<Ty> = tt.cols.iter().map(|c| c.1.clone()).collect();
                let cols = if items.len() == cols.len() { cols } else { cols[1..].to_vec() };
                let vals: Vec<String> = items.iter().zip(&cols).map(|(x, t)| self.to_sql(x, t)).collect();
                format!("rt::db::t_inject(&{q}, vec![{}])", vals.join(", "))
            }
            "has" => format!("rt::db::t_has(&{q}, {})", self.to_sql(&args[0], &tt.cols[0].1)),
            "removeKey" => format!("rt::db::t_remove(&{q}, {})", self.to_sql(&args[0], &tt.cols[0].1)),
            "removeAll" => format!("rt::db::q_delete(&{q})"),
            "reverse" => format!("rt::db::q_reverse(&{q})"),
            "count" => format!("rt::db::q_count(&{q})"),
            "first" => format!("rt::db::q_first(&{q}, {})", self.expr(&args[0])),
            "orderBy" => {
                let col = self.info.columns.get(&args[0].id).cloned().unwrap_or_default();
                format!("rt::db::q_order(&{q}, {col:?})")
            }
            _ => {
                let mut params = Vec::new();
                let sql = self.sql(&args[0], &mut params);
                format!("rt::db::q_where(&{q}, {sql:?}, vec![{}])", params.join(", "))
            }
        }
    }

    /// A :where( ) as SQL. Parts that don't use the columns are worked out first and passed in.
    fn sql(&mut self, e: &Expr, params: &mut Vec<String>) -> String {
        if !crate::check::Checker::mentions_column(self.info, e) {
            let t = self.info.ty(e);
            let p = self.to_sql(e, &t);
            params.push(p);
            return "?".into();
        }
        match &e.kind {
            ExprKind::Name(c) => format!("\"{c}\""),
            ExprKind::Not(x) => format!("(NOT {})", self.sql(x, params)),
            ExprKind::Neg(x) => format!("(-{})", self.sql(x, params)),
            ExprKind::Len(x) => format!("length({})", self.sql(x, params)),
            ExprKind::Method(t, m, args, _) => {
                let a = &args[0];
                match m.as_str() {
                    "contains" => {
                        let tc = self.sql(t, params);
                        let ac = self.sql(a, params);
                        format!("(instr({tc}, {ac}) > 0)")
                    }
                    "startsWith" => {
                        let tc = self.sql(t, params);
                        let a1 = self.sql(a, params);
                        let a2 = self.sql(a, params);
                        format!("(substr({tc}, 1, length({a1})) = {a2})")
                    }
                    _ => {
                        let a1 = self.sql(a, params);
                        let tc = self.sql(t, params);
                        let a2 = self.sql(a, params);
                        let a3 = self.sql(a, params);
                        format!("(length({a1}) = 0 OR substr({tc}, -length({a2})) = {a3})")
                    }
                }
            }
            ExprKind::Bin(op, l, r, _) => crate::lua::sql_bin(*op, l, r, self.info, &mut |x, p| self.sql(x, p), params),
            _ => "NULL".into(),
        }
    }

    /// Something just made, given to whatever owns what's made here: the object's state
    /// option, or the node it's in.
    fn owned(&mut self, made: String) -> String {
        if let Some(l) = &self.owner_label {
            return format!("rt::own(this, {l}, {made})");
        }
        match self.class {
            Some(c) if self.info.stateful(c) => {
                let sf = self.field_of(c, "state");
                format!("rt::own(this, {{ let s = this.borrow().{sf}; s }}, {made})")
            }
            Some(c) if self.info.classes[c].is_node => {
                format!("rt::own(this, {:?}, {made})", crate::ast::shown(&self.info.classes[c].name))
            }
            _ => made,
        }
    }

    /// A view's [ ] as a list of (word, value). Named styles are put in place.
    /// An action in a view (a button's click, an input's change, Enter), kept in `actions`, with
    /// its number put in `slot`. It keeps copies of the view's variables it uses, and the piece
    /// itself. `given` is what an input is bound to: the action gets its new value.
    fn view_action(&mut self, a: &[Stmt], given: Option<&Expr>, slot: &str, out: &mut String, depth: usize) {
        let inner = "    ".repeat(depth + 1);
        out.push_str(&format!("{inner}{{\n"));
        let value = crate::parser::VALUE.to_string();
        for c in captures(a, std::slice::from_ref(&value), self.info) {
            out.push_str(&format!("{inner}    let v_{c} = v_{c}.clone();\n"));
        }
        out.push_str(&format!("{inner}    let this = this.clone();\n"));
        out.push_str(&format!(
            "{inner}    actions.push(Box::new(move || match rt::attempt(|| {{\n{inner}        let this = &this;\n"
        ));
        if let Some(b) = given {
            let opts = match self.info.ty(b) {
                Ty::State(s) => self.info.states[s]
                    .options
                    .iter()
                    .map(|o| format!("{o:?}"))
                    .collect::<Vec<_>>()
                    .join(", "),
                _ => String::new(),
            };
            let state = format!(
                "{{ let s = rt::ui::incoming_text(); [{opts}].into_iter().find(|o| *o == s).unwrap_or({}) }}",
                opts.split(", ").next().unwrap_or("\"\"")
            );
            let get = match self.info.ty(b) {
                Ty::State(_) => state.as_str(),
                Ty::Str => "rt::ui::incoming_text()",
                Ty::Truth => "rt::ui::incoming_truth()",
                Ty::Int => "rt::ui::incoming_num().round() as i64",
                _ => "rt::ui::incoming_num()",
            };
            out.push_str(&format!("{inner}        let v_{value} = {get};\n"));
        }
        let saved = self.current_ret.take();
        self.stmts(a, out, depth + 3);
        self.current_ret = saved;
        out.push_str(&format!(
            "{inner}    }}) {{\n{inner}        None => Ok(()),\n{inner}        Some(e) => Err(e),\n{inner}    }}));\n"
        ));
        out.push_str(&format!("{inner}    {slot} = Some(actions.len() - 1);\n{inner}}}\n"));
    }

    fn styles(&mut self, items: &[StyleItem]) -> String {
        let mut parts = Vec::new();
        self.style_parts(items, &mut parts);
        format!("vec![{}]", parts.join(", "))
    }

    fn style_parts(&mut self, items: &[StyleItem], parts: &mut Vec<String>) {
        for it in items {
            if let (Some(named), StyleValue::Flag) = (self.named_styles.get(&it.name).cloned(), &it.value) {
                self.style_parts(&named, parts);
                continue;
            }
            let v = match &it.value {
                StyleValue::Flag => "rt::ui::SV::Flag".to_string(),
                StyleValue::Word(w) => format!("rt::ui::SV::Text({w:?}.to_string())"),
                StyleValue::Expr(e) => match self.info.ty(e) {
                    Ty::Int | Ty::Float => format!("rt::ui::SV::Num({})", self.coerced(e, &Ty::Float)),
                    // A style that's on only when this is true: [disabled not ready]
                    Ty::Truth => format!("if {} {{ rt::ui::SV::Flag }} else {{ rt::ui::SV::Truth(false) }}", self.expr(e)),
                    _ => format!("rt::ui::SV::Text({})", self.expr(e)),
                },
                StyleValue::Nested(n) => format!("rt::ui::SV::Nested({})", self.styles(n)),
            };
            parts.push(format!("({:?}.to_string(), {v})", crate::ast::shown(&it.name)));
        }
    }

    /// A standard library call.
    fn lib(&mut self, e: &Expr, lib: &crate::stdlib::Lib) -> String {
        let (target, args): (Option<&Expr>, &[Expr]) = match &e.kind {
            ExprKind::Method(t, _, args, _) => ((!crate::stdlib::is_object(t)).then_some(&**t), args),
            _ => (None, &[]),
        };
        match lib.func.as_str() {
            "json_write" | "json_pretty" => {
                let t = self.info.ty(&args[0]);
                let v = self.expr(&args[0]);
                let enc = self.jenc(&t);
                format!("rt::json_write(&{v}, &{enc}, {})", lib.func == "json_pretty")
            }
            "json_read" => {
                let t = self.expr(&args[0]);
                let dec = self.jdec(&lib.ret);
                format!("rt::json_read({t}, &{dec})")
            }
            "web_serve" => format!("rt::web::serve({})", self.expr(&args[0])),
            "files_chooseOpen" => format!("rt::dlg::choose_open({}, rt::acopy(&{}))", self.expr(&args[0]), self.expr(&args[1])),
            "files_chooseSave" => format!("rt::dlg::choose_save({}, {})", self.expr(&args[0]), self.expr(&args[1])),
            "files_chooseFolder" => format!("rt::dlg::choose_folder({})", self.expr(&args[0])),
            "web_get" | "web_post" | "text_matches" | "text_findAll" | "text_replaceAll" | "text_capture" => {
                let (module, name) = match lib.func.as_str() {
                    "web_get" => ("net", "get"),
                    "web_post" => ("net", "post"),
                    "text_matches" => ("pat", "matches"),
                    "text_findAll" => ("pat", "find_all"),
                    "text_replaceAll" => ("pat", "replace_all"),
                    _ => ("pat", "capture"),
                };
                let mut a: Vec<String> = target.map(|t| self.expr(t)).into_iter().collect();
                a.extend(args.iter().map(|x| self.expr(x)));
                format!("rt::{module}::{name}({})", a.join(", "))
            }
            f => {
                let mut a: Vec<String> = target.map(|t| self.expr(t)).into_iter().collect();
                for (x, (_, pt)) in args.iter().zip(&lib.params) {
                    a.push(self.coerced(x, pt));
                }
                format!("rt::{f}({})", a.join(", "))
            }
        }
    }

    /// A Rust function or closure that writes a value of type `t` as JSON.
    fn jenc(&mut self, t: &Ty) -> String {
        let tt = self.ty(t);
        match t {
            Ty::Int => "rt::jw_int".into(),
            Ty::Float => "rt::jw_float".into(),
            Ty::Truth => "rt::jw_truth".into(),
            Ty::Str => "rt::jw_string".into(),
            Ty::State(_) => "rt::jw_state".into(),
            Ty::Class(c) => format!("jw{}", self.root(*c)),
            Ty::Array(el) => {
                let inner = self.jenc(el);
                format!("(|v: &{tt}, d: Option<usize>| -> String {{ rt::jw_array(v, d, &{inner}) }})")
            }
            Ty::Map(el) => {
                let inner = self.jenc(el);
                format!("(|v: &{tt}, d: Option<usize>| -> String {{ rt::jw_map(v, d, &{inner}) }})")
            }
            Ty::CMap(cols) => {
                let ri = self.row_index(cols);
                let mut parts = Vec::new();
                for (n, ct) in cols.iter() {
                    let f = self.jenc(ct);
                    parts.push(format!("({n:?}, ({f})(&r.f_{n}, rt::deeper(d)))"));
                }
                let row = format!(
                    "(|r: &Row{ri}, d: Option<usize>| -> String {{ rt::jw_fields(vec![{}], d) }})",
                    parts.join(", ")
                );
                format!("(|v: &{tt}, d: Option<usize>| -> String {{ rt::jw_cmap(v, d, &{row}) }})")
            }
            _ => "rt::jw_int".into(),
        }
    }

    /// A Rust function or closure that reads a value of type `t` from parsed JSON.
    fn jdec(&mut self, t: &Ty) -> String {
        let tt = self.ty(t);
        match t {
            Ty::Int => "rt::jr_int".into(),
            Ty::Float => "rt::jr_float".into(),
            Ty::Truth => "rt::jr_truth".into(),
            Ty::Str => "rt::jr_string".into(),
            Ty::State(s) => {
                let opts: Vec<String> = self.info.states[*s].options.iter().map(|o| format!("{o:?}")).collect();
                format!(
                    "(|v: &rt::Json, at: &str| -> &'static str {{ rt::jr_state(v, at, &[{}]) }})",
                    opts.join(", ")
                )
            }
            Ty::Class(c) => format!("jr{c}"),
            Ty::Array(el) => {
                let inner = self.jdec(el);
                format!("(|v: &rt::Json, at: &str| -> {tt} {{ rt::jr_array(v, at, &{inner}) }})")
            }
            Ty::Map(el) => {
                let inner = self.jdec(el);
                format!("(|v: &rt::Json, at: &str| -> {tt} {{ rt::jr_map(v, at, &{inner}) }})")
            }
            Ty::CMap(cols) => {
                let ri = self.row_index(cols);
                let kt = self.ty(&cols[0].1);
                let (kn, kty) = cols[0].clone();
                let kdec = self.jdec(&kty);
                let mut body = format!("let k = rt::jget(o, {kn:?}).map(|x| ({kdec})(x, &rt::child(at, {kn:?}))); ");
                let mut fields = Vec::new();
                for (i, (n, ct)) in cols.iter().enumerate().skip(1) {
                    let f = self.jdec(ct);
                    body.push_str(&format!("let c{i} = ({f})(rt::jneed(o, {n:?}, at), &rt::child(at, {n:?})); "));
                    fields.push(format!("f_{n}: c{i}, "));
                }
                let row = format!(
                    "(|o: &rt::Json, at: &str| -> (Option<{kt}>, Row{ri}) {{ {body}(k, Row{ri} {{ {}..Default::default() }}) }})",
                    fields.concat()
                );
                format!("(|v: &rt::Json, at: &str| -> {tt} {{ rt::jr_cmap(v, at, {kn:?}, &{row}) }})")
            }
            _ => "rt::jr_int".into(),
        }
    }

    /// Every class family learns to write itself as JSON, and every class to read itself back.
    fn class_json(&mut self) -> String {
        let info = self.info;
        let n = info.classes.len();
        let mut out = String::new();
        for root in 0..n {
            if info.classes[root].parent.is_some() {
                continue;
            }
            let mut arms = String::new();
            let family: Vec<usize> = (0..n).filter(|&c| self.root(c) == root).collect();
            for c in family {
                let mut parts = Vec::new();
                for (decl, name, ty) in crate::stdlib::json_fields(info, c) {
                    let enc = self.jenc(&ty);
                    parts.push(format!("({name:?}, ({enc})(&b.{}, rt::deeper(d)))", self.fname(decl, &name)));
                }
                arms.push_str(&format!(
                    "ID_{c} => {{ let b = o.borrow(); rt::jw_fields(vec![{}], d) }} ",
                    parts.join(", ")
                ));
            }
            out.push_str(&format!(
                "fn jw{root}(o: &Option<Rc<RefCell<O{root}>>>, d: Option<usize>) -> String {{ let Some(o) = o else {{ return \"null\".to_string() }}; rt::jw_enter(); let c = o.borrow().class; let s = match c {{ {arms}_ => String::new() }}; rt::jw_leave(); s }}\n"
            ));
        }
        for c in 0..n {
            let root = self.root(c);
            let mut sets = String::new();
            for (decl, name, ty) in crate::stdlib::json_fields(info, c) {
                let dec = self.jdec(&ty);
                sets.push_str(&format!(
                    "if let Some(x) = rt::jget(v, {name:?}) {{ let val = ({dec})(x, &rt::child(at, {name:?})); o.borrow_mut().{} = val; }} ",
                    self.fname(decl, &name)
                ));
            }
            out.push_str(&format!(
                "fn jr{c}(v: &rt::Json, at: &str) -> Option<Rc<RefCell<O{root}>>> {{ if !rt::jr_object(v, at) {{ return None; }} {check}let o = new{c}(); {sets}Some(o) }}\n",
                check = {
                    let required: Vec<String> = info.classes[c].required.iter().map(|r| format!("{r:?}")).collect();
                    if required.is_empty() { String::new() } else { format!("rt::jr_required(v, at, &[{}]); ", required.join(", ")) }
                }
            ));
        }
        out
    }

    /// A closure taking &T and giving its text. Inside lists, text is in quotes.
    fn fmt(&mut self, t: &Ty, quoted: bool) -> String {
        if let Ty::Table(x) | Ty::Query(x) = t {
            return self.fmt(&Ty::CMap(x.cols.clone()), quoted);
        }
        let tt = self.ty(t);
        match t {
            Ty::Int => format!("|v: &{tt}| rt::itext(*v)"),
            Ty::Float => format!("|v: &{tt}| rt::ftext(*v)"),
            Ty::Vec3 => format!("|v: &{tt}| rt::vtext(*v)"),
            Ty::Rot => format!("|v: &{tt}| rt::rtext(*v)"),
            Ty::Entity => format!("|v: &{tt}| rt::etext(*v)"),
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
            // A row of a database table: each column read back, shown like a cMap row.
            Ty::TableRow(x) => {
                let mut parts: Vec<String> = Vec::new();
                for (n, ct) in x.cols.iter() {
                    let f = self.fmt(ct, true);
                    let v = self.from_sql(ct, &format!("&rt::db::r_get(r, {n:?})"));
                    parts.push(format!("({f})(&{v})"));
                }
                format!("|r: &{tt}| format!(\"[{{}}]\", vec![{}].join(\", \"))", parts.join(", "))
            }
            _ => format!("|_v: &{tt}| String::from(\"?\")"),
        }
    }

    fn binary(&mut self, _e: &Expr, op: BinOp, l: &Expr, r: &Expr) -> String {
        let lt = self.info.ty(l);
        let rt = self.info.ty(r);
        // Vectors and rotations, with any number as a float.
        if matches!(lt, Ty::Vec3 | Ty::Rot) || matches!(rt, Ty::Vec3 | Ty::Rot) {
            let side = |w: &mut Self, x: &Expr| {
                let t = w.info.ty(x);
                if t.is_number() {
                    w.coerced(x, &Ty::Float)
                } else {
                    w.expr(x)
                }
            };
            let (lc, rc) = (side(self, l), side(self, r));
            return match (op, &lt, &rt) {
                (BinOp::Add, ..) => format!("rt::vadd({lc}, {rc})"),
                (BinOp::Sub, ..) => format!("rt::vsub({lc}, {rc})"),
                (BinOp::Mul, Ty::Vec3, _) => format!("rt::vmul({lc}, {rc})"),
                (BinOp::Mul, Ty::Rot, Ty::Vec3) => format!("rt::qmulv({lc}, {rc})"),
                (BinOp::Mul, Ty::Rot, _) => format!("rt::qmul({lc}, {rc})"),
                (BinOp::Mul, _, _) => format!("rt::vmul({rc}, {lc})"),
                (BinOp::Div, ..) => format!("rt::vdiv({lc}, {rc})"),
                (BinOp::Ne | BinOp::NotSame, ..) => format!("({lc} != {rc})"),
                _ => format!("({lc} == {rc})"),
            };
        }
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

/// Text for .text(): numbers, bools, states and text itself.
fn text(code: &str, t: &Ty) -> String {
    match t {
        Ty::Int => format!("rt::itext({code})"),
        Ty::Float => format!("rt::ftext({code})"),
        Ty::Vec3 => format!("rt::vtext({code})"),
        Ty::Rot => format!("rt::rtext({code})"),
        Ty::Entity => format!("rt::etext({code})"),
        Ty::Truth => format!("rt::btext({code})"),
        Ty::State(_) => format!("{code}.to_string()"),
        _ => code.to_string(),
    }
}

/// A Lust name as a Rust one: Player·2 becomes Player__2.
fn ident(n: &str) -> String {
    n.replace(crate::ast::MARK, "__")
}

fn coerce(code: String, got: &Ty, want: &Ty) -> String {
    match (got, want) {
        (Ty::Int, Ty::Float) => format!("({code} as f64)"),
        (Ty::Nothing, Ty::Entity) => "0u64".into(),
        _ => code,
    }
}
