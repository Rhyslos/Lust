// Views, continued: the mouse beyond clicking. Right-click menus, rows of a list dragged to new
// places, files dropped on the window, and the mouse on a canvas.
//
// This is part of the same module as native_ui.rs (lust includes it, and built programs get it,
// after native_ui_table.rs), kept apart so neither file grows too long.
//
// A right-click menu is a piece's `ctx` rows, shown in the same pop-up list a menu uses (the
// window's `popup`, with `ctx_at` saying where the mouse was). A row being dragged is a `Drag::Row`,
// and the line showing where it will land is painted over everything, from the paint state's
// marks. A canvas's press, drag, release and hover, a drop's block and a list's reorder are actions
// the view gives by name (a node's `mouse`), handed a point or a list of paths.

/// A row of a list with [reorder], held down: the list's place, the row's place in the order shown
/// (from 0), where the mouse went down, whether it has moved far enough to be a drag, and the gap
/// it would land in (0 is above the first row, the row count below the last).
#[derive(Clone, PartialEq, Debug)]
struct RowDrag {
    list: Vec<usize>,
    from: usize,
    start: (f32, f32),
    active: bool,
    slot: usize,
}

/// How far the mouse moves before a held row starts to drag, as on Windows.
const DRAG_START: f32 = 4.0;
/// How near the top or bottom of its list a dragged row makes the list scroll by itself, and how
/// fast it scrolls at the very edge (per second).
const EDGE: f32 = 24.0;
const EDGE_SPEED: f32 = 900.0;
/// The colour of the line where a dragged row lands, and of the edge shown while files are held
/// over the window.
const DROP_LINE: u32 = 0x1c7ed6;
const FILES_EDGE: u32 = 0x339af0;

/// Whether this is a table or list whose rows can be dragged to new places.
fn reorders(n: &Laid) -> bool {
    n.list.is_some() && n.mouse.iter().any(|(k, _)| k == "reorder")
}

fn mouse_action(n: &Laid, name: &str) -> Option<usize> {
    n.mouse.iter().find(|(k, _)| k == name).map(|(_, a)| *a)
}

/// What dump says about a piece's mouse: (menu) for a right-click menu, (reorder) for a list
/// whose rows drag, and a canvas's blocks: (press, drag, release, hover).
fn mouse_dump(n: &Laid) -> String {
    let mut out = String::new();
    if !n.ctx.is_empty() {
        out.push_str(" (menu)");
    }
    if reorders(n) {
        out.push_str(" (reorder)");
    }
    if n.kind == "canvas" && !n.mouse.is_empty() {
        let names: Vec<&str> = n.mouse.iter().map(|(k, _)| k.as_str()).collect();
        out.push_str(&format!(" ({})", names.join(", ")));
    }
    out
}

/// A right-click menu's list, at the mouse: below and to the right of it, or above or to the
/// left where there isn't room, as Windows places them.
fn ctx_layer(owner: &Laid, at: (f32, f32), hl: Option<usize>, w: f32, h: f32) -> Laid {
    let (mut list, lw, tall) = popup_box(owner.ctx.clone(), hl, None, 160.0, w);
    let mut x = at.0 + 2.0;
    if x + lw > w - 8.0 {
        x = at.0 - lw - 2.0;
    }
    let x = x.min(w - 8.0 - lw).max(8.0);
    let room = (h - 16.0).max(40.0);
    let lh = tall.min(room);
    let mut y = at.1 + 2.0;
    if y + lh > h - 8.0 {
        y = if at.1 - lh - 2.0 >= 8.0 { at.1 - lh - 2.0 } else { (h - 8.0 - lh).max(8.0) };
    }
    place(&mut list, x.round(), y.round(), lw, lh.round());
    MEASURED.with(|m| m.borrow_mut().clear());
    layer("popup", w, h, list)
}

/// The deepest piece with a right-click menu at this point. A table's rows that aren't on the
/// screen don't count.
fn menu_owner(n: &Laid, x: f32, y: f32, path: &mut Vec<usize>, found: &mut Option<Vec<usize>>) {
    if x < n.x || y < n.y || x >= n.x + n.w || y >= n.y + n.h {
        return;
    }
    if !n.ctx.is_empty() {
        *found = Some(path.clone());
    }
    for (i, c) in n.children.iter().enumerate() {
        if n.list.is_some() && i >= first_row(n) && !(row_shows(n, c) && y >= list_body(n).1) {
            continue;
        }
        path.push(i);
        menu_owner(c, x, y, path, found);
        path.pop();
    }
}

/// What's drawn over everything: the line where a dragged row will land, and a soft edge round
/// the window while files are held over it.
fn paint_marks(c: &mut Canvas, tree: &Laid, st: &PaintState) {
    for m in &st.marks {
        c.rect(m[0], m[1], m[2], m[3], m[3] / 2.0, DROP_LINE, 1.0);
        c.shape(m[0] - 3.0, m[1] + m[3] / 2.0 - 4.0, 8.0, 8.0, 4.0, None, DROP_LINE, 1.0);
    }
    if st.files_over {
        let (w, h) = (tree.w, tree.h);
        c.rect(0.0, 0.0, w, h, 0.0, FILES_EDGE, 0.06);
        c.shape(1.5, 1.5, (w - 3.0).max(0.0), (h - 3.0).max(0.0), 6.0, Some(3.0), FILES_EDGE, 0.7);
    }
}

/// The open dialogs in the view, as places in the tree of nodes (whether shown or not).
fn open_dialogs(n: &Node, path: &mut Vec<usize>, out: &mut Vec<Vec<usize>>) {
    if n.kind == "dialog" && matches!(n.value, Some(SV::Truth(true))) {
        out.push(path.clone());
    }
    for (i, c) in n.children.iter().enumerate() {
        path.push(i);
        open_dialogs(c, path, out);
        path.pop();
    }
}

fn node_in<'a>(n: &'a Node, path: &[usize]) -> Option<&'a Node> {
    match path.split_first() {
        None => Some(n),
        Some((&i, rest)) => n.children.get(i).and_then(|c| node_in(c, rest)),
    }
}

/// The drop block that runs when files are dropped: the first in the dialog opened last, if it
/// has one, or else the first in the view, as it's written. Only one runs for each drop.
fn drop_action(root: &Node) -> Option<usize> {
    let first = |n: &Node| {
        let mut found = None;
        each_shown(n, &mut vec![], &mut |d| {
            if d.kind == "drop" && found.is_none() {
                found = d.mouse.iter().find(|(k, _)| k == "drop").map(|(_, a)| *a);
            }
        });
        found
    };
    let mut dialogs = Vec::new();
    open_dialogs(root, &mut vec![], &mut dialogs);
    for p in dialogs.iter().rev() {
        if let Some(a) = node_in(root, p).and_then(first) {
            return Some(a);
        }
    }
    first(root)
}

/// Where the row at place `p` is after the row at `from` moves to `to`: the ones between shift over.
fn after_move(p: usize, from: usize, to: usize) -> usize {
    if p == from {
        to
    } else if from < p && p <= to {
        p - 1
    } else if to <= p && p < from {
        p + 1
    } else {
        p
    }
}

/// Words for a test's command, split at spaces, with "…" keeping spaces in one.
fn test_words(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    let mut any = false;
    for ch in s.chars() {
        match ch {
            '"' => {
                quoted = !quoted;
                any = true;
            }
            c if c.is_whitespace() && !quoted => {
                if any {
                    out.push(std::mem::take(&mut cur));
                    any = false;
                }
            }
            c => {
                cur.push(c);
                any = true;
            }
        }
    }
    if any {
        out.push(cur);
    }
    out
}

/// Where `rightclick` presses in a test: at x y, or on the button, row or words showing this text.
fn right_target(ui: &Ui, rest: &str, nums: &[f32]) -> Option<(f32, f32)> {
    if nums.len() >= 2 && rest.split_whitespace().count() == 2 {
        return Some((nums[0], nums[1]));
    }
    fn any_text(n: &Laid, text: &str, path: &mut Vec<usize>) -> Option<Vec<usize>> {
        if n.text == text && n.kind != "layer" {
            return Some(path.clone());
        }
        for (i, c) in n.children.iter().enumerate() {
            if n.list.is_some() && i >= first_row(n) && !row_shows(n, c) {
                continue;
            }
            path.push(i);
            let f = any_text(c, text, path);
            path.pop();
            if f.is_some() {
                return f;
            }
        }
        None
    }
    let p = find_button(&ui.tree, rest, &mut vec![])
        .or_else(|| find_row_text(&ui.tree, rest, &mut vec![]))
        .or_else(|| any_text(&ui.tree, rest, &mut vec![]))?;
    Some(centre(ui, &p))
}

/// `drag Ada Cleo` in a test: drags the row showing Ada onto the row showing Cleo, so it takes
/// Cleo's place. `drag 2 5` drags the second row on the screen of the first list that reorders
/// onto the fifth.
fn test_drag(ui: &mut Ui, rest: &str) -> Result<(), String> {
    let words = test_words(rest);
    let [a, b] = words.as_slice() else {
        println!("(drag needs the row to drag and the row to drag it to: drag Ada Cleo, or drag 2 5)");
        return Ok(());
    };
    let row_of = |ui: &Ui, w: &str| -> Option<Vec<usize>> {
        if let Ok(k) = w.parse::<usize>() {
            let mut lists = Vec::new();
            fn every(n: &Laid, path: &mut Vec<usize>, out: &mut Vec<Vec<usize>>) {
                if reorders(n) {
                    out.push(path.clone());
                }
                for (i, c) in n.children.iter().enumerate() {
                    path.push(i);
                    every(c, path, out);
                    path.pop();
                }
            }
            every(&ui.tree, &mut vec![], &mut lists);
            let l = lists.first()?;
            let n = node_at(&ui.tree, l)?;
            let j = k.checked_sub(1)?;
            let i = n.children.iter().enumerate().skip(first_row(n)).find(|(_, r)| r.index == j && row_shows(n, r))?.0;
            let mut p = l.clone();
            p.push(i);
            return Some(p);
        }
        let p = find_row_text(&ui.tree, w, &mut vec![])?;
        // The row the words are in.
        (1..=p.len()).rev().map(|d| p[..d].to_vec()).find(|q| node_at(&ui.tree, q).is_some_and(|n| matches!(n.kind.as_str(), "tablerow" | "listrow")))
    };
    let (Some(from), Some(to)) = (row_of(ui, a), row_of(ui, b)) else {
        println!("(there's no row showing \"{a}\" or \"{b}\" to drag)");
        return Ok(());
    };
    let (Some(f), Some(t)) = (node_at(&ui.tree, &from).cloned(), node_at(&ui.tree, &to).cloned()) else { return Ok(()) };
    let (x0, y0) = (f.x + f.w / 2.0, f.y + f.h / 2.0);
    // Onto the lower part of a row below, or the upper part of one above, to take its place.
    let ty = if t.index > f.index { t.y + t.h * 0.75 } else { t.y + t.h * 0.25 };
    let tx = t.x + t.w / 2.0;
    ui.press(x0, y0, Mods::default())?;
    let step = if ty >= y0 { DRAG_START + 1.0 } else { -(DRAG_START + 1.0) };
    if let Drag::Row(r) = ui.drag.clone() {
        ui.row_moved(r, x0, y0 + step, false)?;
    }
    if let Drag::Row(r) = ui.drag.clone() {
        ui.row_moved(r, tx, ty, false)?;
    }
    ui.st.hover = Some((tx, ty));
    ui.release(tx, ty)
}

impl<'a, 'b> Ui<'a, 'b> {
    /// The rows of the open list: a menu's items, a dropdown's choices, or a right-click menu's.
    fn popup_items<'n>(&self, owner: &'n Laid) -> &'n Vec<Laid> {
        if self.ctx_at.is_some() {
            &owner.ctx
        } else {
            &owner.items
        }
    }

    /// Whether this point is inside the open list.
    fn in_popup(&self, x: f32, y: f32) -> bool {
        self.tree
            .children
            .iter()
            .rfind(|c| c.kind == "layer" && c.text == "popup")
            .and_then(|l| l.children.first())
            .is_some_and(|b| x >= b.x && y >= b.y && x < b.x + b.w && y < b.y + b.h)
    }

    /// The deepest piece with a right-click menu at this point, that the mouse can reach.
    fn menu_owner_at(&self, x: f32, y: f32) -> Option<Vec<usize>> {
        for i in self.reach().rev() {
            let c = &self.tree.children[i];
            if c.kind == "layer" && c.text != "dialog" {
                continue;
            }
            let mut found = None;
            menu_owner(c, x, y, &mut vec![i], &mut found);
            if found.is_some() {
                return found;
            }
        }
        None
    }

    /// The right button went down: an open list closes (unless it's under the mouse), and the
    /// row under the mouse is picked, as Windows Explorer does, ready for its menu.
    fn right_press(&mut self, x: f32, y: f32) -> Result<(), String> {
        self.st.hover = Some((x, y));
        self.hide_tip();
        self.right = None;
        if matches!(self.drag, Drag::Row(_)) {
            return Ok(());
        }
        if self.popup.is_some() {
            if self.in_popup(x, y) {
                return Ok(());
            }
            self.popup = None;
            self.relayout();
        }
        // Outside an open dialog, the right button does nothing.
        if let Some(m) = modal_in(&self.tree) {
            let inside = self.tree.children[m].children.first().is_some_and(|b| x >= b.x && y >= b.y && x < b.x + b.w && y < b.y + b.h);
            if !inside {
                return Ok(());
            }
        }
        if let Some(path) = self.hit_at(x, y) {
            let row = (1..=path.len())
                .rev()
                .map(|d| path[..d].to_vec())
                .find(|p| node_at(&self.tree, p).is_some_and(|n| matches!(n.kind.as_str(), "tablerow" | "listrow")));
            if let Some(row) = row {
                let list = row[..row.len() - 1].to_vec();
                let j = node_at(&self.tree, &row).map(|r| r.index).unwrap_or(0);
                if node_at(&self.tree, &list).is_some_and(|l| !l.look.disabled) {
                    self.set_focus(Some(list.clone()), false);
                    self.pick_row(&list, j, false)?;
                }
            }
        }
        self.right = self.menu_owner_at(x, y).map(|p| (p, x, y));
        Ok(())
    }

    /// The right button was let go: the menu of what it went down on opens at the mouse.
    fn right_release(&mut self, x: f32, y: f32) -> Result<(), String> {
        self.st.hover = Some((x, y));
        let Some((owner, _, _)) = self.right.take() else { return Ok(()) };
        self.open_ctx(&owner, (x, y), false);
        Ok(())
    }

    /// Opens the right-click menu of the piece at `owner`, at a place in the window. From the
    /// keyboard its first item that can be chosen is picked.
    fn open_ctx(&mut self, owner: &[usize], at: (f32, f32), keyboard: bool) {
        let Some(n) = node_at(&self.tree, owner) else { return };
        if n.ctx.is_empty() {
            return;
        }
        let hl = if keyboard { n.ctx.iter().position(|r| r.action.is_some()) } else { None };
        self.popup = Some((owner.to_vec(), hl));
        self.ctx_at = Some(at);
        self.relayout();
    }

    /// Shift+F10 or the Menu key: opens the right-click menu of what has focus (or of what holds
    /// it), at its bottom left. In a table or list, it's the picked row's menu.
    fn menu_key(&mut self) -> Result<(), String> {
        let Some(focus) = self.st.focus.clone() else { return Ok(()) };
        let mut owner = None;
        if let Some(n) = node_at(&self.tree, &focus).filter(|n| n.list.is_some()) {
            let pick = n.list.as_ref().and_then(|i| i.pick);
            if let Some((i, _)) = n.children.iter().enumerate().skip(first_row(n)).find(|(_, r)| Some(r.index) == pick && !r.ctx.is_empty()) {
                let mut p = focus.clone();
                p.push(i);
                owner = Some(p);
            }
        }
        if owner.is_none() {
            owner = (0..=focus.len()).rev().map(|d| focus[..d].to_vec()).find(|p| node_at(&self.tree, p).is_some_and(|n| !n.ctx.is_empty()));
        }
        let Some(owner) = owner else { return Ok(()) };
        let Some(n) = node_at(&self.tree, &owner) else { return Ok(()) };
        let at = (n.x + 8.0, n.y + n.h);
        self.open_ctx(&owner, at, true);
        Ok(())
    }

    // ----- dragging rows to new places -----

    /// The mouse went down on a row: in a list with [reorder], it's held, ready to drag.
    fn row_press(&mut self, row: &[usize], x: f32, y: f32) {
        let Some((_, list)) = row.split_last() else { return };
        let (Some(l), Some(r)) = (node_at(&self.tree, list), node_at(&self.tree, row)) else { return };
        if reorders(l) && !l.look.disabled {
            self.drag = Drag::Row(RowDrag { list: list.to_vec(), from: r.index, start: (x, y), active: false, slot: r.index });
        }
    }

    /// Which gap between rows a dragged row would land in, with the mouse at height `y`.
    fn row_slot(&self, r: &RowDrag, y: f32) -> usize {
        let Some(n) = node_at(&self.tree, &r.list) else { return r.from };
        let Some(info) = n.list.clone() else { return r.from };
        let (_, by, _, _) = list_body(n);
        let rh = with_list(info.id, |st| st.row_h).max(1.0);
        (((y - by + n.scrolled) / rh).round().max(0.0) as usize).min(info.count)
    }

    /// The mouse moved with a row held: once it has gone far enough, the row is dragged and the
    /// line shows where it would land. A table sorted by a header can't be dragged in, since its
    /// order isn't the array's. With `auto`, a row near the list's top or bottom scrolls it a
    /// step (a test's `move`); a window scrolls it on its own clock instead.
    fn row_moved(&mut self, mut r: RowDrag, x: f32, y: f32, auto: bool) -> Result<(), String> {
        if !r.active {
            if (x - r.start.0).abs() < DRAG_START && (y - r.start.1).abs() < DRAG_START {
                return Ok(());
            }
            let sorted = node_at(&self.tree, &r.list).and_then(|n| n.list.clone()).is_some_and(|i| with_list(i.id, |st| st.sort.is_some()));
            if sorted {
                self.drag = Drag::None;
                return Ok(());
            }
            r.active = true;
            self.pressed = None;
        }
        self.st.hover = Some((x, y));
        self.drag = Drag::Row(r.clone());
        if auto {
            let speed = self.edge_speed();
            if speed != 0.0 {
                let now = node_at(&self.tree, &r.list).map(|n| n.scrolled).unwrap_or(0.0);
                self.list_scroll(&r.list, now + speed / 30.0)?;
            }
        }
        r.slot = self.row_slot(&r, y);
        self.drag = Drag::Row(r);
        Ok(())
    }

    /// How fast the list under a dragged row should scroll by itself (per second, up below 0):
    /// faster the nearer the mouse is to its edge.
    fn edge_speed(&self) -> f32 {
        let (Drag::Row(r), Some((_, y))) = (&self.drag, self.st.hover) else { return 0.0 };
        if !r.active {
            return 0.0;
        }
        let Some(n) = node_at(&self.tree, &r.list) else { return 0.0 };
        let (_, by, _, bh) = list_body(n);
        let max = (n.content_h - (by - n.y - n.look.pad[0]) - bh).max(0.0);
        if y < by + EDGE && n.scrolled > 0.0 {
            -EDGE_SPEED * ((by + EDGE - y) / EDGE).min(1.0)
        } else if y > by + bh - EDGE && n.scrolled < max {
            EDGE_SPEED * ((y - (by + bh - EDGE)) / EDGE).min(1.0)
        } else {
            0.0
        }
    }

    /// Scrolls the list under a dragged row by itself, by the time since the last step.
    fn edge_scroll(&mut self) -> Result<(), String> {
        let speed = self.edge_speed();
        let Drag::Row(r) = self.drag.clone() else { return Ok(()) };
        let now = std::time::Instant::now();
        let dt = self.edge_last.map(|t| now.duration_since(t).as_secs_f32()).unwrap_or(1.0 / 60.0).min(0.05);
        self.edge_last = Some(now);
        let at = node_at(&self.tree, &r.list).map(|n| n.scrolled).unwrap_or(0.0);
        self.list_scroll(&r.list, at + speed * dt)?;
        if let Some((x, y)) = self.st.hover {
            self.row_moved(r, x, y, false)?;
        }
        Ok(())
    }

    /// The line where a dragged row would land, to paint over everything.
    fn drag_marks(&self) -> Vec<[f32; 4]> {
        let Drag::Row(r) = &self.drag else { return vec![] };
        if !r.active {
            return vec![];
        }
        let Some(n) = node_at(&self.tree, &r.list) else { return vec![] };
        let Some(info) = &n.list else { return vec![] };
        let (bx, by, bw, bh) = list_body(n);
        let rh = with_list(info.id, |st| st.row_h);
        let y = (by + r.slot as f32 * rh - n.scrolled).clamp(by + 1.0, by + bh - 1.0);
        vec![[bx + 4.0, (y - 1.0).round(), (bw - 16.0).max(8.0), 2.0]]
    }

    /// A dragged row was let go: it moves to where the line shows.
    fn row_drop(&mut self, r: RowDrag) -> Result<(), String> {
        let to = if r.slot > r.from { r.slot - 1 } else { r.slot };
        self.move_row(&r.list, r.from, to)
    }

    /// Moves the row at `from` to `to` (places in the list, from 0) in the array itself, then
    /// picks it there. The picked row stays the one that's picked.
    fn move_row(&mut self, list: &[usize], from: usize, to: usize) -> Result<(), String> {
        let Some(n) = node_at(&self.tree, list) else { return Ok(()) };
        let (Some(a), Some(info)) = (mouse_action(n, "reorder"), n.list.clone()) else { return Ok(()) };
        if from == to || from >= info.count || to >= info.count {
            return Ok(());
        }
        run_with(&mut self.actions[a], SV::Point(from as f64, to as f64))?;
        with_list(info.id, |st| st.pick = st.pick.map(|p| after_move(p, from, to)));
        self.rebuild()?;
        self.pick_row(list, to, false)
    }

    /// Alt+Up and Alt+Down: the picked row of a list with [reorder] moves up or down one place.
    fn row_nudge(&mut self, list: &[usize], up: bool) -> Result<(), String> {
        let Some(n) = node_at(&self.tree, list) else { return Ok(()) };
        let Some(info) = n.list.clone() else { return Ok(()) };
        if !reorders(n) || n.look.disabled || with_list(info.id, |st| st.sort.is_some()) {
            return Ok(());
        }
        let Some(p) = info.pick else { return Ok(()) };
        let to = if up { p.checked_sub(1) } else { Some(p + 1).filter(|&t| t < info.count) };
        match to {
            Some(t) => self.move_row(list, p, t),
            None => Ok(()),
        }
    }

    // ----- the mouse on a canvas -----

    /// Where the mouse is on the canvas at `path`, from its top left (inside its padding).
    fn canvas_point(&self, path: &[usize], x: f32, y: f32) -> (f64, f64) {
        match node_at(&self.tree, path) {
            Some(n) => ((x - n.x - n.look.pad[3]) as f64, (y - n.y - n.look.pad[0]) as f64),
            None => (0.0, 0.0),
        }
    }

    /// Runs a canvas's press, drag, release or hover with where the mouse is, then draws again.
    fn canvas_run(&mut self, path: &[usize], name: &str, x: f32, y: f32) -> Result<(), String> {
        let Some(a) = node_at(&self.tree, path).filter(|n| n.kind == "canvas").and_then(|n| mouse_action(n, name)) else { return Ok(()) };
        let (px, py) = self.canvas_point(path, x, y);
        self.ran += 1;
        run_with(&mut self.actions[a], SV::Point(px, py))?;
        self.rebuild()
    }

    /// The left button went down on a canvas: its press runs, and its drag and release follow the
    /// mouse until the button is let go, even outside the canvas.
    fn canvas_press(&mut self, path: &[usize], x: f32, y: f32) -> Result<(), String> {
        self.drag = Drag::Canvas(path.to_vec());
        self.canvas_run(path, "press", x, y)
    }

    fn canvas_drag(&mut self, path: &[usize], x: f32, y: f32) -> Result<(), String> {
        self.canvas_run(path, "drag", x, y)
    }

    fn canvas_release(&mut self, path: &[usize], x: f32, y: f32) -> Result<(), String> {
        self.canvas_run(path, "release", x, y)
    }

    /// The mouse moved with no button held: the hover of the canvas under it runs.
    fn canvas_hover(&mut self, x: f32, y: f32) -> Result<(), String> {
        fn find(n: &Laid, x: f32, y: f32, path: &mut Vec<usize>, out: &mut Option<Vec<usize>>) {
            if x < n.x || y < n.y || x >= n.x + n.w || y >= n.y + n.h {
                return;
            }
            if n.kind == "canvas" && mouse_action(n, "hover").is_some() {
                *out = Some(path.clone());
            }
            for (i, c) in n.children.iter().enumerate() {
                path.push(i);
                find(c, x, y, path, out);
                path.pop();
            }
        }
        if self.popup.is_some() && self.in_popup(x, y) {
            return Ok(());
        }
        let mut found = None;
        for i in self.reach().rev() {
            let c = &self.tree.children[i];
            if c.kind == "layer" && c.text != "dialog" {
                continue;
            }
            find(c, x, y, &mut vec![i], &mut found);
            if found.is_some() {
                break;
            }
        }
        match found {
            Some(p) => self.canvas_run(&p, "hover", x, y),
            None => Ok(()),
        }
    }

    // ----- files dropped on the window -----

    /// Files were dropped on the window: one drop block runs with all their paths.
    fn drop_files(&mut self, paths: Vec<String>) -> Result<(), String> {
        self.files_over = false;
        if paths.is_empty() {
            return Ok(());
        }
        let Some(a) = drop_action(&self.node) else { return Ok(()) };
        run_with(&mut self.actions[a], SV::List(paths))?;
        self.rebuild()
    }

    /// What dump adds after the pieces: a row being dragged, and files held over the window.
    fn mouse_dump_lines(&self) -> String {
        let mut out = String::new();
        if let Drag::Row(r) = &self.drag {
            if r.active {
                let count = node_at(&self.tree, &r.list).and_then(|n| n.list.as_ref()).map(|i| i.count).unwrap_or(0);
                let lands = if r.slot >= count { "after the last row".to_string() } else { format!("before row {}", r.slot + 1) };
                out.push_str(&format!("dragging row {}, landing {lands}\n", r.from + 1));
            }
        }
        if self.files_over {
            out.push_str("files held over the window\n");
        }
        out
    }
}

#[cfg(test)]
mod mouse_tests {
    use super::*;

    #[test]
    fn rows_shift_over_when_one_moves() {
        // [a b c d e]: b (1) moves to 3, giving [a c d b e].
        let moved: Vec<usize> = (0..5).map(|p| after_move(p, 1, 3)).collect();
        assert_eq!(moved, vec![0, 3, 1, 2, 4]);
        // And back up: d (3) moves to 0, giving [d a b c e].
        let moved: Vec<usize> = (0..5).map(|p| after_move(p, 3, 0)).collect();
        assert_eq!(moved, vec![1, 2, 3, 0, 4]);
        assert_eq!(after_move(7, 2, 2), 7);
        // Every place is still used once.
        for (from, to) in [(0, 4), (4, 0), (2, 3), (3, 2)] {
            let mut all: Vec<usize> = (0..5).map(|p| after_move(p, from, to)).collect();
            all.sort();
            assert_eq!(all, vec![0, 1, 2, 3, 4]);
        }
    }

    #[test]
    fn test_words_keep_quoted_spaces() {
        assert_eq!(test_words(r#"C:/a.txt "C:/My files/b.png""#), vec!["C:/a.txt", "C:/My files/b.png"]);
        assert_eq!(test_words("  Ada   Cleo "), vec!["Ada", "Cleo"]);
        assert_eq!(test_words(r#""""#), vec![""]);
        assert!(test_words("").is_empty());
        // Odd quotes and odd letters don't upset it.
        for s in ["\"", "a\"b", "\u{0}", "\"\"\"x", "é \"ü ö\""] {
            let _ = test_words(s);
        }
    }

    #[test]
    fn a_right_click_menu_stays_in_the_window() {
        let mut owner = lay(&Node::new("text"));
        let mut item = Node::new("item");
        item.text = "Copy".into();
        item.action = Some(0);
        owner.ctx = vec![lay(&item)];
        let inside = |l: &Laid, w: f32, h: f32| {
            let b = &l.children[0];
            b.x >= 0.0 && b.y >= 0.0 && b.x + b.w <= w && b.y + b.h <= h
        };
        // Below and right of the mouse when there's room.
        let l = ctx_layer(&owner, (50.0, 50.0), None, 400.0, 300.0);
        assert!(inside(&l, 400.0, 300.0));
        assert!(l.children[0].x >= 50.0 && l.children[0].y >= 50.0);
        // Above and left of it in the bottom right corner.
        let l = ctx_layer(&owner, (395.0, 295.0), None, 400.0, 300.0);
        assert!(inside(&l, 400.0, 300.0));
        assert!(l.children[0].x < 395.0 && l.children[0].y < 295.0);
        // A window too small for it still starts it on the screen.
        let l = ctx_layer(&owner, (5.0, 5.0), Some(0), 60.0, 40.0);
        assert!(l.children[0].x >= 0.0 && l.children[0].y >= 0.0);
    }
}
