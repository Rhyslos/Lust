//! Mistakes found before running, collected and shown all at once.

use crate::ast::Pos;

#[derive(Default)]
pub struct Diags {
    pub list: Vec<(Pos, String)>,
}

impl Diags {
    pub fn add(&mut self, pos: Pos, msg: impl Into<String>) {
        let msg = msg.into();
        // One mistake per spot is plenty: later ones there are usually knock-on effects.
        if self.list.iter().any(|(p, m)| *p == pos && (m == &msg || pos.line > 0)) {
            return;
        }
        self.list.push((pos, msg));
    }

    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    pub fn sorted(&self) -> Vec<(Pos, String)> {
        let mut v = self.list.clone();
        v.sort_by_key(|(p, _)| (p.line, p.col));
        v
    }
}
