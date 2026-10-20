//! Lust's types, as the checker sees them.

use std::rc::Rc;

#[derive(Clone, Debug, PartialEq)]
pub enum Ty {
    Int,
    Float,
    Str,
    Truth,
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
    CArray(Rc<Vec<(String, Ty)>>),
    /// One row of a cArray.
    Row(Rc<Vec<(String, Ty)>>),
    Hook(Rc<HookSig>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct HookSig {
    pub ret: Ty,
    pub params: Vec<(Ty, Option<String>)>,
}

impl Ty {
    pub fn is_number(&self) -> bool {
        matches!(self, Ty::Int | Ty::Float)
    }
    /// Values that can be `nothing`.
    pub fn can_be_nothing(&self) -> bool {
        matches!(self, Ty::Class(_) | Ty::Hook(_) | Ty::Nothing | Ty::Error)
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
            Ty::Float => "float".into(),
            Ty::Str => "string".into(),
            Ty::Truth => "truth".into(),
            Ty::Nothing => "nothing".into(),
            Ty::Void => "nothing at all".into(),
            Ty::Error => "?".into(),
            Ty::Class(c) => self.classes[*c].clone(),
            Ty::State(s) => format!("{} option", self.states[*s]),
            Ty::StateList(s) => format!("the state {}", self.states[*s]),
            Ty::Array(e) => format!("{} array", self.show(e)),
            Ty::Map(e) => format!("{} map", self.show(e)),
            Ty::CArray(cols) => {
                let c: Vec<String> = cols.iter().map(|(n, t)| format!("{} {n}", self.show(t))).collect();
                format!("cArray({})", c.join(", "))
            }
            Ty::Row(_) => "cArray row".into(),
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
        if "aeiou".contains(first) {
            format!("an {s}")
        } else {
            format!("a {s}")
        }
    }
}
