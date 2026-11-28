//! Named values (`spawn(x = 3, y = 5)`, `send Wants(jump = true, fire = false)`): once a
//! program is checked, each call's values are put in the method's or event's own order and
//! their names dropped, so the writers see an ordinary call and a named value costs nothing
//! when the program runs. The checker has already said where each value goes (`Info::named`),
//! and made sure the new order can't change what the values give.

use crate::ast::*;
use crate::check::Info;

/// Puts every named value in its place, everywhere in the program.
pub fn place(prog: &mut Program, info: &Info) {
    let mut p = Placer { info };
    for item in &mut prog.items {
        p.item(item);
    }
    p.stmts(&mut prog.body);
}

struct Placer<'a> {
    info: &'a Info,
}

impl Placer<'_> {
    /// One call's values: in the method's order, without their names.
    fn args(&mut self, args: &mut Vec<Expr>) {
        let order = args.first().and_then(|a| self.info.named.get(&a.id)).cloned();
        for a in args.iter_mut() {
            self.expr(a);
        }
        let mut values: Vec<Option<Expr>> = std::mem::take(args)
            .into_iter()
            .map(|a| match a.kind {
                ExprKind::Named(_, _, v) => Some(*v),
                _ => Some(a),
            })
            .collect();
        *args = match order {
            Some(order) if order.len() == values.len() => order.iter().filter_map(|&w| values[w].take()).collect(),
            _ => values.into_iter().flatten().collect(),
        };
    }

    fn item(&mut self, item: &mut Item) {
        match item {
            Item::Class(c) => {
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
            Item::Method(m) => self.method(m),
            Item::Assembly(a) => {
                for (_, _, e) in &mut a.recipes {
                    self.expr(e);
                }
            }
            Item::Database(d) => {
                for (t, _, _) in &mut d.tables {
                    self.ty(t);
                }
            }
            Item::Style(s) => self.styles(&mut s.items),
            Item::State(s) => self.options(&mut s.options),
            Item::Route(_) | Item::Event(_) => {}
        }
    }

    fn options(&mut self, options: &mut [StateOption]) {
        for o in options {
            for (_, _, e) in &mut o.data {
                self.expr(e);
            }
            self.options(&mut o.children);
        }
    }

    fn method(&mut self, m: &mut MethodDecl) {
        for p in &mut m.params {
            self.ty(&mut p.ty);
        }
        if let Some(r) = &mut m.ret {
            self.ty(r);
        }
        self.stmts(&mut m.body);
    }

    fn ty(&mut self, t: &mut TypeRef) {
        match t {
            TypeRef::Named(..) => {}
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

    fn styles(&mut self, items: &mut [StyleItem]) {
        for it in items {
            match &mut it.value {
                StyleValue::Expr(e) => self.expr(e),
                StyleValue::Nested(n) => self.styles(n),
                StyleValue::Flag | StyleValue::Word(_) => {}
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
            Stmt::Assign { target, value, .. } => {
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
                    LoopKind::Count { init, cond, step } => {
                        self.stmt(init);
                        self.expr(cond);
                        self.stmt(step);
                    }
                    LoopKind::Forever => {}
                }
                self.stmts(body);
            }
            Stmt::Return(e, _) => {
                if let Some(e) = e {
                    self.expr(e);
                }
            }
            Stmt::Try { body, handler, .. } => {
                self.stmts(body);
                self.stmts(handler);
            }
            Stmt::Start { call, .. } => self.expr(call),
            Stmt::Cancel { work, .. } => self.expr(work),
            Stmt::Wait {
                seconds,
                questions,
                check,
                limit,
                ..
            } => {
                if let Some(e) = seconds {
                    self.expr(e);
                }
                for q in questions {
                    self.expr(q);
                }
                if let Some(c) = check {
                    self.stmt(c);
                }
                if let Some((l, b)) = limit {
                    self.expr(l);
                    self.stmts(b);
                }
            }
            Stmt::Keep { value, into, .. } => {
                self.expr(value);
                if let Some(i) = into {
                    self.expr(i);
                }
            }
            Stmt::Tween {
                place,
                target,
                secs,
                ease,
                done,
                ..
            } => {
                for e in [place, target, secs] {
                    self.expr(e);
                }
                for e in [ease, done].into_iter().flatten() {
                    self.expr(e);
                }
            }
            Stmt::Send { args, to, .. } => {
                self.args(args);
                if let Some(t) = to {
                    self.expr(t);
                }
            }
            Stmt::Element {
                arg,
                style,
                children,
                action,
                enter,
                choices,
                extra,
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
                for a in extra.iter_mut() {
                    if let Some((t, _, _)) = &mut a.param {
                        self.ty(t);
                    }
                    self.stmts(&mut a.body);
                }
            }
            Stmt::Stop(_) | Stmt::Skip(_) | Stmt::Lua { .. } | Stmt::SaveDecl { .. } | Stmt::Save(..) | Stmt::Load(..) => {}
        }
    }

    fn expr(&mut self, e: &mut Expr) {
        match &mut e.kind {
            ExprKind::Call(_, args) => self.args(args),
            ExprKind::Method(t, _, args, _) => {
                self.expr(t);
                self.args(args);
            }
            ExprKind::Field(t, ..)
            | ExprKind::MethodRef(t, ..)
            | ExprKind::Len(t)
            | ExprKind::Neg(t)
            | ExprKind::BitNot(t)
            | ExprKind::Not(t)
            | ExprKind::Like(t, ..)
            | ExprKind::Convert(t, ..) => self.expr(t),
            ExprKind::Index(a, b) | ExprKind::Bin(_, a, b, _) | ExprKind::Random(a, b) => {
                self.expr(a);
                self.expr(b);
            }
            ExprKind::Either(a, b, c) => {
                self.expr(a);
                self.expr(b);
                self.expr(c);
            }
            ExprKind::Template(parts) => {
                for p in parts {
                    if let TemplatePart::Value(v) = p {
                        self.expr(v);
                    }
                }
            }
            ExprKind::List { items, keys } => {
                for i in items {
                    self.expr(i);
                }
                for k in keys.iter_mut().flatten() {
                    self.expr(k);
                }
            }
            ExprKind::New { inits, .. } => {
                for (_, _, v) in inits {
                    self.expr(v);
                }
            }
            ExprKind::NewHook(body) => self.stmts(body),
            // Only a call's values are named, and the call takes the name off. One anywhere
            // else is already a mistake, so this never happens in a checked program.
            ExprKind::Named(..) => {
                if let ExprKind::Named(_, _, v) = std::mem::replace(&mut e.kind, ExprKind::Nothing) {
                    *e = *v;
                    self.expr(e);
                }
            }
            ExprKind::Int(_)
            | ExprKind::Float(_)
            | ExprKind::Str(_)
            | ExprKind::Bool(_)
            | ExprKind::Nothing
            | ExprKind::Name(_)
            | ExprKind::Saved(_) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    fn lua(src: &str) -> String {
        let files = vec![("main.lst".to_string(), format!("{src}end\n"))];
        match crate::to_lua("main.lst", &files) {
            Ok(c) => c.code,
            Err(p) => panic!("{}", p.iter().map(|x| x.message.clone()).collect::<Vec<_>>().join("\n")),
        }
    }

    /// The values arrive in the method's order, whatever order they're written in, and the
    /// output is the same as for the call written in order.
    #[test]
    fn named_values_become_a_plain_call() {
        let named = lua("method spawn(int x, int y) { print(x - y); }\nspawn(y = 5, x = 3);\n");
        let plain = lua("method spawn(int x, int y) { print(x - y); }\nspawn(3, 5);\n");
        assert_eq!(named, plain);
    }

    #[test]
    fn named_values_in_a_send_become_a_plain_send() {
        let src = |args: &str| {
            format!(
                "event Wants(bool jump, bool crouch, bool fire);\nobject Body {{\n    on {{\n        Wants(bool jump, bool crouch, bool fire) {{ print(jump, crouch, fire); }}\n    }}\n}}\nsend Wants({args}) to Body;\n"
            )
        };
        assert_eq!(
            lua(&src("fire = true, jump = false, crouch = true")),
            lua(&src("false, true, true"))
        );
    }

    fn mistakes(src: &str) -> Vec<String> {
        let files = vec![("main.lst".to_string(), format!("{src}\nend\n"))];
        crate::check("main.lst", &files)
            .err()
            .unwrap_or_default()
            .into_iter()
            .map(|p| p.message)
            .collect()
    }

    #[test]
    fn assignment_inside_a_call_was_never_anything_else() {
        // `f(a = 1)` used to be a parse mistake, so naming a value takes nothing away. An
        // expression that only looks like a name (a field, a sum) still can't be named.
        let src = "method f(int a) { print(a); }\nint b = 1;\n";
        assert!(mistakes(&format!("{src}f(a = 1);")).is_empty());
        assert!(!mistakes(&format!("{src}f(b + 1 = 1);")).is_empty());
        assert_eq!(mistakes(&format!("{src}f(a = b = 1);")), vec!["expected , or ) here.".to_string()]);
    }

    #[test]
    fn values_that_cant_change_go_anywhere() {
        let src = "object C { has { int n = 0; } method int bump() { n += 1; return n; } }\nmethod f(int a, int b) { print(a, b); }\nmethod g() {\n    int k = 2;\n    LINE\n}\ng();\n";
        for (line, ok) in [
            ("f(b = C:bump(), a = 1);", true),
            ("f(b = C:bump(), a = k + 1);", true),
            ("f(b = k, a = C:bump());", true),
            ("f(a = C:bump(), b = C:bump());", true),
            ("f(b = C:bump(), a = C.n);", false),
            ("f(b = C:bump(), a = C:bump());", false),
            ("f(b = k / 2, a = C.n);", false),
            ("f(b = C.n, a = C.n);", true),
        ] {
            assert_eq!(mistakes(&src.replace("LINE", line)).is_empty(), ok, "{line}");
        }
    }

    #[test]
    fn named_values_fuzzed() {
        // Calls and sends made at random from a fixed seed, with names right, wrong, repeated,
        // missing or mixed, and sometimes broken by a piece dropped or added: checking never
        // panics, and a program that checks writes both outputs.
        let mut seed = 95u64;
        let mut next = |n: usize| {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            ((seed >> 33) % n as u64) as usize
        };
        let names = ["x", "y", "z", "q", "x"];
        let values = ["1", "k", "C:bump()", "C.n", "k / 2", "Random[1, 2]", "true", "-k"];
        let junk = ["(", ")", "=", ",", "x", "=="];
        let head = "event E(int x, int y);\nobject C { has { int n = 0; } on { E(int x, int y) { print(x, y); } } method int bump() { n += 1; return n; } }\nmethod f(int x, int y) { print(x - y); }\nmethod g() {\n    int k = 3;\n";
        let mut fine = 0;
        for _ in 0..1500 {
            let given: Vec<String> = (0..next(4))
                .map(|_| {
                    let v = values[next(values.len())];
                    if next(5) == 0 {
                        v.to_string()
                    } else {
                        format!("{} = {v}", names[next(names.len())])
                    }
                })
                .collect();
            let mut text = match next(3) {
                0 => format!("f({});", given.join(", ")),
                1 => format!("send E({});", given.join(", ")),
                _ => format!("send E({}) to C;", given.join(", ")),
            };
            if next(3) == 0 {
                let at = next(text.len());
                match next(2) {
                    0 => text.insert_str(at, junk[next(junk.len())]),
                    _ => {
                        text.remove(at);
                    }
                }
            }
            let src = format!("{head}    {text}\n}}\ng();\nend\n");
            let f = vec![("main.lst".to_string(), src.clone())];
            if crate::check("main.lst", &f).is_ok() {
                crate::to_lua("main.lst", &f).unwrap();
                crate::to_rust_library("main.lst", &f).unwrap();
                fine += 1;
            }
        }
        assert!(fine > 0, "no program checked");
    }
}
