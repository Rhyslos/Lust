//! Lust's standard library, as the checker sees it: what each part takes and gives back.
//!
//! - Built-in objects: `Maths`, `Files`, `Time` and `Json`, used like any object: `Maths:sqrt(2.0)`.
//! - Methods on text and arrays: `name:contains("a")`, `words:join(", ")`.
//! - `.upper()`, `.lower()` and `.trim()` are conversions, like `.text()`.
//!
//! Each call becomes a runtime function with the same name in Lua (`_L.maths_sqrt`) and in
//! Rust (`rt::maths_sqrt`). The JSON ones are special: the writers build them from the types.

use crate::types::Ty;

pub const OBJECTS: &[&str] = &["Maths", "Files", "Time", "Json", "Web"];

/// One library call: the runtime function, and what it takes and gives back.
#[derive(Clone, Debug)]
pub struct Lib {
    pub func: String,
    pub params: Vec<(&'static str, Ty)>,
    pub ret: Ty,
}

fn lib(func: &str, params: Vec<(&'static str, Ty)>, ret: Ty) -> Result<Lib, String> {
    Ok(Lib {
        func: func.to_string(),
        params,
        ret,
    })
}

fn numeric(args: &[Ty]) -> Option<Ty> {
    if args.contains(&Ty::Float) {
        Some(Ty::Float)
    } else if args.iter().all(|t| *t == Ty::Int || *t == Ty::Error) {
        Some(Ty::Int)
    } else {
        None
    }
}

/// What the methods of each built-in object are, for messages.
pub fn listing(obj: &str) -> &'static str {
    match obj {
        "Maths" => "abs, min, max, clamp, round, floor, ceil, sqrt, power, sin, cos and tan, and Maths.pi",
        "Files" => "read, write, append, exists, remove, list and makeFolder",
        "Time" => "now, clock, date, time, addDays, daysBetween, weekday and isDate",
        "Web" => "serve, request, fail, get and post",
        _ => "read, write and pretty",
    }
}

/// `Maths:sqrt(2.0)` and the like. `args` are the argument types, for the methods that work
/// on both ints and floats. None when the object has no such method.
pub fn object_method(obj: &str, m: &str, args: &[Ty]) -> Option<Result<Lib, String>> {
    use Ty::*;
    let f = || Float;
    Some(match (obj, m) {
        ("Maths", "abs") => match numeric(args) {
            Some(t) => lib(&format!("maths_abs_{}", tname(&t)), vec![("number", t.clone())], t),
            None => Err("Maths:abs works on a number.".into()),
        },
        ("Maths", "min" | "max") => match numeric(args) {
            Some(t) => lib(&format!("maths_{m}_{}", tname(&t)), vec![("a", t.clone()), ("b", t.clone())], t),
            None => Err(format!("Maths:{m} works on two numbers.")),
        },
        ("Maths", "clamp") => match numeric(args) {
            Some(t) => lib(
                &format!("maths_clamp_{}", tname(&t)),
                vec![("number", t.clone()), ("low", t.clone()), ("high", t.clone())],
                t,
            ),
            None => Err("Maths:clamp works on numbers: Maths:clamp(hp, 0, 100).".into()),
        },
        ("Maths", "round" | "floor" | "ceil") => lib(&format!("maths_{m}"), vec![("number", f())], Int),
        ("Maths", "sqrt" | "sin" | "cos" | "tan") => lib(&format!("maths_{m}"), vec![("number", f())], Float),
        ("Maths", "power") => lib("maths_power", vec![("number", f()), ("power", f())], Float),
        ("Files", "read") => lib("files_read", vec![("file", Str)], Str),
        ("Files", "write" | "append") => lib(&format!("files_{m}"), vec![("file", Str), ("text", Str)], Void),
        ("Files", "exists") => lib("files_exists", vec![("path", Str)], Truth),
        ("Files", "remove") => lib("files_remove", vec![("path", Str)], Void),
        ("Files", "list") => lib("files_list", vec![("folder", Str)], Array(Box::new(Str))),
        ("Files", "makeFolder") => lib("files_makeFolder", vec![("folder", Str)], Void),
        ("Time", "now" | "clock") => lib(&format!("time_{m}"), vec![], Float),
        ("Web", "get") => lib("web_get", vec![("address", Str)], Str),
        ("Web", "post") => lib("web_post", vec![("address", Str), ("body", Str)], Str),
        ("Time", "addDays") => lib("time_addDays", vec![("date", Str), ("days", Int)], Str),
        ("Time", "daysBetween") => lib("time_daysBetween", vec![("from", Str), ("to", Str)], Int),
        ("Time", "weekday") => lib("time_weekday", vec![("date", Str)], Str),
        ("Time", "isDate") => lib("time_isDate", vec![("text", Str)], Truth),
        ("Web", "serve") => lib("web_serve", vec![("port", Int)], Void),
        ("Web", "request") => lib("web_request", vec![("method", Str), ("path", Str), ("body", Str)], Str),
        ("Web", "fail") => lib("web_fail", vec![("status", Int), ("message", Str)], Void),
        ("Time", "date" | "time") => lib(&format!("time_{m}"), vec![], Str),
        _ => return None,
    })
}

fn tname(t: &Ty) -> &'static str {
    if *t == Ty::Float {
        "float"
    } else {
        "int"
    }
}

/// Methods on text and arrays: `name:contains("a")`. The value itself goes first.
pub fn value_method(t: &Ty, m: &str) -> Option<Result<Lib, String>> {
    use Ty::*;
    Some(match (t, m) {
        (Str, "contains" | "startsWith" | "endsWith") => lib(&format!("text_{m}"), vec![("text", Str)], Truth),
        (Str, "find") => lib("text_find", vec![("text", Str)], Int),
        (Str, "replace") => lib("text_replace", vec![("old", Str), ("new", Str)], Str),
        (Str, "split") => lib("text_split", vec![("between", Str)], Array(Box::new(Str))),
        (Str, "slice") => lib("text_slice", vec![("from", Int), ("to", Int)], Str),
        (Str, "matches") => lib("text_matches", vec![("pattern", Str)], Truth),
        (Str, "findAll") => lib("text_findAll", vec![("pattern", Str)], Array(Box::new(Str))),
        (Str, "replaceAll") => lib("text_replaceAll", vec![("pattern", Str), ("with", Str)], Str),
        (Str, "capture") => lib("text_capture", vec![("pattern", Str)], Array(Box::new(Str))),
        (Array(el), "join") => {
            if **el == Str {
                lib("array_join", vec![("between", Str)], Str)
            } else {
                Err("join works on string arrays. Turn the items into text first.".into())
            }
        }
        (Array(el), "has" | "find") => {
            if matches!(**el, Int | Float | Str | Truth) {
                let ret = if m == "has" { Truth } else { Int };
                lib(&format!("array_{m}"), vec![("item", (**el).clone())], ret)
            } else {
                Err(format!(
                    ":{m} works on arrays of numbers, text and truths. For other arrays, use a loop."
                ))
            }
        }
        (Array(el), "sort") => {
            if matches!(**el, Int | Float | Str) {
                lib("array_sort", vec![], Void)
            } else {
                Err(":sort works on arrays of numbers and text.".into())
            }
        }
        (Array(_), "reverse") => lib("array_reverse", vec![], Void),
        _ => return None,
    })
}

/// Is this one of the built-in objects, written as a name: `Maths` in `Maths:sqrt(2.0)`?
pub fn is_object(e: &crate::ast::Expr) -> bool {
    matches!(&e.kind, crate::ast::ExprKind::Name(n) if OBJECTS.contains(&n.as_str()))
}

/// The class's fields that go into JSON, parents' first. Hooks stay out.
pub fn json_fields(info: &crate::check::Info, c: usize) -> Vec<(usize, String, Ty)> {
    let mut chain = vec![c];
    while let Some(p) = info.classes[*chain.last().unwrap()].parent {
        chain.push(p);
    }
    chain.reverse();
    let mut out = Vec::new();
    for k in chain {
        for f in &info.classes[k].fields {
            if !matches!(f.ty, Ty::Hook(_)) {
                out.push((k, f.name.clone(), f.ty.clone()));
            }
        }
    }
    out
}

pub fn uses_json(info: &crate::check::Info) -> bool {
    !info.saves.is_empty() || !info.routes.is_empty() || info.libs.values().any(|l| l.func.starts_with("json_"))
}

/// Can this be turned into JSON and back? Hooks can't, and neither can `nothing` on its own.
pub fn json_problem(t: &Ty, show: &dyn Fn(&Ty) -> String) -> Option<String> {
    match t {
        Ty::Int | Ty::Float | Ty::Str | Ty::Truth | Ty::State(_) | Ty::Class(_) | Ty::Error => None,
        Ty::Array(e) | Ty::Map(e) => json_problem(e, show),
        Ty::CMap(cols) => cols.iter().find_map(|(_, t)| json_problem(t, show)),
        other => Some(format!("{} can't be JSON.", show(other))),
    }
}
