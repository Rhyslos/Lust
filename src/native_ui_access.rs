// ----- screen readers: what each piece is, for people who can't see the screen -----
//
// A screen reader (Narrator or NVDA on Windows, Orca on Linux) reads out what's on the screen and
// lets people use it from the keyboard. It can't see Lust's pixels, so each time a window is drawn
// Lust also describes it as a tree: every piece with its role (a button, a check box, a slider…),
// its name, its value and its states (checked, disabled, focused…), and what can be done to it.
// AccessKit hands the tree to the computer: UI Automation on Windows, AT-SPI on Linux.
//
// The tree is built here from the laid-out pieces, the same for a window and for `dump access` in
// tests. Layout boxes (rows, columns, stacks) aren't in it: only what a person would want named.
// A table or list says how many rows it has, but only the rows on the screen are in the tree, so
// 100,000 rows cost no more than ten. Each node keeps one number (its id) from frame to frame: it's
// made from where the piece is in the view, and a row's from its place among the rows, so a row
// keeps its id as the table scrolls.
//
// What a screen reader asks for (click, focus, set a value, a step up or down, scroll into view)
// goes through the same paths as the mouse and keyboard, so the bound fields change as they would.

/// What a piece is, as a screen reader says it.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Role {
    Window,
    Label,
    Button,
    CheckBox,
    Switch,
    RadioGroup,
    RadioButton,
    Slider,
    SpinButton,
    TextInput,
    TextBox,
    ComboBox,
    MenuButton,
    Menu,
    MenuItem,
    ListBox,
    Choice,
    Dialog,
    TabList,
    Tab,
    Table,
    ColumnHeader,
    Row,
    Cell,
    List,
    ListItem,
    DateInput,
    ColourWell,
    Calendar,
    ColourChooser,
    ProgressBar,
    Status,
    Image,
    Tooltip,
    Canvas,
}

impl Role {
    /// The role's name in `dump access`, in the words screen readers use.
    fn name(self) -> &'static str {
        match self {
            Role::Window => "window",
            Role::Label => "text",
            Role::Button => "button",
            Role::CheckBox => "check box",
            Role::Switch => "switch",
            Role::RadioGroup => "radio group",
            Role::RadioButton => "radio button",
            Role::Slider => "slider",
            Role::SpinButton => "spin button",
            Role::TextInput => "text input",
            Role::TextBox => "text box",
            Role::ComboBox => "combo box",
            Role::MenuButton => "menu button",
            Role::Menu => "menu",
            Role::MenuItem => "menu item",
            Role::ListBox => "list box",
            Role::Choice => "option",
            Role::Dialog => "dialog",
            Role::TabList => "tab list",
            Role::Tab => "tab",
            Role::Table => "table",
            Role::ColumnHeader => "column header",
            Role::Row => "row",
            Role::Cell => "cell",
            Role::List => "list",
            Role::ListItem => "list item",
            Role::DateInput => "date input",
            Role::ColourWell => "colour well",
            Role::Calendar => "calendar",
            Role::ColourChooser => "colour chooser",
            Role::ProgressBar => "progress bar",
            Role::Status => "status",
            Role::Image => "image",
            Role::Tooltip => "tooltip",
            Role::Canvas => "canvas",
        }
    }
}

// What can be done to a piece, as bits.
const DO_CLICK: u16 = 1;
const DO_FOCUS: u16 = 2;
const DO_SET: u16 = 4;
const DO_STEP: u16 = 8;
const DO_SHOW: u16 = 16;
const DO_OPEN: u16 = 32;

/// One piece as a screen reader sees it.
#[derive(Clone, Debug, PartialEq)]
struct ANode {
    id: u64,
    role: Role,
    name: String,
    value: Option<String>,
    /// A slider's, number box's or progress bar's number: its value, lowest, highest and step.
    num: Option<[f64; 4]>,
    /// focused, disabled, checked, not checked, selected, expanded, collapsed, modal, sorted up,
    /// sorted down, invalid.
    states: Vec<&'static str>,
    /// A table or list: how many rows it has, and the first and last on the screen (from 1); and
    /// a table's columns.
    rows: Option<(usize, usize, usize)>,
    columns: Option<usize>,
    /// A row's place among all the rows, from 1.
    row_index: Option<usize>,
    /// An input's or text box's lines (where each starts and ends in its value), and its caret
    /// and the other end of its selection, when it has focus.
    lines: Vec<(usize, usize)>,
    caret: Option<(usize, usize)>,
    placeholder: String,
    /// A toast, read out politely when it shows.
    live: bool,
    bounds: [f32; 4],
    /// Where the piece is in the window's tree (empty for the window itself).
    path: Vec<usize>,
    actions: u16,
    children: Vec<usize>,
}

impl ANode {
    fn new(id: u64, role: Role, n: &Laid, path: &[usize]) -> ANode {
        ANode {
            id,
            role,
            name: String::new(),
            value: None,
            num: None,
            states: vec![],
            rows: None,
            columns: None,
            row_index: None,
            lines: vec![],
            caret: None,
            placeholder: String::new(),
            live: false,
            bounds: [n.x, n.y, n.w, n.h],
            path: path.to_vec(),
            actions: 0,
            children: vec![],
        }
    }
}

/// The window itself is always node 1.
const ROOT_ID: u64 = 1;

/// A child's id, from its parent's and what it's known by there (its place, or a row's place
/// among the rows). Never 0 or the window's.
fn child_id(parent: u64, part: u64) -> u64 {
    let mut h = parent ^ 0xcbf2_9ce4_8422_2325;
    for b in part.to_le_bytes() {
        h = (h ^ b as u64).wrapping_mul(0x0100_0000_01b3);
    }
    if h <= ROOT_ID {
        h + 2
    } else {
        h
    }
}

fn words_id(s: &str) -> u64 {
    s.bytes().fold(0x1_0000_0000u64, |h, b| (h ^ b as u64).wrapping_mul(0x0100_0000_01b3))
}

/// What a piece is called: its `label`, else its words, else its tip, else its icon's name.
fn name_of(n: &Laid, words: &str) -> String {
    [n.look.label.as_str(), words, n.look.tip.as_str(), n.look.icon.as_str()].into_iter().find(|s| !s.is_empty()).unwrap_or("").to_string()
}

/// The first words inside a piece, to name a dialog by.
fn first_words(n: &Laid) -> String {
    if n.kind == "text" && !n.text.is_empty() {
        return n.text.clone();
    }
    n.children.iter().map(first_words).find(|s| !s.is_empty()).unwrap_or_default()
}

/// Builds the tree of one window from its laid-out pieces.
struct Access<'u> {
    out: Vec<ANode>,
    st: &'u PaintState,
    focusable: Vec<Vec<usize>>,
    /// The dropdown or menu whose list is open, and the row picked in it; whether it's a
    /// right-click menu.
    popup: Option<(Vec<usize>, Option<usize>)>,
    ctx: bool,
    /// Whether the open list is a dropdown's (a list box), not a menu.
    popup_is_dropdown: bool,
    /// Which of the window's top pieces are layers over the view, and which are open lists.
    layers: HashMap<usize, Option<&'static str>>,
    /// The id of what has focus.
    focus: u64,
}

impl Access<'_> {
    fn add(&mut self, parent: usize, a: ANode) -> usize {
        let i = self.out.len();
        if a.states.contains(&"focused") {
            self.focus = a.id;
        }
        self.out.push(a);
        self.out[parent].children.push(i);
        i
    }

    fn base(&self, id: u64, role: Role, n: &Laid, path: &[usize]) -> ANode {
        let mut a = ANode::new(id, role, n, path);
        if n.look.disabled {
            a.states.push("disabled");
        }
        if self.st.focus.as_deref() == Some(path) {
            a.states.push("focused");
        }
        if self.focusable.iter().any(|p| p == path) {
            a.actions |= DO_FOCUS;
        }
        a.actions |= DO_SHOW;
        a
    }

    /// A piece and what's in it. Layout boxes add nothing of their own: what's in them goes to
    /// `parent`.
    fn walk(&mut self, n: &Laid, path: &mut Vec<usize>, id: u64, parent: usize) {
        let me = self.piece(n, path, id, parent);
        let into = me.unwrap_or(parent);
        match n.kind.as_str() {
            // These hold nothing a screen reader should see apart from themselves.
            "button" | "checkbox" | "switch" | "input" | "textarea" | "number" | "slider" | "dropdown" | "menu" | "datePicker" | "colourPicker"
            | "progress" | "image" | "icon" | "text" | "tabbtn" | "branchhead" | "radiobtn" | "toast" | "tip" | "celltext" | "header" | "option"
            | "item" | "divider" | "calendar" | "colourpop" | "line" => {}
            "table" | "list" => self.rows(n, path, id, into),
            _ => {
                for (i, c) in n.children.iter().enumerate() {
                    path.push(i);
                    self.walk(c, path, child_id(id, i as u64), into);
                    path.pop();
                }
            }
        }
    }

    /// A table's header and the rows on the screen, each known by its place among all the rows.
    fn rows(&mut self, n: &Laid, path: &mut Vec<usize>, id: u64, into: usize) {
        let first = first_row(n);
        if first == 1 {
            if let Some(head) = n.children.first() {
                path.push(0);
                let hid = child_id(id, 0);
                let mut a = self.base(hid, Role::Row, head, path);
                a.actions &= !DO_FOCUS;
                let at = self.add(into, a);
                for (i, h) in head.children.iter().enumerate() {
                    path.push(i);
                    self.walk(h, path, child_id(hid, i as u64), at);
                    path.pop();
                }
                path.pop();
            }
        }
        for (i, r) in n.children.iter().enumerate().skip(first) {
            if !row_shows(n, r) {
                continue;
            }
            path.push(i);
            self.walk(r, path, child_id(id, (1u64 << 40) + r.index as u64), into);
            path.pop();
        }
    }

    /// The node for one piece, if it's one a screen reader names.
    fn piece(&mut self, n: &Laid, path: &[usize], id: u64, parent: usize) -> Option<usize> {
        let k = n.kind.as_str();
        let checked = |a: &mut ANode| {
            a.states.push(if matches!(n.value, Some(SV::Truth(true))) { "checked" } else { "not checked" });
        };
        let open_here = |s: &Self| s.popup.as_ref().is_some_and(|(p, _)| p.as_slice() == path) && !s.ctx;
        let a = match k {
            "text" if n.text.is_empty() => return None,
            "text" => {
                let mut a = self.base(id, Role::Label, n, path);
                a.name = name_of(n, &n.text);
                a
            }
            "celltext" => {
                let mut a = self.base(id, Role::Cell, n, path);
                a.name = n.text.clone();
                a
            }
            "cell" => self.base(id, Role::Cell, n, path),
            "button" => {
                let mut a = self.base(id, Role::Button, n, path);
                a.name = name_of(n, &n.text);
                a.actions |= DO_CLICK;
                a
            }
            "checkbox" | "switch" => {
                let mut a = self.base(id, if k == "checkbox" { Role::CheckBox } else { Role::Switch }, n, path);
                a.name = name_of(n, &n.text);
                checked(&mut a);
                a.actions |= DO_CLICK;
                a
            }
            "radio" => {
                let mut a = self.base(id, Role::RadioGroup, n, path);
                a.name = name_of(n, "");
                a.value = match &n.value {
                    Some(SV::Text(t)) => Some(t.clone()),
                    _ => None,
                };
                a
            }
            "radiobtn" => {
                let mut a = self.base(id, Role::RadioButton, n, path);
                a.name = name_of(n, &n.text);
                checked(&mut a);
                a.actions |= DO_CLICK;
                a
            }
            "input" | "textarea" | "number" | "datePicker" => {
                let role = match k {
                    "input" => Role::TextInput,
                    "textarea" => Role::TextBox,
                    "number" => Role::SpinButton,
                    _ => Role::DateInput,
                };
                let mut a = self.base(id, role, n, path);
                a.name = name_of(n, "");
                a.placeholder = n.look.placeholder.clone();
                let t = value_text(n).to_string();
                a.lines = if k == "textarea" { n.spans.clone() } else { vec![(0, t.len())] };
                if a.states.contains(&"focused") {
                    let (c, anchor) = (self.st.caret.caret.min(t.len()), self.st.caret.anchor.min(t.len()));
                    a.caret = Some((c, anchor));
                }
                if k == "number" {
                    let l = &n.look;
                    a.num = Some([n.num, l.min, l.max, l.num_step]);
                    a.actions |= DO_STEP;
                }
                if n.bad {
                    a.states.push("invalid");
                }
                if k == "datePicker" {
                    let open = self.st.picker.as_ref().is_some_and(|p| p.owner == path);
                    a.states.push(if open { "expanded" } else { "collapsed" });
                    a.actions |= DO_OPEN;
                }
                a.value = Some(t);
                if n.action.is_some() {
                    a.actions |= DO_SET | DO_CLICK;
                }
                a
            }
            "slider" | "progress" => {
                let mut a = self.base(id, if k == "slider" { Role::Slider } else { Role::ProgressBar }, n, path);
                a.name = name_of(n, "");
                let l = &n.look;
                let v = value_num(n) as f64;
                let step = if k == "slider" { l.step as f64 } else { 0.0 };
                a.num = Some([v, l.from.min(l.to) as f64, l.from.max(l.to) as f64, step]);
                a.value = Some(num_text(v));
                if k == "slider" && n.action.is_some() {
                    a.actions |= DO_SET | DO_STEP;
                }
                a
            }
            "dropdown" => {
                let mut a = self.base(id, Role::ComboBox, n, path);
                a.name = name_of(n, "");
                a.value = Some(n.text.clone());
                a.states.push(if open_here(self) { "expanded" } else { "collapsed" });
                if n.action.is_some() {
                    a.actions |= DO_CLICK | DO_OPEN;
                }
                a
            }
            "menu" => {
                let mut a = self.base(id, Role::MenuButton, n, path);
                a.name = name_of(n, &n.text);
                a.states.push(if open_here(self) { "expanded" } else { "collapsed" });
                if !n.look.disabled {
                    a.actions |= DO_CLICK | DO_OPEN;
                }
                a
            }
            "colourPicker" => {
                let mut a = self.base(id, Role::ColourWell, n, path);
                a.name = name_of(n, "");
                if let Some(SV::Rgba(c)) = n.value {
                    a.value = Some(rgba_hex(c, n.look.alpha));
                }
                let open = self.st.picker.as_ref().is_some_and(|p| p.owner == path);
                a.states.push(if open { "expanded" } else { "collapsed" });
                if n.action.is_some() {
                    a.actions |= DO_CLICK | DO_SET | DO_OPEN;
                }
                a
            }
            "image" | "icon" => {
                let mut a = self.base(id, Role::Image, n, path);
                // A picture's name is its label or tip; an icon's is its own name if it has neither.
                a.name = name_of(n, "");
                if a.name.is_empty() && k == "icon" {
                    a.name = n.text.clone();
                }
                a
            }
            "canvas" => {
                let mut a = self.base(id, Role::Canvas, n, path);
                a.name = name_of(n, "");
                a
            }
            "box" | "circle" if n.action.is_some() => {
                let mut a = self.base(id, Role::Button, n, path);
                a.name = name_of(n, "");
                a.actions |= DO_CLICK;
                a
            }
            "tabbar" => self.base(id, Role::TabList, n, path),
            "tabbtn" => {
                let mut a = self.base(id, Role::Tab, n, path);
                a.name = name_of(n, &n.text);
                if matches!(n.value, Some(SV::Truth(true))) {
                    a.states.push("selected");
                }
                a.actions |= DO_CLICK;
                a
            }
            "branchhead" => {
                let mut a = self.base(id, Role::Button, n, path);
                a.name = name_of(n, &n.text);
                a.states.push(if matches!(n.value, Some(SV::Truth(true))) { "expanded" } else { "collapsed" });
                a.actions |= DO_CLICK | DO_OPEN;
                a
            }
            "table" | "list" => {
                let mut a = self.base(id, if k == "table" { Role::Table } else { Role::List }, n, path);
                a.name = name_of(n, "");
                if let Some(info) = &n.list {
                    let shown: Vec<usize> = n.children.iter().skip(first_row(n)).filter(|r| row_shows(n, r)).map(|r| r.index + 1).collect();
                    a.rows = Some((info.count, shown.first().copied().unwrap_or(0), shown.last().copied().unwrap_or(0)));
                }
                if k == "table" {
                    a.columns = n.children.first().map(|h| h.children.len());
                }
                a
            }
            "header" => {
                let mut a = self.base(id, Role::ColumnHeader, n, path);
                a.name = n.text.clone();
                match &n.value {
                    Some(SV::Text(t)) if t == "rising" => a.states.push("sorted up"),
                    Some(SV::Text(t)) if t == "falling" => a.states.push("sorted down"),
                    _ => {}
                }
                if n.whole {
                    a.actions |= DO_CLICK;
                }
                a
            }
            "tablerow" | "listrow" => {
                let mut a = self.base(id, if k == "tablerow" { Role::Row } else { Role::ListItem }, n, path);
                a.row_index = Some(n.index + 1);
                if matches!(n.value, Some(SV::Truth(true))) {
                    a.states.push("selected");
                }
                if k == "listrow" {
                    a.name = first_words(n);
                }
                a.actions |= DO_CLICK | DO_FOCUS;
                a
            }
            "dialog" => {
                let mut a = self.base(id, Role::Dialog, n, path);
                a.name = name_of(n, &first_words(n));
                a.states.push("modal");
                a
            }
            // An open list of choices or menu items.
            "scroll" if path.len() == 2 && self.layer_kind(path) == Some("popup") => {
                let dropdown = !self.ctx && self.popup_is_dropdown;
                let mut a = self.base(id, if dropdown { Role::ListBox } else { Role::Menu }, n, path);
                a.actions &= !DO_FOCUS;
                a
            }
            "option" | "item" => {
                let mut a = self.base(id, if k == "option" { Role::Choice } else { Role::MenuItem }, n, path);
                a.name = name_of(n, &n.text);
                let row = path.last().copied();
                if self.popup.as_ref().is_some_and(|(_, hl)| *hl == row && row.is_some()) {
                    a.states.push("focused");
                }
                if k == "option" && n.look.weight >= 600.0 {
                    a.states.push("selected");
                }
                if n.action.is_some() {
                    a.actions |= DO_CLICK;
                } else if !a.states.contains(&"disabled") {
                    a.states.push("disabled");
                }
                a
            }
            "calendar" | "colourpop" => {
                let mut a = self.base(id, if k == "calendar" { Role::Calendar } else { Role::ColourChooser }, n, path);
                if let Some(p) = &self.st.picker {
                    if p.colour {
                        a.name = "Colour".into();
                        a.value = Some(rgba_hex([p.rgb[0], p.rgb[1], p.rgb[2], p.a], n.look.alpha));
                    } else {
                        a.name = cal_title(p);
                        a.value = Some(cal_text(p.cursor));
                    }
                }
                a
            }
            "toast" => {
                let mut a = self.base(id, Role::Status, n, path);
                a.name = n.text.clone();
                a.live = true;
                a.actions |= DO_CLICK;
                a
            }
            "tip" => {
                let mut a = self.base(id, Role::Tooltip, n, path);
                a.name = n.text.clone();
                a
            }
            _ => return None,
        };
        Some(self.add(parent, a))
    }

    fn layer_kind(&self, path: &[usize]) -> Option<&'static str> {
        self.layers.get(path.first()?).copied().flatten()
    }
}

impl<'a, 'b> Ui<'a, 'b> {
    /// The window's tree for screen readers: the window first, then what's in it. Behind an open
    /// dialog nothing can be used, so only the dialog and what's over it are in the tree.
    fn access_tree(&self) -> (Vec<ANode>, u64) {
        let mut focusable_paths = Vec::new();
        for i in self.reach() {
            focusable(&self.tree.children[i], &mut vec![i], &mut focusable_paths);
        }
        let mut root = ANode::new(ROOT_ID, Role::Window, &self.tree, &[]);
        root.name = self.s.title.clone();
        root.bounds = [0.0, 0.0, self.w, self.h];
        let popup_is_dropdown = self.popup.as_ref().and_then(|(p, _)| node_at(&self.tree, p)).is_some_and(|o| o.kind == "dropdown");
        let layers: HashMap<usize, Option<&'static str>> = self
            .tree
            .children
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let k = match (c.kind.as_str(), c.text.as_str()) {
                    ("layer", "popup") => Some("popup"),
                    ("layer", _) => Some("layer"),
                    _ => None,
                };
                (i, k)
            })
            .collect();
        // What painting knows: focus, the caret, and the picker that's open.
        let st = self.paint_state();
        let mut b = Access {
            out: vec![root],
            st: &st,
            focusable: focusable_paths,
            popup: self.popup.clone(),
            ctx: self.ctx_at.is_some(),
            focus: ROOT_ID,
            popup_is_dropdown,
            layers,
        };
        let mut seen: HashMap<String, u64> = HashMap::new();
        for i in self.reach() {
            let c = &self.tree.children[i];
            // A layer is known by what it is (a dialog, a list, a tip), so its pieces keep their
            // ids when another layer comes or goes.
            let part = if c.kind == "layer" {
                let nth = seen.entry(c.text.clone()).or_insert(0);
                *nth += 1;
                words_id(&c.text) + *nth
            } else {
                i as u64
            };
            b.walk(c, &mut vec![i], child_id(ROOT_ID, part), 0);
        }
        let focus = b.focus;
        (b.out, focus)
    }

    /// Does what a screen reader asked of a piece, through the same paths as the mouse and keys.
    fn access_do(&mut self, a: &ANode, what: Act) -> Result<(), String> {
        let path = a.path.clone();
        let Some(n) = node_at(&self.tree, &path).cloned() else { return Ok(()) };
        if n.look.disabled {
            return Ok(());
        }
        // A row is picked, or opened, through its table or list.
        let row = list_on_path(&self.tree, &path).filter(|(d, _)| path.len() == d + 1 && matches!(n.kind.as_str(), "tablerow" | "listrow"));
        match what {
            Act::Show => self.show_piece(&path)?,
            Act::Focus => {
                if let Some((d, _)) = row {
                    self.set_focus(Some(path[..d].to_vec()), true);
                    self.pick_row(&path[..d], n.index, false)?;
                } else if a.actions & DO_FOCUS != 0 {
                    self.show_piece(&path)?;
                    self.set_focus(Some(path), true);
                }
            }
            Act::Click => {
                if let Some((d, _)) = row {
                    return self.pick_row(&path[..d], n.index, false);
                }
                if a.actions & DO_CLICK == 0 {
                    return Ok(());
                }
                self.show_piece(&path)?;
                let Some(n) = node_at(&self.tree, &path) else { return Ok(()) };
                let (x, y) = match n.kind.as_str() {
                    // A date picker opens from its calendar icon.
                    "datePicker" => (n.x + n.w - DATE_ICON_W / 2.0, n.y + n.h / 2.0),
                    // A checkbox or switch with words is clicked on its box.
                    "checkbox" | "switch" if !n.text.is_empty() => (n.x + 9.0, n.y + n.h / 2.0),
                    _ => (n.x + n.w / 2.0, n.y + n.h / 2.0),
                };
                self.press(x, y, Mods::default())?;
                self.release(x, y)?;
            }
            Act::Open(open) => {
                let is_open = a.states.contains(&"expanded");
                if is_open != open {
                    match n.kind.as_str() {
                        "branchhead" => self.toggle(&path, Some(open))?,
                        "dropdown" | "menu" if open => self.open_popup(&path, true),
                        "dropdown" | "menu" => {
                            self.popup = None;
                            self.relayout();
                        }
                        "datePicker" | "colourPicker" => self.open_picker(&path),
                        _ => {}
                    }
                }
            }
            Act::Set(text) => match n.kind.as_str() {
                "input" | "textarea" | "number" | "datePicker" if n.action.is_some() => {
                    self.set_text(&path, text)?;
                    if self.st.focus.as_deref() == Some(path.as_slice()) {
                        let t = self.focused().map(|n| value_text(n).to_string()).unwrap_or_default();
                        self.st.caret = Caret::at(t.len());
                    }
                }
                "slider" => {
                    if let Ok(v) = text.trim().parse::<f64>() {
                        self.slide_value(&path, v)?;
                    }
                }
                "colourPicker" if n.action.is_some() => {
                    if let Some(c) = hex_parse(&text, n.look.alpha) {
                        self.run(&path, SV::Rgba(c))?;
                    }
                }
                _ => {}
            },
            Act::Step(up) => match n.kind.as_str() {
                "number" => self.spin(&path, if up { 1.0 } else { -1.0 })?,
                "slider" => {
                    let l = &n.look;
                    let step = if l.step > 0.0 { l.step } else { (l.to - l.from).abs() / 100.0 };
                    let dir = if l.to >= l.from { 1.0 } else { -1.0 };
                    let v = value_num(&n) + if up { step * dir } else { -step * dir };
                    self.slide_value(&path, v as f64)?;
                }
                _ => {}
            },
        }
        Ok(())
    }

    /// A slider moved to this value, kept in its range and on its steps.
    fn slide_value(&mut self, path: &[usize], v: f64) -> Result<(), String> {
        let Some(n) = node_at(&self.tree, path) else { return Ok(()) };
        if n.action.is_none() {
            return Ok(());
        }
        let v = snap(&n.look, v as f32);
        if (v - value_num(n)).abs() < 1e-6 {
            return Ok(());
        }
        self.run(path, SV::Num(v as f64))
    }

    /// Scrolls so a piece shows: a row of a table or list into the room for rows, and anything
    /// else into the scrolls around it.
    fn show_piece(&mut self, path: &[usize]) -> Result<(), String> {
        if let Some((d, info)) = list_on_path(&self.tree, path) {
            if let (Some(list), Some(r)) = (node_at(&self.tree, &path[..d]).cloned(), node_at(&self.tree, &path[..=d]).cloned()) {
                let (_, by, _, bh) = list_body(&list);
                let rh = with_list(info.id, |st| st.row_h.max(1.0));
                let to = if r.y < by {
                    Some(r.index as f32 * rh)
                } else if r.y + r.h > by + bh {
                    Some((r.index + 1) as f32 * rh - bh)
                } else {
                    None
                };
                if let Some(to) = to {
                    self.list_scroll(&path[..d], to)?;
                }
            }
            return Ok(());
        }
        let was = self.st.focus.replace(path.to_vec());
        self.reveal();
        self.st.focus = was;
        Ok(())
    }
}

/// What a screen reader asks for.
#[derive(Clone, Debug, PartialEq)]
enum Act {
    Click,
    Focus,
    Show,
    /// Opens (true) or closes a list, menu, picker or branch.
    Open(bool),
    /// A new value: text, a number, a date or a colour's hex.
    Set(String),
    /// A step up (true) or down.
    Step(bool),
}

/// The tree as `dump access` prints it: each piece's role, name, value and states, indented.
fn access_dump(tree: &[ANode], at: usize, depth: usize, out: &mut String) {
    let a = &tree[at];
    let mut line = format!("{}{}", "  ".repeat(depth), a.role.name());
    if !a.name.is_empty() || matches!(a.role, Role::Button | Role::Window | Role::Image) {
        line.push_str(&format!(" {:?}", a.name));
    }
    if let Some(v) = &a.value {
        line.push_str(&format!(" = {v:?}"));
    }
    if let Some([_, lo, hi, step]) = a.num {
        let range = |x: f64| if x.is_finite() && x.abs() < 1e300 { num_text(x) } else { "any".into() };
        line.push_str(&format!(" ({} to {}", range(lo), range(hi)));
        if step > 0.0 {
            line.push_str(&format!(", step {}", num_text(step)));
        }
        line.push(')');
    }
    if let Some((count, a0, b0)) = a.rows {
        let rows = if count == 1 { "row" } else { "rows" };
        if a0 > 0 {
            line.push_str(&format!(", {count} {rows}, showing {a0}\u{2013}{b0}"));
        } else {
            line.push_str(&format!(", {count} {rows}"));
        }
    }
    if let Some(c) = a.columns {
        line.push_str(&format!(", {c} column{}", if c == 1 { "" } else { "s" }));
    }
    if let Some(r) = a.row_index {
        line.push_str(&format!(" {r}"));
    }
    if !a.placeholder.is_empty() && a.value.as_deref() == Some("") {
        line.push_str(&format!(" placeholder {:?}", a.placeholder));
    }
    let mut states: Vec<String> = a.states.iter().map(|s| s.to_string()).collect();
    if let (Some((c, anchor)), Some(v)) = (a.caret, &a.value) {
        let chars = |i: usize| v[..i.min(v.len())].chars().count();
        if c == anchor {
            states.push(format!("caret {}", chars(c)));
        } else {
            states.push(format!("selected {}..{}", chars(c.min(anchor)), chars(c.max(anchor))));
        }
    }
    if a.live {
        states.push("read out politely".into());
    }
    if !states.is_empty() {
        line.push_str(&format!(" ({})", states.join(", ")));
    }
    out.push_str(&line);
    out.push('\n');
    for &c in &a.children {
        access_dump(tree, c, depth + 1, out);
    }
}

impl<'a, 'b> Wins<'a, 'b> {
    /// What `dump access` shows: each open window's tree for screen readers.
    fn dump_access(&mut self) -> String {
        let mut out = String::new();
        for i in 0..self.list.len() {
            let (tree, _) = self.with(i, |u| u.access_tree());
            access_dump(&tree, 0, 0, &mut out);
        }
        out
    }
}

impl<'a, 'b> Ui<'a, 'b> {
    /// `act click Save`, `act set Name Ada`, `act up Volume`: does what a screen reader would ask
    /// of the first piece with that name (or role, like `slider`), for tests.
    fn access_test(&mut self, rest: &str) -> Result<(), String> {
        let words = test_words(rest);
        let (Some(what), Some(target)) = (words.first(), words.get(1)) else {
            println!("(act takes what to do and a piece's name: act click Save)");
            return Ok(());
        };
        let value = words[2..].join(" ");
        let act = match what.as_str() {
            "click" => Act::Click,
            "focus" => Act::Focus,
            "show" => Act::Show,
            "open" => Act::Open(true),
            "close" => Act::Open(false),
            "set" => Act::Set(value),
            "up" => Act::Step(true),
            "down" => Act::Step(false),
            _ => {
                println!("(act does click, focus, show, open, close, set, up or down, not {what})");
                return Ok(());
            }
        };
        let (tree, _) = self.access_tree();
        // By name first, then by role ("slider", "row 5000"): what a screen reader would find.
        let found = tree.iter().find(|a| a.id != ROOT_ID && a.name == *target).or_else(|| {
            let (role, nth) = match target.rsplit_once(' ') {
                Some((r, n)) if n.parse::<usize>().is_ok() => (r, n.parse::<usize>().unwrap_or(1)),
                _ => (target.as_str(), 1),
            };
            match role {
                "row" => tree.iter().find(|a| a.row_index == Some(nth)),
                _ => tree.iter().filter(|a| a.role.name() == role).nth(nth.saturating_sub(1)),
            }
        });
        match found.cloned() {
            Some(a) => self.access_do(&a, act),
            None => {
                println!("(there's no piece called \"{target}\" for a screen reader)");
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod access_tests {
    use super::*;

    #[test]
    fn ids_are_steady_and_never_the_window() {
        assert_eq!(child_id(ROOT_ID, 3), child_id(ROOT_ID, 3));
        assert_ne!(child_id(ROOT_ID, 3), child_id(ROOT_ID, 4));
        assert_ne!(child_id(child_id(ROOT_ID, 0), (1 << 40) + 5), child_id(child_id(ROOT_ID, 0), (1 << 40) + 6));
        for part in 0..10_000u64 {
            assert!(child_id(part, part * 7) > ROOT_ID);
        }
    }

    #[test]
    fn names_come_from_label_then_words_then_tip_then_icon() {
        let mut n = lay(&Node::new("button"));
        n.look.icon = "trash".into();
        assert_eq!(name_of(&n, ""), "trash");
        n.look.tip = "Throw away".into();
        assert_eq!(name_of(&n, ""), "Throw away");
        assert_eq!(name_of(&n, "Delete"), "Delete");
        n.look.label = "Delete the note".into();
        assert_eq!(name_of(&n, "Delete"), "Delete the note");
    }
}
