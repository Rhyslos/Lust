//! Lust's types, as the checker sees them.

use std::rc::Rc;

/// A sized whole number's size: int8 to int32, uint8 to uint32.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Width {
    I8,
    I16,
    I32,
    U8,
    U16,
    U32,
}

impl Width {
    pub const ALL: [Width; 6] = [Width::I8, Width::I16, Width::I32, Width::U8, Width::U16, Width::U32];

    /// As Lust writes it: int8.
    pub fn name(self) -> &'static str {
        match self {
            Width::I8 => "int8",
            Width::I16 => "int16",
            Width::I32 => "int32",
            Width::U8 => "uint8",
            Width::U16 => "uint16",
            Width::U32 => "uint32",
        }
    }

    /// As Rust writes it: i8.
    pub fn rust(self) -> &'static str {
        match self {
            Width::I8 => "i8",
            Width::I16 => "i16",
            Width::I32 => "i32",
            Width::U8 => "u8",
            Width::U16 => "u16",
            Width::U32 => "u32",
        }
    }

    /// The smallest and biggest numbers it holds.
    pub fn range(self) -> (i64, i64) {
        match self {
            Width::I8 => (i8::MIN as i64, i8::MAX as i64),
            Width::I16 => (i16::MIN as i64, i16::MAX as i64),
            Width::I32 => (i32::MIN as i64, i32::MAX as i64),
            Width::U8 => (0, u8::MAX as i64),
            Width::U16 => (0, u16::MAX as i64),
            Width::U32 => (0, u32::MAX as i64),
        }
    }

    pub fn from_name(n: &str) -> Option<Width> {
        Width::ALL.into_iter().find(|w| w.name() == n)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Ty {
    Int,
    /// A whole number kept in fewer bits: int8 hp = 3;. Only places that keep a value (variables,
    /// fields, items, parameters) have one; reading it gives an int.
    Sized(Width),
    Float,
    Str,
    Truth,
    /// Three numbers (f32), copied like a number: a position, a direction, a speed.
    Vec3,
    /// A turn in 3D space, kept as a quaternion of four f32 numbers, written as degrees.
    Rot,
    /// An entity in an engine (Solis), or nothing: its 64-bit ID.
    Entity,
    /// `nothing` before it's known what it's standing in for.
    Nothing,
    /// What a method that gives nothing back gives back.
    Void,
    /// Something already reported as a mistake: stay quiet about it.
    Error,
    Class(usize),
    /// One option of a state.
    State(usize),
    /// The state itself, as a list of options: `GameState` in `loop (GameState s in GameState)`.
    StateList(usize),
    Array(Box<Ty>),
    Map(Box<Ty>),
    CMap(Rc<Vec<(String, Ty)>>),
    /// One row of a cMap.
    Row(Rc<Vec<(String, Ty)>>),
    Hook(Rc<HookSig>),
    /// A database table: `Shop.items`.
    Table(Rc<TableTy>),
    /// Some of a table's rows: `Shop.items:where(price < 3.0)`.
    Query(Rc<TableTy>),
    /// One row of a table, read and changed in the database itself: `Shop.items["tea"]`.
    TableRow(Rc<TableTy>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct TableTy {
    pub db: usize,
    pub table: usize,
    /// As people see it: "Shop.items".
    pub name: String,
    /// The columns, the key first, like a cMap's.
    pub cols: Rc<Vec<(String, Ty)>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct HookSig {
    pub ret: Ty,
    pub params: Vec<(Ty, Option<String>)>,
}

impl Ty {
    pub fn is_number(&self) -> bool {
        matches!(self, Ty::Int | Ty::Float | Ty::Sized(_))
    }
    /// Values that can be `nothing`.
    pub fn can_be_nothing(&self) -> bool {
        matches!(self, Ty::Class(_) | Ty::Hook(_) | Ty::Entity | Ty::Nothing | Ty::Error)
    }
}

pub struct Names<'a> {
    pub classes: &'a [String],
    pub states: &'a [String],
}

impl<'a> Names<'a> {
    pub fn show(&self, t: &Ty) -> String {
        match t {
            Ty::Int => "int".into(),
            Ty::Sized(w) => w.name().into(),
            Ty::Float => "float".into(),
            Ty::Str => "string".into(),
            Ty::Truth => "bool".into(),
            Ty::Vec3 => "vec3".into(),
            Ty::Rot => "rotation".into(),
            Ty::Entity => "entity".into(),
            Ty::Nothing => "nothing".into(),
            Ty::Void => "nothing at all".into(),
            Ty::Error => "?".into(),
            Ty::Class(c) => self.classes[*c].clone(),
            Ty::State(s) => format!("{} option", self.states[*s]),
            Ty::StateList(s) => format!("the state {}", self.states[*s]),
            Ty::Array(e) => format!("{} array", self.show(e)),
            Ty::Map(e) => format!("{} map", self.show(e)),
            Ty::CMap(cols) => {
                let c: Vec<String> = cols.iter().map(|(n, t)| format!("{} {n}", self.show(t))).collect();
                format!("cMap({})", c.join(", "))
            }
            Ty::Row(_) => "cMap row".into(),
            Ty::Table(t) => format!("the table {}", t.name),
            Ty::Query(t) => format!("query on {}", t.name),
            Ty::TableRow(t) => format!("row of {}", t.name),
            Ty::Hook(sig) => {
                let mut s = String::from("hook");
                if sig.ret != Ty::Void {
                    s.push(' ');
                    s.push_str(&self.show(&sig.ret));
                }
                if !sig.params.is_empty() {
                    let p: Vec<String> = sig.params.iter().map(|(t, _)| self.show(t)).collect();
                    s.push_str(&format!("({})", p.join(", ")));
                }
                s
            }
        }
    }

    /// "an int", "a Player"
    pub fn with_article(&self, t: &Ty) -> String {
        let s = self.show(t);
        if matches!(t, Ty::Void | Ty::Nothing) || s.starts_with("the ") {
            return s;
        }
        let first = s.chars().next().unwrap_or('x').to_ascii_lowercase();
        // "a uint8": u sounds like you there.
        if "aeiou".contains(first) && !s.starts_with("uint") {
            format!("an {s}")
        } else {
            format!("a {s}")
        }
    }
}
