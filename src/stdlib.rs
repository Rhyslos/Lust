//! Lust's standard library, as the checker sees it: what each part takes and gives back.
//!
//! - Built-in objects: `Maths`, `Files`, `Time`, `Json`, `Web`, `Random`, `Rotation`, `Colour` and `Ease`, used
//!   like any object: `Maths:sqrt(2.0)`. `Random[1, 6]` is its own piece of syntax; `Random:seed(42)` is a
//!   method. `Rotation` (a turn) is a different word from `rotation` (the type), and `Colour` from `colour`.
//! - Methods on text and arrays: `name:contains("a")`, `words:join(", ")`.
//! - `.upper()`, `.lower()` and `.trim()` are conversions, like `.text()`.
//!
//! Each call becomes a runtime function with the same name in Lua (`_L.maths_sqrt`) and in
//! Rust (`rt::maths_sqrt`). The JSON ones are special: the writers build them from the types.

use crate::types::Ty;

pub const OBJECTS: &[&str] = &["Maths", "Files", "Time", "Json", "Web", "Random", "Rotation", "Colour", "Ease"];

/// The built-in states that aren't objects too: their names can't be used for anything else.
pub const STATES: &[&str] = &["Icon", "Problem", "Day", "Request", "Answer", "Run"];

/// `Ease` is a built-in object (`Ease:apply(Ease.OutBack, t)`) and a built-in state at the same
/// time: its options are the easing curves, and the checker makes it the first state, so its
/// number is always this.
pub const EASE: usize = 0;

/// `Icon`, Lust's icons as a built-in state (`icon Icon.Trash;`), is always the second.
pub const ICON: usize = 1;

/// `Problem`, the kinds of failure `try … error (Problem p, string message)` tells apart, is
/// always the third.
pub const PROBLEM: usize = 2;

/// `Day`, the days of the week `Time:weekday` gives back, is always the fourth.
pub const DAY: usize = 3;

/// `Request`, the kinds of request `Web:request` sends (the route words `get`, `post`, `put`
/// and `delete`), is always the fifth.
pub const REQUEST: usize = 4;

/// `Answer`, the statuses `Web:fail` answers with, is always the sixth.
pub const ANSWER: usize = 5;

/// `Run`, how started work is going (`gate.state`), is always the seventh, and the last
/// built-in one: the program's own states come after it.
pub const RUN: usize = 6;

/// Run's options: paused in a wait, running, finished, or stopped by `cancel`. The runtimes
/// (runtime.lua and native_rt.rs) use the same words.
pub const RUNS: &[&str] = &["Waiting", "Going", "Done", "Cancelled"];

/// Day's options, Monday first, as `Time:weekday` names them.
pub const DAYS: &[&str] = &["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"];

/// Request's options. Each is sent as its name in capitals (`Get` is GET).
pub const REQUESTS: &[&str] = &["Get", "Post", "Put", "Delete"];

/// Answer's options and the status each sends. The runtimes (runtime.lua and native_rt.rs)
/// each keep the same list, which a test checks.
pub const ANSWERS: &[(&str, u16)] = &[
    ("BadRequest", 400),
    ("NotSignedIn", 401),
    ("NotAllowed", 403),
    ("NotFound", 404),
    ("Conflict", 409),
    ("Gone", 410),
    ("TooBig", 413),
    ("TooMany", 429),
    ("Broken", 500),
    ("Busy", 503),
];

/// The built-in states after Ease and Icon, by number, with their options: each is made by the
/// checker in this order.
pub fn plain_states() -> Vec<(usize, &'static str, Vec<String>)> {
    let own = |l: &[&str]| l.iter().map(|o| o.to_string()).collect::<Vec<_>>();
    vec![
        (PROBLEM, "Problem", own(PROBLEMS)),
        (DAY, "Day", own(DAYS)),
        (REQUEST, "Request", own(REQUESTS)),
        (ANSWER, "Answer", ANSWERS.iter().map(|(o, _)| o.to_string()).collect()),
        (RUN, "Run", own(RUNS)),
    ]
}

/// Problem's options. Each failure in the runtimes (runtime.lua and native_rt.rs) names one of
/// these, the same one in both; anything not named is `Other`.
pub const PROBLEMS: &[&str] = &[
    "NotFound",
    "NotAllowed",
    "AlreadyThere",
    "BadJson",
    "BadText",
    "BadSave",
    "OutOfRange",
    "DividedByZero",
    "Nothing",
    "NoAnswer",
    "Database",
    "Stopped",
    "Other",
];

/// Icon's options, one per icon, from its name: "triangle-alert" is TriangleAlert.
pub fn icon_options() -> Vec<String> {
    crate::icons::ICONS
        .iter()
        .map(|(n, _)| n.split('-').map(|p| p[..1].to_uppercase() + &p[1..]).collect())
        .collect()
}

/// The easing curves, in order: `Ease`'s options. Their formulas are in runtime.lua and
/// native_rt.rs (`ease`), from easings.net.
pub const EASES: &[&str] = &[
    "Linear",
    "InSine",
    "OutSine",
    "InOutSine",
    "InQuad",
    "OutQuad",
    "InOutQuad",
    "InCubic",
    "OutCubic",
    "InOutCubic",
    "InQuart",
    "OutQuart",
    "InOutQuart",
    "InQuint",
    "OutQuint",
    "InOutQuint",
    "InExpo",
    "OutExpo",
    "InOutExpo",
    "InCirc",
    "OutCirc",
    "InOutCirc",
    "InBack",
    "OutBack",
    "InOutBack",
    "InElastic",
    "OutElastic",
    "InOutElastic",
    "InBounce",
    "OutBounce",
    "InOutBounce",
];

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
        "Maths" => "abs, min, max, clamp, wrap, round, floor, ceil, trunc, fraction, sign, sqrt, power, log, exp, sin, cos, tan, asin, acos, atan, atan2, degrees, radians, lerp, moveTowards, smoothstep, isFinite and noise, and Maths.pi and Maths.e",
        "Random" => "seed, chance and direction, and Random[1, 6] for a number",
        "Rotation" => "toward and around",
        "Colour" => "hex",
        "Ease" => "apply",
        "Files" => "read, write, append, exists, remove, list, makeFolder, chooseOpen, chooseSave and chooseFolder",
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
        // For going round a list both ways: Maths:wrap(i - 1, #items) is the last item from 0.
        ("Maths", "wrap") => match numeric(args) {
            Some(t) => lib(&format!("maths_wrap_{}", tname(&t)), vec![("number", t.clone()), ("count", t.clone())], t),
            None => Err("Maths:wrap works on numbers: Maths:wrap(i - 1, #items).".into()),
        },
        ("Maths", "sign") => match numeric(args) {
            Some(t) => lib(&format!("maths_sign_{}", tname(&t)), vec![("number", t.clone())], t),
            None => Err("Maths:sign works on a number.".into()),
        },
        ("Maths", "round" | "floor" | "ceil" | "trunc") => lib(&format!("maths_{m}"), vec![("number", f())], Int),
        ("Maths", "sqrt" | "sin" | "cos" | "tan" | "asin" | "acos" | "atan") => lib(&format!("maths_{m}"), vec![("number", f())], Float),
        ("Maths", "log" | "exp" | "fraction") => lib(&format!("maths_{m}"), vec![("number", f())], Float),
        ("Maths", "isFinite") => lib("maths_isFinite", vec![("number", f())], Truth),
        ("Maths", "lerp") => lib("maths_lerp", vec![("a", f()), ("b", f()), ("amount", f())], Float),
        ("Maths", "moveTowards") => lib("maths_moveTowards", vec![("current", f()), ("target", f()), ("step", f())], Float),
        ("Maths", "smoothstep") => lib("maths_smoothstep", vec![("low", f()), ("high", f()), ("x", f())], Float),
        // Two or three numbers: noise across a flat map, or through a space.
        ("Maths", "noise") if !(2..=3).contains(&args.len()) => Err(format!(
            "Maths:noise takes two numbers (x, y) or three (x, y, z), but gets {}.",
            args.len()
        )),
        ("Maths", "noise") if args.len() == 3 => lib("maths_noise3", vec![("x", f()), ("y", f()), ("z", f())], Float),
        ("Maths", "noise") => lib("maths_noise", vec![("x", f()), ("y", f())], Float),
        ("Rotation", "toward") => lib("rotation_toward", vec![("direction", Vec3)], Rot),
        ("Rotation", "around") => lib("rotation_around", vec![("axis", Vec3), ("degrees", f())], Rot),
        ("Colour", "hex") => lib("colour_hex", vec![("text", Str)], Colour),
        ("Ease", "apply") => lib("ease_apply", vec![("ease", State(EASE)), ("t", f())], Float),
        ("Random", "seed") => lib("random_seed", vec![("seed", Int)], Void),
        ("Random", "chance") => lib("random_chance", vec![("chance", f())], Truth),
        ("Random", "direction") => lib("random_direction", vec![], Vec3),
        ("Maths", "atan2") => lib("maths_atan2", vec![("y", f()), ("x", f())], Float),
        ("Maths", "degrees") => lib("maths_degrees", vec![("radians", f())], Float),
        ("Maths", "radians") => lib("maths_radians", vec![("degrees", f())], Float),
        ("Maths", "power") => lib("maths_power", vec![("number", f()), ("power", f())], Float),
        ("Files", "read") => lib("files_read", vec![("file", Str)], Str),
        ("Files", "write" | "append") => lib(&format!("files_{m}"), vec![("file", Str), ("text", Str)], Void),
        ("Files", "exists") => lib("files_exists", vec![("path", Str)], Truth),
        ("Files", "remove") => lib("files_remove", vec![("path", Str)], Void),
        ("Files", "list") => lib("files_list", vec![("folder", Str)], Array(Box::new(Str))),
        ("Files", "chooseOpen") => lib("files_chooseOpen", vec![("title", Str), ("types", Array(Box::new(Str)))], Str),
        ("Files", "chooseSave") => lib("files_chooseSave", vec![("title", Str), ("name", Str)], Str),
        ("Files", "chooseFolder") => lib("files_chooseFolder", vec![("title", Str)], Str),
        ("Files", "makeFolder") => lib("files_makeFolder", vec![("folder", Str)], Void),
        ("Time", "now" | "clock") => lib(&format!("time_{m}"), vec![], Float),
        ("Web", "get") => lib("web_get", vec![("address", Str)], Str),
        ("Web", "post") => lib("web_post", vec![("address", Str), ("body", Str)], Str),
        ("Time", "addDays") => lib("time_addDays", vec![("date", Str), ("days", Int)], Str),
        ("Time", "daysBetween") => lib("time_daysBetween", vec![("from", Str), ("to", Str)], Int),
        ("Time", "weekday") => lib("time_weekday", vec![("date", Str)], State(DAY)),
        ("Time", "isDate") => lib("time_isDate", vec![("text", Str)], Truth),
        ("Web", "serve") => lib("web_serve", vec![("port", Int)], Void),
        ("Web", "request") => lib("web_request", vec![("kind", State(REQUEST)), ("path", Str), ("body", Str)], Str),
        ("Web", "fail") => lib("web_fail", vec![("answer", State(ANSWER)), ("message", Str)], Void),
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
        // Values are compared as === does: a state's very option, an entity's ID, and which
        // thing (two things with the same fields are two things, as with ==).
        (Array(el), "has" | "find") => {
            let ret = if m == "has" { Truth } else { Int };
            match **el {
                Int | Float | Str | Truth | State(_) | Entity => lib(&format!("array_{m}"), vec![("item", (**el).clone())], ret),
                Class(_) => lib(&format!("array_{m}_thing"), vec![("item", (**el).clone())], ret),
                _ => Err(format!(
                    ":{m} works on arrays of numbers, text, bools, states, entities and things. For other arrays, use a loop."
                )),
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
        (Array(_), "shuffle") => lib("array_shuffle", vec![], Void),
        (Array(el), "pick") => lib("array_pick", vec![], (**el).clone()),
        (Array(el), "sum" | "average" | "smallest" | "largest") => match **el {
            Int | Float => {
                let ret = if m == "average" { Float } else { (**el).clone() };
                lib(&format!("array_{m}_{}", tname(el)), vec![], ret)
            }
            _ => Err(format!(":{m} works on arrays of numbers (ints or floats).")),
        },
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
        Ty::Vec3 | Ty::Vec2 | Ty::Rot | Ty::Colour => None,
        Ty::Array(e) | Ty::Map(e) => json_problem(e, show),
        Ty::CMap(cols) => cols.iter().find_map(|(_, t)| json_problem(t, show)),
        other => Some(format!("{} can't be JSON.", show(other))),
    }
}

/// Reads a colour's hex text, `"#3a8f2a"` or `"#3a8f2a80"`, into its four bytes (alpha 255 when
/// it isn't given), or says what's wrong with it, the same way the runtimes do.
pub fn colour_hex(text: &str) -> Result<[u8; 4], String> {
    let bad = || format!("hex needs a colour written as \"#rrggbb\" or \"#rrggbbaa\", like \"#3a8f2a\", but this is \"{text}\".");
    let hex = text.strip_prefix('#').ok_or_else(bad)?;
    if !matches!(hex.len(), 6 | 8) || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(bad());
    }
    let byte = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).unwrap_or(0);
    Ok([byte(0), byte(2), byte(4), if hex.len() == 8 { byte(6) } else { 255 }])
}
