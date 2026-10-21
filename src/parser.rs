//! Reads tokens into a Program, reporting syntax mistakes in plain words.

use crate::ast::*;
use crate::diag::Diags;
use crate::lexer::{Tk, Token};

pub const RESERVED: &[&str] = &[
    "class", "object", "node", "assembly", "create", "has", "method", "hook", "return", "like", "new", "nothing", "schema", "route",
    "view", "style", "keep", "pass", "fetch", "get", "lua", "is", "then", "or", "either", "not", "true", "false", "loop", "stop", "skip",
    "in", "try", "error", "state", "save", "load", "saved", "end", "database", "int", "float", "string", "truth", "array", "cMap", "map",
];
pub const BUILTINS: &[&str] = &["print", "wait", "random", "ask"];

/// Words from other languages, with what Lust says instead.
fn hint_for(word: &str) -> Option<&'static str> {
    Some(match word {
        "if" => "Lust asks questions instead of if: is hp == 0? then { }",
        "else" | "elif" | "elseif" => "Lust writes or { } for the other case, and or is …? then { } for another question.",
        "while" | "for" | "foreach" | "repeat" => "Lust has one loop: loop (hp > 0) { }, loop 3 { }, loop (int x in list) { }",
        "function" | "def" | "fn" | "func" | "do" => "Lust makes methods with method: method greet() { }",
        "var" | "let" | "const" | "auto" => "put the type first: int lives = 3;",
        "null" | "nil" | "None" => "Lust calls it nothing.",
        "self" | "this" => "inside a method, use a field's name on its own: hp, not self.hp.",
        "send" | "export" => "Lust shares things with pass: pass Player;",
        "import" | "require" => "Lust brings in other files with fetch.",
        "give" => "Lust gives values back with return.",
        "switch" | "match" | "case" => "Lust asks questions: is x == 1? then { } or is x == 2? then { }",
        "enum" => "Lust calls it a state: state Mood = [Happy, Grumpy];",
        "extends" | "inherits" => "Lust uses like: class Wizard like Player { }",
        "bool" | "boolean" => "Lust calls it truth: truth alive = true;",
        "double" => "Lust calls it float.",
        "str" => "Lust calls it string.",
        "void" => "Lust has no void. Leave the return type out when nothing comes back.",
        _ => return None,
    })
}

const CONVERSIONS: &[(&str, Conv)] = &[
    ("text", Conv::Text),
    ("int", Conv::Int),
    ("float", Conv::Float),
    ("isNumber", Conv::IsNumber),
    ("lines", Conv::Lines),
    ("upper", Conv::Upper),
    ("lower", Conv::Lower),
    ("trim", Conv::Trim),
];

#[derive(Clone, Copy, PartialEq)]
pub enum NameKind {
    Type,
    Value,
}

/// Checks a declared name against Lust's naming rules.
pub fn check_name(diags: &mut Diags, name: &str, pos: Pos, kind: NameKind, what: &str) {
    if RESERVED.contains(&name) {
        diags.add(pos, format!("{name} is one of Lust's own words, so it can't be a name."));
        return;
    }
    if BUILTINS.contains(&name) {
        diags.add(pos, format!("{name} is a built-in, so it can't be a name."));
        return;
    }
    if crate::stdlib::OBJECTS.contains(&name) {
        diags.add(pos, format!("{name} is one of Lust's built-in objects, so it can't be a name."));
        return;
    }
    let letters: Vec<char> = name.chars().filter(|c| c.is_alphabetic()).collect();
    if letters.len() >= 2 && letters.iter().all(|c| c.is_uppercase()) {
        let camel = to_camel(name, kind == NameKind::Type);
        diags.add(pos, format!("names can't be in capitals. Try {camel}."));
        return;
    }
    let first = name.chars().next().unwrap_or('a');
    match kind {
        NameKind::Type if !first.is_uppercase() => {
            diags.add(pos, format!("{what} names start with a capital letter: {}", capitalise(name)));
        }
        NameKind::Value if first.is_uppercase() => {
            diags.add(pos, format!("{what} names start with a small letter: {}", decapitalise(name)));
        }
        _ => {}
    }
}

fn capitalise(s: &str) -> String {
    let mut c = s.chars();
    c.next()
        .map(|f| f.to_uppercase().collect::<String>() + c.as_str())
        .unwrap_or_default()
}
fn decapitalise(s: &str) -> String {
    let mut c = s.chars();
    c.next()
        .map(|f| f.to_lowercase().collect::<String>() + c.as_str())
        .unwrap_or_default()
}
fn to_camel(s: &str, upper_first: bool) -> String {
    let parts: Vec<String> = s.split('_').filter(|p| !p.is_empty()).map(|p| p.to_lowercase()).collect();
    let mut out = String::new();
    for (i, p) in parts.iter().enumerate() {
        if i == 0 && !upper_first {
            out.push_str(p);
        } else {
            out.push_str(&capitalise(p));
        }
    }
    out
}

struct Stop;
type R<T> = Result<T, Stop>;

pub struct Parser<'a> {
    toks: Vec<Token>,
    p: usize,
    diags: &'a mut Diags,
    next_id: usize,
    is_script: bool,
    uses_lua: bool,
    loop_depth: usize,
    last_field: Option<FieldDecl>,
    imports: Vec<Import>,
    passes: Vec<(String, Pos)>,
}

pub fn parse(toks: Vec<Token>, diags: &mut Diags, is_script: bool, first_id: usize) -> Program {
    let mut p = Parser {
        toks,
        p: 0,
        diags,
        next_id: first_id,
        imports: vec![],
        passes: vec![],
        is_script,
        uses_lua: false,
        loop_depth: 0,
        last_field: None,
    };
    p.program()
}

impl<'a> Parser<'a> {
    // ----- token helpers -----

    fn cur(&self) -> &Token {
        &self.toks[self.p.min(self.toks.len() - 1)]
    }
    fn peek(&self, n: usize) -> &Token {
        &self.toks[(self.p + n).min(self.toks.len() - 1)]
    }
    fn prev_end(&self) -> Pos {
        if self.p == 0 {
            Pos { line: 1, col: 1 }
        } else {
            self.toks[self.p - 1].end
        }
    }
    fn next(&mut self) -> Token {
        let t = self.cur().clone();
        if self.p < self.toks.len() - 1 {
            self.p += 1;
        }
        t
    }
    fn at_sym(&self, s: &str) -> bool {
        self.cur().is_sym(s)
    }
    fn at_word(&self, w: &str) -> bool {
        self.cur().is_word(w)
    }
    fn at_eof(&self) -> bool {
        matches!(self.cur().kind, Tk::Eof)
    }
    fn fail<T>(&mut self, pos: Pos, msg: impl Into<String>) -> R<T> {
        self.diags.add(pos, msg);
        Err(Stop)
    }
    fn expect_sym(&mut self, s: &str, msg: &str) -> R<Token> {
        if self.at_sym(s) {
            Ok(self.next())
        } else {
            let at = if self.cur().pos.line > self.prev_end().line {
                self.prev_end()
            } else {
                self.cur().pos
            };
            self.fail(at, msg)
        }
    }
    fn ident(&mut self, what: &str) -> R<(String, Pos)> {
        match self.cur().kind.clone() {
            Tk::Ident(s) => {
                let t = self.next();
                Ok((s, t.pos))
            }
            _ => {
                let pos = self.cur().pos;
                self.fail(pos, format!("expected {what} here."))
            }
        }
    }
    fn id(&mut self) -> usize {
        self.next_id += 1;
        self.next_id
    }
    fn mk(&mut self, pos: Pos, kind: ExprKind) -> Expr {
        Expr { id: self.id(), pos, kind }
    }

    /// The ; at the end of a statement. A missing one is reported where the line ends.
    fn semi(&mut self) -> R<()> {
        if self.at_sym(";") {
            self.next();
            return Ok(());
        }
        let end = self.prev_end();
        if self.cur().pos.line > end.line || self.at_sym("}") || self.at_eof() {
            self.diags.add(end, "missing ; at the end of this line.");
            return Ok(());
        }
        let pos = self.cur().pos;
        self.fail(pos, format!("expected ; here, but found {}.", self.cur().describe()))
    }

    /// After a mistake: skip to the end of the statement, so the next one can be read.
    fn recover(&mut self) {
        let start_line = self.cur().pos.line;
        let mut depth = 0i32;
        while !self.at_eof() {
            if self.at_sym("{") {
                depth += 1;
            } else if self.at_sym("}") {
                if depth == 0 {
                    return;
                }
                depth -= 1;
                if depth == 0 {
                    self.next();
                    return;
                }
            } else if self.at_sym(";") && depth == 0 {
                self.next();
                return;
            } else if depth == 0 && self.cur().pos.line > start_line && self.p > 0 && self.starts_statement() {
                return;
            }
            self.next();
        }
    }

    fn starts_statement(&self) -> bool {
        matches!(&self.cur().kind, Tk::Ident(w) if matches!(w.as_str(),
            "is" | "loop" | "method" | "class" | "object" | "state" | "return" | "stop" | "skip" | "try" | "end" | "fetch" | "pass"))
    }

    // ----- the file -----

    /// After fetch or get: "path":Name; or "path":[A, B];
    fn import(&mut self, is_get: bool) -> R<()> {
        let example = if is_get {
            "get \"helpers/maths\":clamp;"
        } else {
            "fetch \"entities/player\":Player;"
        };
        let t = self.cur().clone();
        let Tk::Str(path) = &t.kind else {
            return self.fail(
                t.pos,
                format!("the file's path goes in quotes, from the project's folder: {example}"),
            );
        };
        let path = path.clone();
        self.next();
        self.expect_sym(":", &format!("after the path comes : and what to bring in: {example}"))?;
        let mut names = Vec::new();
        if self.at_sym("[") {
            self.next();
            while !self.at_sym("]") {
                names.push(self.ident("a name to bring in")?);
                if self.at_sym(",") {
                    self.next();
                } else if !self.at_sym("]") {
                    let pos = self.cur().pos;
                    return self.fail(pos, "expected , or ] here.");
                }
            }
            self.next();
        } else {
            names.push(self.ident("a name to bring in")?);
        }
        self.imports.push(Import {
            path,
            path_pos: t.pos,
            is_get,
            names,
        });
        self.semi()
    }

    fn program(&mut self) -> Program {
        let mut items = Vec::new();
        let mut body = Vec::new();
        let mut seen_other = false;
        let mut ended = false;
        while !self.at_eof() {
            if self.at_word("end") && !self.peek(1).is_sym(";") && !self.peek(1).is_sym("(") {
                self.next();
                if !self.at_eof() {
                    let pos = self.cur().pos;
                    self.diags.add(pos, "nothing can come after end. end is the file's last word.");
                }
                ended = true;
                break;
            }
            let r = self.top_level(&mut items, &mut body, &mut seen_other);
            if r.is_err() {
                self.recover();
            }
        }
        if !ended {
            let end = self.prev_end();
            self.diags.add(end, "every file ends with the word end on its last line.");
        }
        Program {
            items,
            body,
            uses_lua: self.uses_lua,
            imports: std::mem::take(&mut self.imports),
            passes: std::mem::take(&mut self.passes),
            next_id: self.next_id + 1,
        }
    }

    fn top_level(&mut self, items: &mut Vec<Item>, body: &mut Vec<Stmt>, seen_other: &mut bool) -> R<()> {
        let t = self.cur().clone();
        let word = match &t.kind {
            Tk::Ident(w) => w.clone(),
            _ => String::new(),
        };
        match word.as_str() {
            "fetch" | "get" => {
                self.next();
                if *seen_other {
                    self.diags
                        .add(t.pos, format!("{word} lines go at the top of the file, before anything else."));
                }
                if word == "fetch" && self.at_word("lua") {
                    self.next();
                    if !self.is_script {
                        self.diags.add(t.pos, "fetch lua; only works in LustScript (.lsc) files.");
                    }
                    self.uses_lua = true;
                    return self.semi();
                }
                if word == "fetch" && self.at_word("cs") {
                    return self.fail(t.pos, "Lust v2 doesn't use .NET, so there's no fetch cs:.");
                }
                return self.import(word == "get");
            }
            "pass" => {
                self.next();
                *seen_other = true;
                loop {
                    let (name, pos) = self.ident("the name of something in this file to pass")?;
                    self.passes.push((name, pos));
                    if self.at_sym(",") {
                        self.next();
                    } else {
                        break;
                    }
                }
                return self.semi();
            }
            "state" if matches!(self.peek(1).kind, Tk::Ident(_)) && self.peek(2).is_sym("=") => {
                *seen_other = true;
                let s = self.state_decl()?;
                items.push(Item::State(s));
                return Ok(());
            }
            "class" | "object" => {
                *seen_other = true;
                let c = self.class_decl()?;
                items.push(Item::Class(c));
                return Ok(());
            }
            "method" => {
                *seen_other = true;
                let m = self.method_decl()?;
                items.push(Item::Method(m));
                return Ok(());
            }
            "node" => {
                *seen_other = true;
                let c = self.class_decl()?;
                items.push(Item::Class(c));
                return Ok(());
            }
            "assembly" => {
                *seen_other = true;
                let a = self.assembly_decl()?;
                items.push(Item::Assembly(a));
                return Ok(());
            }
            "database" => {
                *seen_other = true;
                let d = self.database_decl()?;
                items.push(Item::Database(d));
                return Ok(());
            }
            "schema" => {
                *seen_other = true;
                let c = self.class_decl()?;
                items.push(Item::Class(c));
                return Ok(());
            }
            "route" => {
                *seen_other = true;
                let r = self.route_decl()?;
                items.push(Item::Route(r));
                return Ok(());
            }
            "view" | "style" => {
                return self.fail(t.pos, format!("{word} is planned for Lust v2 but isn't built yet."));
            }
            _ => {}
        }
        *seen_other = true;
        let s = self.statement()?;
        body.push(s);
        Ok(())
    }

    fn state_decl(&mut self) -> R<StateDecl> {
        self.next(); // state
        let (name, pos) = self.ident("the state's name")?;
        check_name(self.diags, &name, pos, NameKind::Type, "state");
        self.expect_sym("=", "expected = after the state's name: state Mood = [Happy, Grumpy];")?;
        let open = self.expect_sym("[", "a state lists its options in [ ]: state Mood = [Happy, Grumpy];")?;
        let options = self.state_options(open.pos)?;
        if options.is_empty() {
            self.diags.add(pos, format!("{name} needs at least one option."));
        }
        self.semi()?;
        Ok(StateDecl { name, pos, options })
    }

    fn state_options(&mut self, open: Pos) -> R<Vec<StateOption>> {
        let mut out = Vec::new();
        loop {
            if self.at_sym("]") {
                self.next();
                return Ok(out);
            }
            if self.at_eof() {
                return self.fail(open, "this [ is never closed. Add a ] after the last option.");
            }
            let (name, pos) = self.ident("an option's name")?;
            check_name(self.diags, &name, pos, NameKind::Type, "option");
            if out.iter().any(|o: &StateOption| o.name == name) {
                self.diags.add(pos, format!("{name} is already an option here."));
            }
            let mut children = Vec::new();
            if self.at_sym("[") {
                let o = self.next();
                children = self.state_options(o.pos)?;
            }
            out.push(StateOption { name, pos, children });
            if self.at_sym(",") {
                self.next();
            } else if !self.at_sym("]") {
                let pos = self.cur().pos;
                return self.fail(pos, "expected , or ] between options.");
            }
        }
    }

    fn class_decl(&mut self) -> R<ClassDecl> {
        let word = self.next();
        let is_node = word.is_word("node");
        let is_schema = word.is_word("schema");
        let is_object = word.is_word("object") || is_node;
        let what = if is_node {
            "node"
        } else if is_schema {
            "schema"
        } else if is_object {
            "object"
        } else {
            "class"
        };
        let (name, pos) = self.ident(&format!("the {what}'s name"))?;
        check_name(self.diags, &name, pos, NameKind::Type, what);
        let mut parent = None;
        if self.at_word("like") {
            self.next();
            parent = Some(self.ident("the class it's like")?);
        }
        let open = self.expect_sym("{", &format!("expected {{ to start the {what}'s body."))?;
        let mut fields = Vec::new();
        let mut methods = Vec::new();
        let mut seen_has = false;
        loop {
            if self.at_sym("}") {
                self.next();
                break;
            }
            if self.at_eof() {
                return self.fail(open.pos, format!("this {{ is never closed. Add a }} where {name} ends."));
            }
            if self.at_word("has") {
                let h = self.next();
                if seen_has {
                    self.diags.add(h.pos, "a class has one has block. Put all its fields in it.");
                } else if !methods.is_empty() {
                    self.diags.add(h.pos, "the has block goes first, above the methods.");
                }
                seen_has = true;
                if self.has_block(&mut fields).is_err() {
                    self.recover();
                }
            } else if self.at_word("method") {
                match self.method_decl() {
                    Ok(m) => methods.push(m),
                    Err(_) => self.recover(),
                }
            } else {
                let pos = self.cur().pos;
                self.diags
                    .add(pos, format!("a {what} holds a has block and methods. Fields go inside has {{ }}."));
                self.recover();
            }
        }
        Ok(ClassDecl {
            name,
            pos,
            is_object,
            is_node,
            is_schema,
            parent,
            fields,
            methods,
        })
    }

    /// route "/api/v1" { get "/user/:id" => getUser; }
    fn route_decl(&mut self) -> R<RouteDecl> {
        let start = self.next();
        let t = self.cur().clone();
        let prefix = match &t.kind {
            Tk::Str(p) => {
                let p = p.clone();
                self.next();
                p
            }
            _ => String::new(),
        };
        let open = self.expect_sym("{", "expected { to start the routes: route \"/api\" { get \"/hello\" => hello; }")?;
        let mut routes = Vec::new();
        loop {
            if self.at_sym("}") {
                self.next();
                break;
            }
            if self.at_eof() {
                return self.fail(open.pos, "this route's { is never closed.");
            }
            let w = self.cur().clone();
            let method = match &w.kind {
                Tk::Ident(m) if matches!(m.as_str(), "get" | "post" | "put" | "delete") => m.to_uppercase(),
                _ => {
                    self.diags
                        .add(w.pos, "a route answers get, post, put or delete: get \"/user/:id\" => getUser;");
                    self.recover();
                    continue;
                }
            };
            self.next();
            let p = self.cur().clone();
            let Tk::Str(path) = &p.kind else {
                return self.fail(p.pos, "the path goes in quotes: get \"/user/:id\" => getUser;");
            };
            let path = path.clone();
            self.next();
            self.expect_sym(
                "=>",
                &format!(
                    "after the path comes => and the method that answers: {} \"{path}\" => answer;",
                    method.to_lowercase()
                ),
            )?;
            let (handler, hpos) = self.ident("the method that answers")?;
            self.semi()?;
            let full = format!("{}/{}", prefix.trim_end_matches('/'), path.trim_start_matches('/'));
            routes.push((method, if full.is_empty() { "/".into() } else { full }, handler, hpos));
        }
        Ok(RouteDecl {
            prefix,
            pos: start.pos,
            routes,
        })
    }

    /// database Shop in "shop.db" { cMap(string name, float price) items; }
    fn database_decl(&mut self) -> R<DatabaseDecl> {
        self.next();
        let (name, pos) = self.ident("the database's name")?;
        check_name(self.diags, &name, pos, NameKind::Type, "database");
        if !self.at_word("in") {
            let p = self.cur().pos;
            return self.fail(
                p,
                format!("say which file it's in: database {name} in \"{}.db\" {{ … }}", name.to_lowercase()),
            );
        }
        self.next();
        let t = self.cur().clone();
        let Tk::Str(path) = &t.kind else {
            return self.fail(
                t.pos,
                format!(
                    "the file's name goes in quotes: database {name} in \"{}.db\" {{ … }}",
                    name.to_lowercase()
                ),
            );
        };
        let path = path.clone();
        self.next();
        let open = self.expect_sym("{", "expected { to start the database's tables.")?;
        let mut tables = Vec::new();
        loop {
            if self.at_sym("}") {
                self.next();
                break;
            }
            if self.at_eof() {
                return self.fail(open.pos, format!("this {{ is never closed. Add a }} where {name} ends."));
            }
            if !self.at_word("cMap") {
                let p = self.cur().pos;
                self.diags.add(
                    p,
                    "a database holds tables, each written like a cMap: cMap(string name, float price) items;",
                );
                self.recover();
                continue;
            }
            let ty = self.ty()?;
            let (tname, tpos) = self.ident("the table's name")?;
            check_name(self.diags, &tname, tpos, NameKind::Value, "table");
            self.semi()?;
            tables.push((ty, tname, tpos));
        }
        Ok(DatabaseDecl { name, pos, path, tables })
    }

    /// assembly LootSpawner { create Item { name = "Potion"; value = 50; } }
    fn assembly_decl(&mut self) -> R<AssemblyDecl> {
        self.next();
        let (name, pos) = self.ident("the assembly's name")?;
        check_name(self.diags, &name, pos, NameKind::Type, "assembly");
        let open = self.expect_sym("{", "expected { to start the assembly's recipes.")?;
        let mut recipes = Vec::new();
        loop {
            if self.at_sym("}") {
                self.next();
                break;
            }
            if self.at_eof() {
                return self.fail(open.pos, format!("this {{ is never closed. Add a }} where {name} ends."));
            }
            if !self.at_word("create") {
                let pos = self.cur().pos;
                self.diags.add(pos, "an assembly holds recipes: create Item { name = \"Potion\"; }");
                self.recover();
                continue;
            }
            let start = self.next();
            let (class, cpos) = self.ident("the class this recipe makes")?;
            self.expect_sym(
                "{",
                &format!("expected {{ to start the recipe: create {class} {{ name = \"Potion\"; }}"),
            )?;
            let mut inits = Vec::new();
            while !self.at_sym("}") {
                if self.at_eof() {
                    return self.fail(start.pos, "this recipe's { is never closed.");
                }
                let (f, fpos) = self.ident("a field's name")?;
                self.expect_sym("=", &format!("a recipe sets fields: {f} = …;"))?;
                let v = self.expr()?;
                self.semi()?;
                inits.push((f, fpos, v));
            }
            self.next();
            let made = self.mk(
                start.pos,
                ExprKind::New {
                    class: class.clone(),
                    class_pos: cpos,
                    inits,
                },
            );
            recipes.push((class, cpos, made));
        }
        Ok(AssemblyDecl { name, pos, recipes })
    }

    fn has_block(&mut self, fields: &mut Vec<FieldDecl>) -> R<()> {
        let open = self.expect_sym("{", "expected { after has.")?;
        loop {
            if self.at_sym("}") {
                self.next();
                return Ok(());
            }
            if self.at_eof() {
                return self.fail(open.pos, "this has { is never closed.");
            }
            if self.field().is_err() {
                self.recover();
                continue;
            }
            fields.push(self.last_field.take().unwrap());
        }
    }

    fn field(&mut self) -> R<()> {
        // state is GameState.Idle;
        if self.at_word("state") && self.peek(1).is_word("is") {
            let t = self.next();
            self.next();
            let opt = self.expr()?;
            self.semi()?;
            self.last_field = Some(FieldDecl {
                ty: None,
                name: "state".into(),
                pos: t.pos,
                default: Some(opt),
            });
            return Ok(());
        }
        if matches!(self.cur().kind, Tk::Ident(_)) && self.peek(1).is_sym("[") && !self.at_word("cMap") {
            let pos = self.cur().pos;
            return self.fail(pos, "v2 writes fields like variables: int hp = 20;");
        }
        let ty = self.ty()?;
        let (name, pos) = self.ident("the field's name")?;
        check_name(self.diags, &name, pos, NameKind::Value, "field");
        let mut default = None;
        if self.at_sym("=") {
            self.next();
            default = Some(self.expr()?);
        }
        self.semi()?;
        self.last_field = Some(FieldDecl {
            ty: Some(ty),
            name,
            pos,
            default,
        });
        Ok(())
    }

    fn method_decl(&mut self) -> R<MethodDecl> {
        self.next(); // method
        let mut ret = None;
        if !(matches!(self.cur().kind, Tk::Ident(_)) && self.peek(1).is_sym("(")) {
            if self.at_word("void") {
                let pos = self.cur().pos;
                return self.fail(pos, "Lust has no void. Leave the return type out when nothing comes back.");
            }
            let t = self.ty()?;
            if self.at_sym("{") {
                let end = self.prev_end();
                return self.fail(end, format!("expected ( after {t}, even when it takes nothing: method {t}() {{ }}"));
            }
            ret = Some(t);
        }
        let (name, pos) = self.ident("the method's name")?;
        check_name(self.diags, &name, pos, NameKind::Value, "method");
        self.expect_sym(
            "(",
            &format!("expected ( after {name}, even when it takes nothing: method {name}() {{ }}"),
        )?;
        let mut params = Vec::new();
        while !self.at_sym(")") {
            let ty = self.ty()?;
            let (pn, pp) = self.ident("the parameter's name")?;
            check_name(self.diags, &pn, pp, NameKind::Value, "parameter");
            if params.iter().any(|p: &Param| p.name == pn) {
                self.diags.add(pp, format!("there's already a parameter called {pn}."));
            }
            params.push(Param { ty, name: pn, pos: pp });
            if self.at_sym(",") {
                self.next();
            } else if !self.at_sym(")") {
                let pos = self.cur().pos;
                return self.fail(pos, "expected , or ) here.");
            }
        }
        self.next();
        let body = self.block("the method")?;
        Ok(MethodDecl {
            name,
            pos,
            ret,
            params,
            body,
        })
    }

    // ----- types -----

    fn ty(&mut self) -> R<TypeRef> {
        let t = self.cur().clone();
        let mut base = match &t.kind {
            Tk::Ident(w) if w == "hook" => {
                self.next();
                self.hook_type()?
            }
            Tk::Ident(w) if w == "cMap" => {
                self.next();
                self.expect_sym(
                    "(",
                    "a cMap lists its columns, its key first: cMap(int id, string itemName, int amount)",
                )?;
                let mut cols = Vec::new();
                while !self.at_sym(")") {
                    let ty = self.ty()?;
                    let (name, pos) = self.ident("the column's name")?;
                    check_name(self.diags, &name, pos, NameKind::Value, "column");
                    let mut maker = None;
                    if self.at_sym("=") {
                        let at = self.cur().pos;
                        self.next();
                        let e = self.expr()?;
                        if cols.is_empty() {
                            maker = Some(e);
                        } else {
                            self.diags.add(
                                at,
                                "only the key (the first column) can have a key-maker: cMap(int id = rollId, string item)",
                            );
                        }
                    }
                    cols.push(Column { ty, name, pos, maker });
                    if self.at_sym(",") {
                        self.next();
                    } else if !self.at_sym(")") {
                        let pos = self.cur().pos;
                        return self.fail(pos, "expected , or ) here.");
                    }
                }
                self.next();
                TypeRef::CMap(cols)
            }
            Tk::Ident(w) => {
                if let Some(h) = hint_for(w) {
                    if matches!(
                        w.as_str(),
                        "bool" | "boolean" | "double" | "str" | "void" | "var" | "let" | "const" | "auto"
                    ) {
                        return self.fail(t.pos, h);
                    }
                }
                if w == "array" || w == "map" {
                    return self.fail(t.pos, format!("say what the {w} holds first: int {w} scores = [];"));
                }
                self.next();
                TypeRef::Named(w.clone(), t.pos)
            }
            _ => {
                return self.fail(
                    t.pos,
                    format!("expected a type here, like int or string, but found {}.", t.describe()),
                )
            }
        };
        loop {
            if self.at_word("array") {
                self.next();
                base = TypeRef::Array(Box::new(base));
            } else if self.at_word("map") {
                self.next();
                base = TypeRef::Map(Box::new(base));
            } else {
                return Ok(base);
            }
        }
    }

    fn hook_type(&mut self) -> R<TypeRef> {
        let mut ret = None;
        // hook roll = … / hook onClose; take nothing and give back nothing.
        let bare = matches!(self.cur().kind, Tk::Ident(_)) && (self.peek(1).is_sym("=") || self.peek(1).is_sym(";"));
        if !self.at_sym("(") && !bare {
            ret = Some(Box::new(self.ty_no_hook()?));
        }
        let mut params = Vec::new();
        if self.at_sym("(") {
            self.next();
            while !self.at_sym(")") {
                let ty = self.ty()?;
                let mut name = None;
                if let Tk::Ident(n) = &self.cur().kind {
                    if !RESERVED.contains(&n.as_str()) {
                        let n = n.clone();
                        let pos = self.next().pos;
                        check_name(self.diags, &n, pos, NameKind::Value, "parameter");
                        name = Some(n);
                    }
                }
                params.push(HookParam { ty, name });
                if self.at_sym(",") {
                    self.next();
                } else if !self.at_sym(")") {
                    let pos = self.cur().pos;
                    return self.fail(pos, "expected , or ) here.");
                }
            }
            self.next();
        }
        Ok(TypeRef::Hook { ret, params })
    }

    fn ty_no_hook(&mut self) -> R<TypeRef> {
        if self.at_word("hook") {
            let pos = self.cur().pos;
            return self.fail(pos, "a hook can't give back another hook yet.");
        }
        self.ty()
    }

    /// Does a declaration start here? `int x`, `Player array team`, `cMap(…) bag`, `hook … f`.
    fn looks_like_decl(&self) -> bool {
        if self.at_word("hook") || self.at_word("cMap") {
            return true;
        }
        if !matches!(self.cur().kind, Tk::Ident(_)) {
            return false;
        }
        let mut n = 1;
        while self.peek(n).is_word("array") || self.peek(n).is_word("map") {
            n += 1;
        }
        matches!(&self.peek(n).kind, Tk::Ident(w) if !RESERVED.contains(&w.as_str()) || w == "state")
            && (self.peek(n + 1).is_sym("=") || self.peek(n + 1).is_sym(";"))
            && !self.at_word("state")
    }

    // ----- statements -----

    fn block(&mut self, what: &str) -> R<Vec<Stmt>> {
        let open = self.expect_sym("{", &format!("expected {{ to start {what}."))?;
        let mut out = Vec::new();
        loop {
            if self.at_sym("}") {
                self.next();
                return Ok(out);
            }
            if self.at_eof() || (self.at_word("end") && self.peek(1).kind == Tk::Eof) {
                return self.fail(open.pos, "this { is never closed. Add a } where the block ends.");
            }
            match self.statement() {
                Ok(s) => out.push(s),
                Err(_) => self.recover(),
            }
        }
    }

    fn statement(&mut self) -> R<Stmt> {
        let t = self.cur().clone();
        if let Tk::Ident(w) = &t.kind {
            match w.as_str() {
                "is" => return self.if_chain(),
                "loop" => return self.loop_stmt(),
                "stop" | "skip" => {
                    self.next();
                    if self.loop_depth == 0 {
                        self.diags.add(t.pos, format!("{w} only works inside a loop."));
                    }
                    self.semi()?;
                    return Ok(if w == "stop" { Stmt::Stop(t.pos) } else { Stmt::Skip(t.pos) });
                }
                "return" => {
                    self.next();
                    let value = if self.at_sym(";") || self.cur().pos.line > t.pos.line {
                        None
                    } else {
                        Some(self.expr()?)
                    };
                    self.semi()?;
                    return Ok(Stmt::Return(value, t.pos));
                }
                "try" => return self.try_stmt(),
                "save" | "load" if matches!(self.peek(1).kind, Tk::Ident(_)) => {
                    self.next();
                    let (name, npos) = self.ident("the save's name")?;
                    if w == "save" && self.at_sym("=") {
                        check_name(self.diags, &name, npos, NameKind::Value, "save");
                        self.next();
                        self.expect_sym("[", "a save lists what it holds: save gameSession = [Farm, player, currentDay];")?;
                        let mut items = Vec::new();
                        while !self.at_sym("]") {
                            items.push(self.ident("something to save")?);
                            if self.at_sym(",") {
                                self.next();
                            } else if !self.at_sym("]") {
                                let pos = self.cur().pos;
                                return self.fail(pos, "expected , or ] here.");
                            }
                        }
                        self.next();
                        self.semi()?;
                        return Ok(Stmt::SaveDecl { name, items, pos: t.pos });
                    }
                    self.semi()?;
                    return Ok(if w == "save" {
                        Stmt::Save(name, t.pos)
                    } else {
                        Stmt::Load(name, t.pos)
                    });
                }
                "keep" => {
                    self.next();
                    let value = self.expr()?;
                    let into = if self.at_word("in") {
                        self.next();
                        Some(self.expr()?)
                    } else {
                        None
                    };
                    self.semi()?;
                    return Ok(Stmt::Keep { value, into, pos: t.pos });
                }
                // wait 0.5; is a built-in that takes its value without ( ).
                "wait" if !self.peek(1).is_sym("(") => {
                    self.next();
                    let secs = self.expr()?;
                    self.semi()?;
                    return Ok(Stmt::Expr(self.mk(t.pos, ExprKind::Call("wait".into(), vec![secs]))));
                }
                "lua" => {
                    self.next();
                    let code = match self.cur().kind.clone() {
                        Tk::LuaCode(c) => {
                            self.next();
                            c
                        }
                        _ => return self.fail(t.pos, "expected { after lua: lua { … }"),
                    };
                    if !self.is_script {
                        self.diags.add(t.pos, "lua { } only works in LustScript (.lsc) files.");
                    } else if !self.uses_lua {
                        self.diags.add(t.pos, "add fetch lua; at the top of the file to use lua { }.");
                    }
                    if self.at_sym(";") {
                        self.next();
                    }
                    return Ok(Stmt::Lua { code, pos: t.pos });
                }
                "method" => {
                    return self.fail(
                        t.pos,
                        "a method can't go inside another one. Put it at the top level, or in a class.",
                    )
                }
                "class" | "object" => return self.fail(t.pos, format!("a {w} goes at the top level of a file, not inside a block.")),
                "state" if matches!(self.peek(1).kind, Tk::Ident(_)) && self.peek(2).is_sym("=") => {
                    return self.fail(t.pos, "a state is declared at the top level of a file.")
                }
                "end" => return self.fail(t.pos, "end only goes at the very end of a file. Blocks close with }."),
                "fetch" | "get" | "pass" => return self.fail(t.pos, format!("{w} lines go at the top level of a file.")),
                _ => {
                    if let Some(h) = hint_for(w) {
                        // `do`, `if` and friends at the start of a line are other languages' words.
                        let next = &self.peek(1);
                        let used_as_name = next.is_sym("=") || next.is_sym(".") || next.is_sym(":") || next.is_sym("[");
                        if !used_as_name || matches!(w.as_str(), "self" | "this") {
                            return self.fail(t.pos, h);
                        }
                    }
                }
            }
        }
        if self.looks_like_decl() {
            let ty = self.ty()?;
            let (name, pos) = self.ident("the variable's name")?;
            check_name(self.diags, &name, pos, NameKind::Value, "variable");
            if self.at_sym(";") {
                return self.fail(pos, format!("{name} needs a starting value: {ty} {name} = …;"));
            }
            self.expect_sym("=", &format!("expected = after {name}."))?;
            let value = self.expr()?;
            self.semi()?;
            return Ok(Stmt::Var { ty, name, pos, value });
        }
        let target = self.expr()?;
        if self.at_sym("=") {
            self.next();
            if !matches!(target.kind, ExprKind::Name(_) | ExprKind::Field(..) | ExprKind::Index(..)) {
                return self.fail(t.pos, "the left side of = must be something that can change, like hp or player.hp.");
            }
            let value = self.expr()?;
            self.semi()?;
            return Ok(Stmt::Assign { target, value, pos: t.pos });
        }
        if self.at_word("is") && self.cur().pos.line == self.prev_end().line {
            self.next();
            let option = self.expr()?;
            self.semi()?;
            return Ok(Stmt::SetState {
                target,
                option,
                pos: t.pos,
            });
        }
        if let ExprKind::MethodRef(_, m, _) = &target.kind {
            let m = m.clone();
            return self.fail(t.pos, format!("add () to call it: {m}()"));
        }
        if !matches!(target.kind, ExprKind::Call(..) | ExprKind::Method(..)) {
            if let ExprKind::Name(n) = &target.kind {
                if self.cur().pos.line == t.pos.line && matches!(self.cur().kind, Tk::Ident(_)) {
                    let pos = self.cur().pos;
                    return self.fail(pos, format!("Lust doesn't know a type called {n}."));
                }
            }
            return self.fail(
                t.pos,
                "this line doesn't do anything. A line can declare, change something, call something, ask a question or loop.",
            );
        }
        self.semi()?;
        Ok(Stmt::Expr(target))
    }

    fn question(&mut self) -> R<Expr> {
        let check = self.expr()?;
        if self.at_sym("=") {
            let pos = self.cur().pos;
            return self.fail(pos, "questions compare with ==, not =: is hp == 0?");
        }
        if !self.at_sym("?") {
            let at = if self.cur().pos.line > self.prev_end().line {
                self.prev_end()
            } else {
                self.cur().pos
            };
            let msg = if self.at_word("then") {
                "a question ends with ? before then: is hp == 0? then { }"
            } else if self.at_word("and") {
                "Lust joins checks with &&: is a > 1 && b > 2?"
            } else {
                "a question ends with ?, like is hp == 0? then { }"
            };
            return self.fail(at, msg);
        }
        self.next();
        Ok(check)
    }

    fn if_chain(&mut self) -> R<Stmt> {
        let start = self.next().pos; // is
        let mut branches = Vec::new();
        let check = self.question()?;
        if !self.at_word("then") {
            let pos = self.cur().pos;
            return self.fail(pos, "expected then after the question: is hp == 0? then { }");
        }
        self.next();
        let body = self.block("the answer")?;
        branches.push((check, body));
        let mut otherwise = None;
        while self.at_word("or") {
            self.next();
            if self.at_word("is") {
                self.next();
                let c = self.question()?;
                if !self.at_word("then") {
                    let pos = self.cur().pos;
                    return self.fail(pos, "expected then after the question.");
                }
                self.next();
                let b = self.block("the answer")?;
                branches.push((c, b));
            } else {
                otherwise = Some(self.block("the other case")?);
                break;
            }
        }
        if self.at_sym(";") {
            self.next();
        }
        Ok(Stmt::If {
            branches,
            otherwise,
            pos: start,
        })
    }

    fn loop_stmt(&mut self) -> R<Stmt> {
        let start = self.next().pos; // loop
        let kind = if self.at_sym("{") {
            LoopKind::Forever
        } else if self.at_sym("(") {
            self.next();
            let each = self.looks_like_each();
            let k = if each {
                let mut vars = Vec::new();
                loop {
                    let ty = self.ty()?;
                    let (name, pos) = self.ident("the loop's variable")?;
                    check_name(self.diags, &name, pos, NameKind::Value, "variable");
                    vars.push((ty, name, pos));
                    if self.at_sym(",") {
                        self.next();
                        continue;
                    }
                    break;
                }
                if !self.at_word("in") {
                    let pos = self.cur().pos;
                    return self.fail(pos, "expected in: loop (int score in scores) { }");
                }
                self.next();
                let source = self.expr()?;
                LoopKind::Each { vars, source }
            } else {
                LoopKind::While(self.expr()?)
            };
            self.expect_sym(")", "expected ) to close the loop's ( ).")?;
            k
        } else {
            let times = self.expr()?;
            if self.at_word("times") {
                self.next();
            }
            LoopKind::Times(times)
        };
        self.loop_depth += 1;
        let body = self.block("the loop");
        self.loop_depth -= 1;
        Ok(Stmt::Loop {
            kind,
            body: body?,
            pos: start,
        })
    }

    fn looks_like_each(&self) -> bool {
        let mut depth = 0;
        let mut n = 0;
        loop {
            let t = self.peek(n);
            match &t.kind {
                Tk::Eof => return false,
                Tk::Sym("(") => depth += 1,
                Tk::Sym(")") => {
                    if depth == 0 {
                        return false;
                    }
                    depth -= 1
                }
                Tk::Ident(w) if w == "in" && depth == 0 => return true,
                Tk::Sym("{") => return false,
                _ => {}
            }
            n += 1;
        }
    }

    fn try_stmt(&mut self) -> R<Stmt> {
        let start = self.next().pos;
        let body = self.block("the try")?;
        if !self.at_word("error") {
            let pos = self.cur().pos;
            return self.fail(pos, "a try needs an error part: try { … } error (string message) { … }");
        }
        self.next();
        self.expect_sym("(", "expected ( after error: error (string message) { }")?;
        if !self.at_word("string") {
            let pos = self.cur().pos;
            return self.fail(pos, "the error is text: error (string message) { }");
        }
        self.next();
        let (name, pos) = self.ident("a name for the error")?;
        check_name(self.diags, &name, pos, NameKind::Value, "variable");
        self.expect_sym(")", "expected ) here.")?;
        let handler = self.block("the error part")?;
        Ok(Stmt::Try {
            body,
            err: (name, pos),
            handler,
            pos: start,
        })
    }

    // ----- expressions -----

    fn expr(&mut self) -> R<Expr> {
        if self.at_word("either") {
            let t = self.next();
            let check = self.or_expr()?;
            if !self.at_sym("?") {
                let at = self.prev_end();
                return self.fail(
                    at,
                    "either's question ends with ? before the answers: either hp > 0? \"up\" or \"down\"",
                );
            }
            self.next();
            let yes = self.expr()?;
            if !self.at_word("or") {
                let at = self.prev_end();
                return self.fail(at, "either needs an or: either hp > 0? \"up\" or \"down\"");
            }
            self.next();
            let no = self.expr()?;
            return Ok(self.mk(t.pos, ExprKind::Either(Box::new(check), Box::new(yes), Box::new(no))));
        }
        self.or_expr()
    }

    fn or_expr(&mut self) -> R<Expr> {
        let mut left = self.and_expr()?;
        while self.at_sym("||") {
            let op = self.next();
            let right = self.and_expr()?;
            left = self.mk(left.pos, ExprKind::Bin(BinOp::Or, Box::new(left), Box::new(right), op.pos));
        }
        Ok(left)
    }

    fn and_expr(&mut self) -> R<Expr> {
        let mut left = self.not_expr()?;
        while self.at_sym("&&") {
            let op = self.next();
            let right = self.not_expr()?;
            left = self.mk(left.pos, ExprKind::Bin(BinOp::And, Box::new(left), Box::new(right), op.pos));
        }
        Ok(left)
    }

    fn not_expr(&mut self) -> R<Expr> {
        if self.at_word("not") {
            let t = self.next();
            let inner = self.not_expr()?;
            return Ok(self.mk(t.pos, ExprKind::Not(Box::new(inner))));
        }
        self.compare()
    }

    fn compare(&mut self) -> R<Expr> {
        let left = self.sum()?;
        if self.at_word("like") {
            let t = self.next();
            let (cls, pos) = self.ident("a class name")?;
            return Ok(self.mk(t.pos, ExprKind::Like(Box::new(left), cls, pos)));
        }
        let op = match &self.cur().kind {
            Tk::Sym("==") => BinOp::Eq,
            Tk::Sym("!=") => BinOp::Ne,
            Tk::Sym("===") => BinOp::Same,
            Tk::Sym("!==") => BinOp::NotSame,
            Tk::Sym("<") => BinOp::Lt,
            Tk::Sym(">") => BinOp::Gt,
            Tk::Sym("<=") => BinOp::Le,
            Tk::Sym(">=") => BinOp::Ge,
            _ => return Ok(left),
        };
        let t = self.next();
        let right = self.sum()?;
        if matches!(&self.cur().kind, Tk::Sym(s) if ["==", "!=", "===", "!==", "<", ">", "<=", ">="].contains(s)) {
            let pos = self.cur().pos;
            return self.fail(pos, "one comparison at a time. Join them with &&: a < b && b < c");
        }
        Ok(self.mk(left.pos, ExprKind::Bin(op, Box::new(left), Box::new(right), t.pos)))
    }

    fn sum(&mut self) -> R<Expr> {
        let mut left = self.product()?;
        loop {
            let op = if self.at_sym("+") {
                BinOp::Add
            } else if self.at_sym("-") {
                BinOp::Sub
            } else {
                return Ok(left);
            };
            let t = self.next();
            let right = self.product()?;
            left = self.mk(left.pos, ExprKind::Bin(op, Box::new(left), Box::new(right), t.pos));
        }
    }

    fn product(&mut self) -> R<Expr> {
        let mut left = self.unary()?;
        loop {
            let op = if self.at_sym("*") {
                BinOp::Mul
            } else if self.at_sym("/") {
                BinOp::Div
            } else if self.at_sym("%") {
                BinOp::Mod
            } else {
                return Ok(left);
            };
            let t = self.next();
            let right = self.unary()?;
            left = self.mk(left.pos, ExprKind::Bin(op, Box::new(left), Box::new(right), t.pos));
        }
    }

    fn unary(&mut self) -> R<Expr> {
        if self.at_sym("-") {
            let t = self.next();
            let inner = self.unary()?;
            return Ok(match inner.kind {
                ExprKind::Int(v) => self.mk(t.pos, ExprKind::Int(-v)),
                ExprKind::Float(v) => self.mk(t.pos, ExprKind::Float(-v)),
                _ => self.mk(t.pos, ExprKind::Neg(Box::new(inner))),
            });
        }
        if self.at_sym("#") {
            let t = self.next();
            let inner = self.postfix()?;
            return Ok(self.mk(t.pos, ExprKind::Len(Box::new(inner))));
        }
        self.postfix()
    }

    fn postfix(&mut self) -> R<Expr> {
        let mut e = self.primary()?;
        loop {
            if self.at_sym(".") {
                self.next();
                let (name, pos) = self.ident("a field's name after .")?;
                if self.at_sym("(") {
                    if let Some((_, conv)) = CONVERSIONS.iter().find(|(n, _)| *n == name) {
                        self.next();
                        self.expect_sym(")", &format!(".{name}() takes nothing in its ( )."))?;
                        e = self.mk(e.pos, ExprKind::Convert(Box::new(e), *conv, pos));
                        continue;
                    }
                    return self.fail(pos, format!("methods are called with : in Lust: thing:{name}()"));
                }
                e = self.mk(e.pos, ExprKind::Field(Box::new(e), name, pos));
            } else if self.at_sym(":") {
                self.next();
                let (name, pos) = self.ident("a method's name after :")?;
                if self.at_sym("(") {
                    let args = self.args()?;
                    e = self.mk(e.pos, ExprKind::Method(Box::new(e), name, args, pos));
                } else {
                    e = self.mk(e.pos, ExprKind::MethodRef(Box::new(e), name, pos));
                }
            } else if self.at_sym("[") && !self.cur_space_before() {
                self.next();
                let index = self.expr()?;
                self.expect_sym("]", "expected ] here.")?;
                e = self.mk(e.pos, ExprKind::Index(Box::new(e), Box::new(index)));
            } else {
                return Ok(e);
            }
        }
    }

    fn cur_space_before(&self) -> bool {
        self.p > 0 && self.toks[self.p - 1].end != self.cur().pos
    }

    fn args(&mut self) -> R<Vec<Expr>> {
        self.next(); // (
        let mut args = Vec::new();
        while !self.at_sym(")") {
            args.push(self.expr()?);
            if self.at_sym(",") {
                self.next();
            } else if !self.at_sym(")") {
                let at = if self.cur().pos.line > self.prev_end().line {
                    self.prev_end()
                } else {
                    self.cur().pos
                };
                return self.fail(at, "expected , or ) here.");
            }
        }
        self.next();
        Ok(args)
    }

    fn primary(&mut self) -> R<Expr> {
        let t = self.cur().clone();
        match &t.kind {
            Tk::Int(v) => {
                self.next();
                Ok(self.mk(t.pos, ExprKind::Int(*v)))
            }
            Tk::Float(v, _) => {
                self.next();
                Ok(self.mk(t.pos, ExprKind::Float(*v)))
            }
            Tk::Str(s) => {
                self.next();
                Ok(self.mk(t.pos, ExprKind::Str(s.clone())))
            }
            Tk::Sym("(") => {
                self.next();
                let e = self.expr()?;
                self.expect_sym(")", "expected ) here.")?;
                Ok(e)
            }
            Tk::Sym("[") => self.list(),
            Tk::Ident(w) => {
                let w = w.clone();
                match w.as_str() {
                    "true" | "false" => {
                        self.next();
                        Ok(self.mk(t.pos, ExprKind::Bool(w == "true")))
                    }
                    "nothing" => {
                        self.next();
                        Ok(self.mk(t.pos, ExprKind::Nothing))
                    }
                    "new" => self.new_expr(),
                    "saved" => {
                        self.next();
                        let (name, _) = self.ident("the save's name: saved gameSession")?;
                        Ok(self.mk(t.pos, ExprKind::Saved(name)))
                    }
                    "null" | "nil" | "None" => self.fail(t.pos, "Lust calls it nothing."),
                    "self" | "this" => {
                        let h = if self.peek(1).is_sym(".") {
                            if let Tk::Ident(f) = &self.peek(2).kind {
                                format!("inside a method, use a field's name on its own: {f}, not {w}.{f}.")
                            } else {
                                hint_for(&w).unwrap().to_string()
                            }
                        } else {
                            hint_for(&w).unwrap().to_string()
                        };
                        self.fail(t.pos, h)
                    }
                    _ if RESERVED.contains(&w.as_str()) && w != "state" => {
                        self.fail(t.pos, format!("expected a value here, but found the word {w}."))
                    }
                    _ => {
                        self.next();
                        if self.at_sym("(") && !self.cur_space_before() || self.at_sym("(") && BUILTINS.contains(&w.as_str()) {
                            let args = self.args()?;
                            return Ok(self.mk(t.pos, ExprKind::Call(w, args)));
                        }
                        Ok(self.mk(t.pos, ExprKind::Name(w)))
                    }
                }
            }
            _ => {
                let at = if t.pos.line > self.prev_end().line {
                    self.prev_end()
                } else {
                    t.pos
                };
                self.fail(at, format!("expected a value here, but found {}.", t.describe()))
            }
        }
    }

    fn list(&mut self) -> R<Expr> {
        let open = self.next();
        let mut items = Vec::new();
        let mut keys: Option<Vec<Expr>> = None;
        while !self.at_sym("]") {
            if self.at_eof() {
                return self.fail(open.pos, "this [ is never closed.");
            }
            let first = self.expr()?;
            if !self.at_sym(",") && !self.at_sym("]") {
                // ["apple" 3]: a key and its value.
                if keys.is_none() && !items.is_empty() {
                    let pos = self.cur().pos;
                    return self.fail(pos, "this looks like a key and its value, but the items before it are plain. A map writes every item as key value: [\"apple\" 3, \"seed\" 1]");
                }
                let value = self.expr()?;
                keys.get_or_insert_with(Vec::new).push(first);
                items.push(value);
            } else {
                if keys.is_some() {
                    let pos = first.pos;
                    return self.fail(pos, "in a map every item is a key and a value: \"apple\" 3");
                }
                items.push(first);
            }
            if self.at_sym(",") {
                self.next();
            } else if !self.at_sym("]") {
                let pos = self.cur().pos;
                return self.fail(pos, "expected , or ] here.");
            }
        }
        self.next();
        Ok(self.mk(open.pos, ExprKind::List { items, keys }))
    }

    fn new_expr(&mut self) -> R<Expr> {
        let start = self.next().pos; // new
        if self.at_word("hook") {
            self.next();
            let body = self.block("the hook")?;
            return Ok(self.mk(start, ExprKind::NewHook(body)));
        }
        let (class, class_pos) = self.ident("what to make: new Player")?;
        let mut inits = Vec::new();
        if self.at_sym("(") {
            return self.fail(
                self.cur().pos,
                format!("Lust fills in fields with {{ }}: new {class} {{ name = \"Ada\" }}"),
            );
        }
        if self.at_sym("{") {
            self.next();
            while !self.at_sym("}") {
                let (name, pos) = self.ident("a field's name")?;
                let value = if self.at_word("is") && name == "state" {
                    self.next();
                    self.expr()?
                } else {
                    if !self.at_sym("=") {
                        let at = self.cur().pos;
                        return self.fail(at, format!("expected = after {name}: new {class} {{ {name} = … }}"));
                    }
                    self.next();
                    self.expr()?
                };
                if inits.iter().any(|(n, _, _): &(String, Pos, Expr)| *n == name) {
                    self.diags.add(pos, format!("{name} is already filled in here."));
                }
                inits.push((name, pos, value));
                if self.at_sym(",") || self.at_sym(";") {
                    self.next();
                } else if !self.at_sym("}") {
                    let pos = self.cur().pos;
                    return self.fail(pos, "expected , or } here.");
                }
            }
            self.next();
        }
        Ok(self.mk(start, ExprKind::New { class, class_pos, inits }))
    }
}
