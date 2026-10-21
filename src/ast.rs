//! The shape of a Lust program after parsing.

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Pos {
    /// The file's number in its top bits (see FILE_SHIFT), and its line in the rest,
    /// so one number tells both and sorting goes file by file.
    pub line: u32,
    pub col: u32,
}

pub const FILE_SHIFT: u32 = 20;

impl Pos {
    /// Which file of the project this is in: 0 is the one being run.
    pub fn file(&self) -> usize {
        (self.line >> FILE_SHIFT) as usize
    }
    /// The line within its file.
    pub fn row(&self) -> u32 {
        self.line & ((1 << FILE_SHIFT) - 1)
    }
}

/// Splits a packed line into its file and its line.
pub fn unpack(line: u32) -> (usize, u32) {
    let p = Pos { line, col: 0 };
    (p.file(), p.row())
}

/// Things from other files that share a name get a mark and a number inside Lust, like
/// Player·2. People only ever see the part before the mark.
pub const MARK: char = '\u{b7}';

pub fn shown(name: &str) -> &str {
    name.split(MARK).next().unwrap_or(name)
}

/// Takes the marks out of a message.
pub fn unmark(msg: &str) -> String {
    let mut out = String::new();
    let mut skipping = false;
    for c in msg.chars() {
        if c == MARK {
            skipping = true;
        } else if skipping && c.is_ascii_digit() {
        } else {
            skipping = false;
            out.push(c);
        }
    }
    out
}

pub struct Import {
    pub path: String,
    pub path_pos: Pos,
    /// get brings methods; fetch brings classes, objects and states.
    pub is_get: bool,
    pub names: Vec<(String, Pos)>,
}

/// A type as written in the source: `int`, `Player array`, `hook int(int n)`.
#[derive(Clone)]
pub enum TypeRef {
    Named(String, Pos),
    Array(Box<TypeRef>),
    Map(Box<TypeRef>),
    CMap(Vec<Column>),
    Hook { ret: Option<Box<TypeRef>>, params: Vec<HookParam> },
}

#[derive(Clone)]
#[allow(dead_code)] // positions kept for messages to come
pub struct Column {
    pub ty: TypeRef,
    pub name: String,
    pub pos: Pos,
    /// The key column's key-maker: `cMap(int id = rollId, …)`.
    pub maker: Option<Expr>,
}

impl TypeRef {
    /// The key-maker of a cMap type, if it has one.
    pub fn key_maker(&self) -> Option<&Expr> {
        match self {
            TypeRef::CMap(cols) => cols.first().and_then(|c| c.maker.as_ref()),
            _ => None,
        }
    }
}

#[derive(Clone)]
pub struct HookParam {
    pub ty: TypeRef,
    pub name: Option<String>,
}

impl std::fmt::Display for TypeRef {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            TypeRef::Named(n, _) => write!(f, "{n}"),
            TypeRef::Array(t) => write!(f, "{t} array"),
            TypeRef::Map(t) => write!(f, "{t} map"),
            TypeRef::CMap(cols) => {
                let c: Vec<String> = cols.iter().map(|c| format!("{} {}", c.ty, c.name)).collect();
                write!(f, "cMap({})", c.join(", "))
            }
            TypeRef::Hook { ret, params } => {
                write!(f, "hook")?;
                if let Some(r) = ret {
                    write!(f, " {r}")?;
                }
                if !params.is_empty() {
                    let p: Vec<String> = params
                        .iter()
                        .map(|p| match &p.name {
                            Some(n) => format!("{} {n}", p.ty),
                            None => p.ty.to_string(),
                        })
                        .collect();
                    write!(f, "({})", p.join(", "))?;
                }
                Ok(())
            }
        }
    }
}

#[allow(dead_code)]
pub struct Program {
    pub items: Vec<Item>,
    pub body: Vec<Stmt>,
    pub uses_lua: bool,
    pub imports: Vec<Import>,
    pub passes: Vec<(String, Pos)>,
    /// The next expression id after this file's, so the next file's ids don't overlap.
    pub next_id: usize,
}

pub enum Item {
    State(StateDecl),
    Class(ClassDecl),
    Method(MethodDecl),
}

pub struct StateDecl {
    pub name: String,
    pub pos: Pos,
    pub options: Vec<StateOption>,
}

#[allow(dead_code)]
pub struct StateOption {
    pub name: String,
    pub pos: Pos,
    pub children: Vec<StateOption>,
}

pub struct ClassDecl {
    pub name: String,
    pub pos: Pos,
    pub is_object: bool,
    pub parent: Option<(String, Pos)>,
    pub fields: Vec<FieldDecl>,
    pub methods: Vec<MethodDecl>,
}

pub struct FieldDecl {
    /// None for `state is GameState.Idle;`, whose type comes from the option.
    pub ty: Option<TypeRef>,
    pub name: String,
    pub pos: Pos,
    pub default: Option<Expr>,
}

#[derive(Clone)]
pub struct MethodDecl {
    pub name: String,
    pub pos: Pos,
    pub ret: Option<TypeRef>,
    pub params: Vec<Param>,
    pub body: Vec<Stmt>,
}

#[derive(Clone)]
pub struct Param {
    pub ty: TypeRef,
    pub name: String,
    pub pos: Pos,
}

#[derive(Clone)]
pub enum Stmt {
    Var {
        ty: TypeRef,
        name: String,
        pos: Pos,
        value: Expr,
    },
    Assign {
        target: Expr,
        value: Expr,
        pos: Pos,
    },
    SetState {
        target: Expr,
        option: Expr,
        pos: Pos,
    },
    Expr(Expr),
    If {
        branches: Vec<(Expr, Vec<Stmt>)>,
        otherwise: Option<Vec<Stmt>>,
        pos: Pos,
    },
    Loop {
        kind: LoopKind,
        body: Vec<Stmt>,
        pos: Pos,
    },
    Stop(Pos),
    Skip(Pos),
    Return(Option<Expr>, Pos),
    Try {
        body: Vec<Stmt>,
        err: (String, Pos),
        handler: Vec<Stmt>,
        pos: Pos,
    },
    Lua {
        code: String,
        pos: Pos,
    },
}

impl Stmt {
    pub fn pos(&self) -> Pos {
        match self {
            Stmt::Var { pos, .. }
            | Stmt::Assign { pos, .. }
            | Stmt::SetState { pos, .. }
            | Stmt::If { pos, .. }
            | Stmt::Loop { pos, .. }
            | Stmt::Try { pos, .. }
            | Stmt::Lua { pos, .. } => *pos,
            Stmt::Stop(p) | Stmt::Skip(p) | Stmt::Return(_, p) => *p,
            Stmt::Expr(e) => e.pos,
        }
    }
}

#[derive(Clone)]
pub enum LoopKind {
    Forever,
    Times(Expr),
    While(Expr),
    Each { vars: Vec<(TypeRef, String, Pos)>, source: Expr },
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    Ne,
    Same,
    NotSame,
    Lt,
    Gt,
    Le,
    Ge,
    And,
    Or,
}

impl BinOp {
    pub fn symbol(self) -> &'static str {
        match self {
            BinOp::Add => "+",
            BinOp::Sub => "-",
            BinOp::Mul => "*",
            BinOp::Div => "/",
            BinOp::Mod => "%",
            BinOp::Eq => "==",
            BinOp::Ne => "!=",
            BinOp::Same => "===",
            BinOp::NotSame => "!==",
            BinOp::Lt => "<",
            BinOp::Gt => ">",
            BinOp::Le => "<=",
            BinOp::Ge => ">=",
            BinOp::And => "&&",
            BinOp::Or => "||",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Conv {
    Text,
    Int,
    Float,
    IsNumber,
    Lines,
}

#[derive(Clone)]
pub struct Expr {
    pub id: usize,
    pub pos: Pos,
    pub kind: ExprKind,
}

#[derive(Clone)]
pub enum ExprKind {
    Int(i64),
    Float(f64),
    Str(String),
    Bool(bool),
    Nothing,
    Name(String),
    /// `a.b`
    Field(Box<Expr>, String, Pos),
    /// `a:b(args)`
    Method(Box<Expr>, String, Vec<Expr>, Pos),
    /// `a:b` without ( ): the method kept as a hook value
    MethodRef(Box<Expr>, String, Pos),
    /// `name(args)`
    Call(String, Vec<Expr>),
    /// `a[i]`
    Index(Box<Expr>, Box<Expr>),
    /// `#a`
    Len(Box<Expr>),
    Neg(Box<Expr>),
    Not(Box<Expr>),
    Bin(BinOp, Box<Expr>, Box<Expr>, Pos),
    Either(Box<Expr>, Box<Expr>, Box<Expr>),
    /// `[a, b]`, `["k" v]`, `[[1, "a"], …]`
    List {
        items: Vec<Expr>,
        keys: Option<Vec<Expr>>,
    },
    New {
        class: String,
        class_pos: Pos,
        inits: Vec<(String, Pos, Expr)>,
    },
    NewHook(Vec<Stmt>),
    Like(Box<Expr>, String, Pos),
    Convert(Box<Expr>, Conv, Pos),
}
