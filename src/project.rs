//! Programs made of several files: `fetch`, `get` and `pass`.
//!
//! Every file the program reaches is read and parsed, starting with the one being run.
//! Then each file's names are checked against what it brought in, and everything is put
//! together into one program for the checker. Only the first file's code runs; the code at
//! the bottom of a fetched file is left out, so every file can still be run on its own to try it.
//!
//! Two files can each have their own private thing with the same name. Inside Lust those get
//! a mark and the file's number (Player·2), and people only ever see "Player".

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::ast::*;
use crate::diag::Diags;

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Class,
    Object,
    State,
    Method,
    Assembly,
    Database,
    Style,
    Event,
}

impl Kind {
    fn word(self) -> &'static str {
        match self {
            Kind::Class => "a class",
            Kind::Object => "an object",
            Kind::State => "a state",
            Kind::Method => "a method",
            Kind::Assembly => "an assembly",
            Kind::Database => "a database",
            Kind::Style => "a style",
            Kind::Event => "an event",
        }
    }
}

pub struct Project {
    pub program: Program,
    /// Every file, by number, as people see it: "main.lst", "entities/player.lst".
    pub files: Vec<String>,
}

struct File {
    shown: String,
    prog: Program,
}

/// Where the files come from: the disk (lust itself), or the program using the compiler, which
/// hands over every file's text by its path.
pub type Memory = HashMap<PathBuf, String>;

/// A path without . and .. in it, worked out from the text alone: files handed over have no
/// disk to ask.
pub fn tidy(p: &Path) -> PathBuf {
    use std::path::Component;
    let mut out = PathBuf::new();
    for c in p.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir => {
                if !out.pop() {
                    out.push("..");
                }
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

fn is_file(p: &Path, mem: Option<&Memory>) -> bool {
    match mem {
        Some(m) => m.contains_key(&tidy(p)),
        None => p.is_file(),
    }
}

/// The project's folder: the nearest folder up from the file that has a main.lst (or main.lsc).
fn root_of(entry: &Path, ext: &str, mem: Option<&Memory>) -> PathBuf {
    let start = entry.parent().map(|p| p.to_path_buf()).unwrap_or_default();
    let start = if start.as_os_str().is_empty() && mem.is_none() {
        PathBuf::from(".")
    } else {
        start
    };
    let mut dir = Some(start.as_path());
    while let Some(d) = dir {
        if is_file(&d.join(format!("main.{ext}")), mem) {
            return d.to_path_buf();
        }
        dir = d.parent().filter(|p| !p.as_os_str().is_empty());
    }
    start
}

fn shown_path(path: &Path, root: &Path) -> String {
    let rel = path.strip_prefix(root).unwrap_or(path);
    rel.to_string_lossy().replace('\\', "/")
}

/// Reads the program that starts at `entry`, with every file it brings in.
pub fn load(entry: &Path, src: String, is_script: bool, diags: &mut Diags, mem: Option<&Memory>) -> Project {
    let ext = if is_script { "lsc" } else { "lst" };
    // The search for the project's folder starts from the file's full path, so a short one like
    // entities/player.lst still finds main.lst in the folder above.
    let full = match mem {
        Some(_) => tidy(entry),
        None => std::path::absolute(entry).unwrap_or_else(|_| entry.to_path_buf()),
    };
    let entry = full.as_path();
    let root = root_of(entry, ext, mem);
    let canon = |p: &Path| match mem {
        Some(_) => tidy(p),
        None => std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf()),
    };
    let mut files: Vec<File> = Vec::new();
    // A file's number is its place in the queue.
    let mut queue: Vec<(PathBuf, Option<String>)> = vec![(entry.to_path_buf(), Some(src))];
    let mut index: HashMap<PathBuf, usize> = HashMap::from([(canon(entry), 0)]);
    // The file each import points at, by (file, import number).
    let mut targets: HashMap<(usize, usize), usize> = HashMap::new();
    let mut next_id = 0;
    let mut i = 0;
    while i < queue.len() {
        let (path, src) = queue[i].clone();
        let src = src.unwrap_or_else(|| {
            let text = match mem {
                Some(m) => m.get(&tidy(&path)).cloned().unwrap_or_default(),
                None => std::fs::read_to_string(&path).unwrap_or_default(),
            };
            text.replace("\r\n", "\n")
        });
        let toks = crate::lexer::lex(&src, i, diags);
        let prog = crate::parser::parse(toks, diags, is_script, next_id);
        next_id = prog.next_id;
        for (k, imp) in prog.imports.iter().enumerate() {
            let pos = imp.path_pos;
            let bad = imp.path.contains('-')
                || imp.path.contains('\\')
                || imp.path.starts_with('/')
                || imp.path.ends_with(".lst")
                || imp.path.ends_with(".lsc");
            if bad {
                diags.add(
                    pos,
                    format!("write the path from the project's folder, with / between folders and without .{ext}: \"entities/player\"."),
                );
                continue;
            }
            let p = root.join(format!("{}.{ext}", imp.path));
            if !is_file(&p, mem) {
                diags.add(pos, format!("there's no file {}.", shown_path(&p, &root)));
                continue;
            }
            let t = match index.get(&canon(&p)) {
                Some(&t) => t,
                None => {
                    queue.push((p.clone(), None));
                    index.insert(canon(&p), queue.len() - 1);
                    queue.len() - 1
                }
            };
            targets.insert((i, k), t);
        }
        files.push(File {
            shown: shown_path(&path, &root),
            prog,
        });
        i += 1;
    }

    // What each file declares, and the name it has inside Lust.
    let declared: Vec<HashMap<String, Kind>> = files.iter().map(|f| declarations(&f.prog)).collect();
    let mut count: HashMap<&str, usize> = HashMap::new();
    for d in &declared {
        for name in d.keys() {
            *count.entry(name.as_str()).or_default() += 1;
        }
    }
    let inner = |file: usize, name: &str| -> String {
        if count.get(name).copied().unwrap_or(0) > 1 {
            format!("{name}{MARK}{file}")
        } else {
            name.to_string()
        }
    };

    // Passes name things the file has.
    for (f, file) in files.iter().enumerate() {
        for (name, pos) in &file.prog.passes {
            // pass route; is welcome but not needed: every file's routes are served.
            if name != "route" && !declared[f].contains_key(name) {
                diags.add(*pos, format!("this file has no {name} to pass."));
            }
        }
    }

    // What each file can see: its own things, and what it brings in.
    let mut visible: Vec<HashMap<String, String>> = Vec::new();
    for (f, file) in files.iter().enumerate() {
        let mut v: HashMap<String, String> = declared[f].keys().map(|n| (n.clone(), inner(f, n))).collect();
        for (k, imp) in file.prog.imports.iter().enumerate() {
            let Some(&t) = targets.get(&(f, k)) else { continue };
            let target = &files[t];
            for (name, pos) in &imp.names {
                let Some(&kind) = declared[t].get(name) else {
                    diags.add(*pos, format!("{} has no {name}.", target.shown));
                    continue;
                };
                if !target.prog.passes.iter().any(|(p, _)| p == name) {
                    diags.add(
                        *pos,
                        format!(
                            "{} keeps {name} to itself. To share it, add pass {name}; to {}.",
                            target.shown, target.shown
                        ),
                    );
                    continue;
                }
                if imp.is_get && kind != Kind::Method {
                    diags.add(
                        *pos,
                        format!("{name} is {}, so it's brought in with fetch \"{}\":{name};", kind.word(), imp.path),
                    );
                    continue;
                }
                if !imp.is_get && kind == Kind::Method {
                    diags.add(
                        *pos,
                        format!("{name} is a method, so it's brought in with get \"{}\":{name};", imp.path),
                    );
                    continue;
                }
                if let Some(own) = declared[f].get(name) {
                    let _ = own;
                    diags.add(*pos, format!("this file has its own {name} already."));
                    continue;
                }
                v.insert(name.clone(), inner(t, name));
            }
        }
        visible.push(v);
    }

    // Where everything lives, for "bring it in with fetch" hints.
    let mut home: HashMap<String, (usize, Kind, bool)> = HashMap::new();
    for (f, d) in declared.iter().enumerate() {
        for (name, kind) in d {
            let passed = files[f].prog.passes.iter().any(|(p, _)| p == name);
            let e = home.entry(name.clone()).or_insert((f, *kind, passed));
            // Point at a file that shares it, if one does.
            if passed && !e.2 {
                *e = (f, *kind, passed);
            }
        }
    }
    let shown: Vec<String> = files.iter().map(|f| f.shown.clone()).collect();

    // Everything named like a variable somewhere: those uses aren't methods from other files.
    let mut values: HashSet<String> = HashSet::new();
    for f in &files {
        value_names(&f.prog, &mut values);
    }

    // What the first file passes: what a library made from it shows to the program using it.
    let exports: Vec<(String, Pos)> = files[0].prog.passes.iter().map(|(n, p)| (inner(0, n), *p)).collect();
    let mut items = Vec::new();
    let mut body = Vec::new();
    let mut uses_lua = false;
    for (f, file) in files.into_iter().enumerate() {
        let mut r = Renamer {
            visible: &visible[f],
            home: &home,
            shown: &shown,
            root_ext: ext,
            values: &values,
            diags,
        };
        let mut prog = file.prog;
        for item in &mut prog.items {
            r.item(item);
        }
        uses_lua |= prog.uses_lua;
        if f == 0 {
            r.stmts(&mut prog.body);
            body = prog.body;
        }
        items.extend(prog.items);
    }
    Project {
        program: Program {
            items,
            body,
            uses_lua,
            imports: vec![],
            passes: exports,
            next_id,
        },
        files: shown,
    }
}

fn declarations(p: &Program) -> HashMap<String, Kind> {
    let mut d = HashMap::new();
    for item in &p.items {
        match item {
            Item::State(s) => d.insert(s.name.clone(), Kind::State),
            Item::Class(c) => d.insert(c.name.clone(), if c.is_object { Kind::Object } else { Kind::Class }),
            Item::Method(m) => d.insert(m.name.clone(), Kind::Method),
            Item::Assembly(a) => d.insert(a.name.clone(), Kind::Assembly),
            Item::Database(b) => d.insert(b.name.clone(), Kind::Database),
            Item::Route(_) => None,
            Item::Style(s) => d.insert(s.name.clone(), Kind::Style),
            Item::Event(e) => d.insert(e.name.clone(), Kind::Event),
        };
    }
    d
}

/// Names of variables, parameters, fields and class methods: things a bare name can mean
/// besides a method on its own.
fn value_names(p: &Program, out: &mut HashSet<String>) {
    fn stmts(list: &[Stmt], out: &mut HashSet<String>) {
        for s in list {
            match s {
                Stmt::Var { name, .. } => {
                    out.insert(name.clone());
                }
                Stmt::If { branches, otherwise, .. } => {
                    for (_, b) in branches {
                        stmts(b, out);
                    }
                    if let Some(o) = otherwise {
                        stmts(o, out);
                    }
                }
                Stmt::Match { arms, .. } => {
                    for a in arms {
                        stmts(&a.body, out);
                    }
                }
                Stmt::Loop { kind, body, .. } => {
                    if let LoopKind::Each { vars, .. } = kind {
                        for (_, n, _) in vars {
                            out.insert(n.clone());
                        }
                    }
                    stmts(body, out);
                }
                Stmt::Try { body, err, handler, .. } => {
                    out.insert(err.0.clone());
                    stmts(body, out);
                    stmts(handler, out);
                }
                Stmt::Element {
                    children, action, enter, ..
                } => {
                    stmts(children, out);
                    for a in [action, enter].into_iter().flatten() {
                        stmts(a, out);
                    }
                }
                _ => {}
            }
        }
    }
    let method = |m: &MethodDecl, out: &mut HashSet<String>| {
        for p in &m.params {
            out.insert(p.name.clone());
        }
        stmts(&m.body, out);
    };
    for item in &p.items {
        match item {
            Item::Class(c) => {
                for f in &c.fields {
                    out.insert(f.name.clone());
                }
                for m in &c.methods {
                    out.insert(m.name.clone());
                    method(m, out);
                }
                if let Some(v) = &c.view {
                    stmts(&v.body, out);
                }
            }
            Item::Method(m) => method(m, out),
            Item::State(_) | Item::Assembly(_) | Item::Database(_) | Item::Route(_) | Item::Style(_) | Item::Event(_) => {}
        }
    }
    stmts(&p.body, out);
    // Hook parameters are named by their hook's type; any name could be one, so they're
    // collected from hook types as the checker sees them. Names inside `new hook` bodies are
    // covered by the statements above only when declared there; uses of a hook's own
    // parameters are bare names that match no method, so they're never renamed anyway.
}

struct Renamer<'a> {
    visible: &'a HashMap<String, String>,
    home: &'a HashMap<String, (usize, Kind, bool)>,
    shown: &'a [String],
    root_ext: &'a str,
    values: &'a HashSet<String>,
    diags: &'a mut Diags,
}

impl<'a> Renamer<'a> {
    /// The inner name for a name used in this file, or a mistake if it lives in another file.
    fn name(&mut self, n: &mut String, pos: Pos, could_be_value: bool) {
        if let Some(inner) = self.visible.get(n.as_str()) {
            *n = inner.clone();
            return;
        }
        if could_be_value && self.values.contains(n.as_str()) {
            return;
        }
        if let Some(&(file, kind, passed)) = self.home.get(n.as_str()) {
            let path = self.shown[file].trim_end_matches(&format!(".{}", self.root_ext)).to_string();
            let how = if kind == Kind::Method { "get" } else { "fetch" };
            let file = &self.shown[file];
            if passed {
                self.diags.add(
                    pos,
                    format!("{n} is in {file}. Bring it in at the top of this file: {how} \"{path}\":{n};"),
                );
            } else {
                self.diags.add(pos, format!("{n} is in {file}, which keeps it to itself. To share it, add pass {n}; to {file}, then {how} \"{path}\":{n}; here."));
            }
        }
    }

    fn item(&mut self, item: &mut Item) {
        match item {
            Item::State(s) => self.name(&mut s.name, s.pos, false),
            Item::Class(c) => {
                self.name(&mut c.name, c.pos, false);
                if let Some((p, pos)) = &mut c.parent {
                    self.name(p, *pos, false);
                }
                for f in &mut c.fields {
                    if let Some(t) = &mut f.ty {
                        self.ty(t);
                    }
                    if let Some(d) = &mut f.default {
                        self.expr(d);
                    }
                }
                for m in &mut c.methods {
                    self.method(m);
                }
                if let Some(v) = &mut c.view {
                    self.styles(&mut v.settings);
                    self.stmts(&mut v.body);
                }
            }
            Item::Method(m) => {
                self.name(&mut m.name, m.pos, false);
                self.method(m);
            }
            Item::Style(s) => {
                self.name(&mut s.name, s.pos, false);
                self.styles(&mut s.items);
            }
            Item::Route(r) => {
                for (_, _, handler, pos) in &mut r.routes {
                    self.name(handler, *pos, false);
                }
            }
            Item::Database(b) => {
                self.name(&mut b.name, b.pos, false);
                for (t, _, _) in &mut b.tables {
                    self.ty(t);
                }
            }
            Item::Assembly(a) => {
                self.name(&mut a.name, a.pos, false);
                for (class, pos, made) in &mut a.recipes {
                    self.name(class, *pos, false);
                    self.expr(made);
                }
            }
            Item::Event(e) => {
                self.name(&mut e.name, e.pos, false);
                for p in &mut e.params {
                    self.ty(&mut p.ty);
                }
            }
        }
    }

    /// Styles: values may use names, and a style's own name may be a named style.
    fn styles(&mut self, items: &mut [StyleItem]) {
        for it in items {
            if let Some(inner) = self.visible.get(it.name.as_str()) {
                it.name = inner.clone();
            }
            match &mut it.value {
                StyleValue::Expr(e) => self.expr(e),
                StyleValue::Nested(n) => self.styles(n),
                _ => {}
            }
        }
    }

    fn method(&mut self, m: &mut MethodDecl) {
        if let Some((e, pos)) = &mut m.on {
            self.name(e, *pos, false);
        }
        if let Some(r) = &mut m.ret {
            self.ty(r);
        }
        for p in &mut m.params {
            self.ty(&mut p.ty);
        }
        self.stmts(&mut m.body);
    }

    fn ty(&mut self, t: &mut TypeRef) {
        match t {
            TypeRef::Named(n, pos) => {
                if !matches!(n.as_str(), "int" | "float" | "string" | "bool") {
                    self.name(n, *pos, false);
                }
            }
            TypeRef::Array(e) | TypeRef::Map(e) => self.ty(e),
            TypeRef::CMap(cols) => {
                for c in cols {
                    self.ty(&mut c.ty);
                    if let Some(m) = &mut c.maker {
                        self.expr(m);
                    }
                }
            }
            TypeRef::Hook { ret, params } => {
                if let Some(r) = ret {
                    self.ty(r);
                }
                for p in params {
                    self.ty(&mut p.ty);
                }
            }
        }
    }

    fn stmts(&mut self, list: &mut [Stmt]) {
        for s in list {
            self.stmt(s);
        }
    }

    fn stmt(&mut self, s: &mut Stmt) {
        match s {
            Stmt::Var { ty, value, .. } => {
                self.ty(ty);
                self.expr(value);
            }
            Stmt::Assign { target, value, .. } | Stmt::SetState { target, option: value, .. } => {
                self.expr(target);
                self.expr(value);
            }
            Stmt::Expr(e) => self.expr(e),
            Stmt::If { branches, otherwise, .. } => {
                for (c, b) in branches {
                    self.expr(c);
                    self.stmts(b);
                }
                if let Some(o) = otherwise {
                    self.stmts(o);
                }
            }
            Stmt::Match { subject, arms, .. } => {
                self.expr(subject);
                for a in arms {
                    self.stmts(&mut a.body);
                }
            }
            Stmt::Loop { kind, body, .. } => {
                match kind {
                    LoopKind::Times(e) | LoopKind::While(e) => self.expr(e),
                    LoopKind::Each { vars, source } => {
                        for (t, _, _) in vars {
                            self.ty(t);
                        }
                        self.expr(source);
                    }
                    LoopKind::Forever => {}
                }
                self.stmts(body);
            }
            Stmt::Return(Some(e), _) => self.expr(e),
            Stmt::Try { body, handler, .. } => {
                self.stmts(body);
                self.stmts(handler);
            }
            Stmt::SaveDecl { items, .. } => {
                for (n, p) in items {
                    if n.starts_with(|c: char| c.is_ascii_uppercase()) {
                        self.name(n, *p, false);
                    }
                }
            }
            Stmt::Keep { value, into, .. } => {
                self.expr(value);
                if let Some(i) = into {
                    self.expr(i);
                }
            }
            Stmt::Send { event, epos, args, to, .. } => {
                self.name(event, *epos, false);
                for a in args {
                    self.expr(a);
                }
                self.expr(to);
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
                    self.expr(a);
                }
                self.styles(style);
                self.stmts(children);
                for a in [action, enter].into_iter().flatten() {
                    self.stmts(a);
                }
            }
            _ => {}
        }
    }

    fn expr(&mut self, e: &mut Expr) {
        let pos = e.pos;
        match &mut e.kind {
            ExprKind::Name(n) => {
                let could_be_value = !n.starts_with(|c: char| c.is_ascii_uppercase());
                self.name(n, pos, could_be_value);
            }
            ExprKind::Call(n, args) => {
                if !crate::parser::BUILTINS.contains(&n.as_str()) {
                    self.name(n, pos, true);
                }
                for a in args {
                    self.expr(a);
                }
            }
            ExprKind::New { class, class_pos, inits } => {
                self.name(class, *class_pos, false);
                for (_, _, v) in inits {
                    self.expr(v);
                }
            }
            ExprKind::Like(t, class, cpos) => {
                self.expr(t);
                self.name(class, *cpos, false);
            }
            ExprKind::Field(t, ..)
            | ExprKind::MethodRef(t, ..)
            | ExprKind::Len(t)
            | ExprKind::Neg(t)
            | ExprKind::Not(t)
            | ExprKind::Convert(t, ..) => self.expr(t),
            ExprKind::Method(t, _, args, _) => {
                self.expr(t);
                for a in args {
                    self.expr(a);
                }
            }
            ExprKind::Index(a, b) | ExprKind::Bin(_, a, b, _) | ExprKind::Random(a, b) => {
                self.expr(a);
                self.expr(b);
            }
            ExprKind::Either(a, b, c) => {
                self.expr(a);
                self.expr(b);
                self.expr(c);
            }
            ExprKind::List { items, keys } => {
                for i in items {
                    self.expr(i);
                }
                for k in keys.iter_mut().flatten() {
                    self.expr(k);
                }
            }
            ExprKind::NewHook(body) => self.stmts(body),
            _ => {}
        }
    }
}
