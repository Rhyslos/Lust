// Lust's runtime for compiled programs. It goes into every program Lust builds,
// as `mod rt`, and prints and fails exactly the way the Lua runtime does.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

thread_local! {
    static LINE: Cell<u32> = Cell::new(0);
    static DEPTH: Cell<u32> = Cell::new(0);
    static RNG: Cell<u64> = Cell::new(0);
}

/// The Lust line now running, for error messages.
pub fn at(line: u32) {
    LINE.with(|c| c.set(line));
}

pub struct Fail(pub String);

pub fn fail(msg: String) -> ! {
    std::panic::panic_any(Fail(msg))
}

pub fn unreachable() -> ! {
    fail("a method ended without returning anything.".into())
}

/// Counts calls inside calls, so a method calling itself forever stops with a message.
pub struct Depth;
impl Depth {
    pub fn enter() -> Depth {
        let d = DEPTH.with(|c| {
            c.set(c.get() + 1);
            c.get()
        });
        if d > 100_000 {
            fail("methods called each other too deeply. That usually means one calls itself with no way to stop.".into());
        }
        Depth
    }
}
impl Drop for Depth {
    fn drop(&mut self) {
        DEPTH.with(|c| c.set(c.get().saturating_sub(1)));
    }
}

fn payload_text(p: &(dyn std::any::Any + Send)) -> String {
    if let Some(Fail(m)) = p.downcast_ref::<Fail>() {
        return m.clone();
    }
    let raw = if let Some(s) = p.downcast_ref::<String>() {
        s.clone()
    } else if let Some(s) = p.downcast_ref::<&str>() {
        s.to_string()
    } else {
        "something went wrong.".into()
    };
    if raw.contains("overflow") {
        return "a number got too big for an int.".into();
    }
    raw
}

/// Runs the program on a thread with a big stack, and turns failures into Lust messages.
pub fn main(program: fn(), file: &'static str) {
    std::panic::set_hook(Box::new(|_| {}));
    let t = std::thread::Builder::new()
        .stack_size(512 * 1024 * 1024)
        .spawn(move || match std::panic::catch_unwind(program) {
            Ok(()) => 0,
            Err(p) => {
                use std::io::Write;
                let _ = std::io::stdout().flush();
                let msg = payload_text(&*p);
                let line = LINE.with(|c| c.get());
                if line > 0 {
                    eprintln!("{file}:{line}  {msg}");
                } else {
                    eprintln!("{file}  {msg}");
                }
                1
            }
        })
        .expect("starting the program");
    let code = t.join().unwrap_or(1);
    std::process::exit(code);
}

/// try { … } error (string message) { … }: the message, if something failed.
pub fn attempt(f: impl FnOnce()) -> Option<String> {
    let depth = DEPTH.with(|c| c.get());
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
        Ok(()) => None,
        Err(p) => {
            DEPTH.with(|c| c.set(depth));
            Some(payload_text(&*p))
        }
    }
}

// ----- things and hooks that may be nothing -----

pub fn obj<T: ?Sized>(o: &Option<Rc<T>>) -> Rc<T> {
    match o {
        Some(r) => r.clone(),
        None => fail("this is nothing, so it has no fields or methods. Check first: is it !== nothing?".into()),
    }
}

pub fn hk<T: ?Sized>(h: &Option<Rc<T>>) -> Rc<T> {
    match h {
        Some(r) => r.clone(),
        None => fail("this hook is nothing, so it can't be called. Check first: is it !== nothing?".into()),
    }
}

pub fn same<T: ?Sized>(a: &Option<Rc<T>>, b: &Option<Rc<T>>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(x), Some(y)) => std::ptr::addr_eq(Rc::as_ptr(x), Rc::as_ptr(y)),
        _ => false,
    }
}

pub trait HasClass {
    fn class_id(&self) -> u32;
}

pub fn like<T: HasClass>(o: &Option<Rc<RefCell<T>>>, ids: &[u32]) -> bool {
    match o {
        None => false,
        Some(r) => ids.contains(&r.borrow().class_id()),
    }
}

// ----- text -----

pub fn itext(v: i64) -> String {
    v.to_string()
}

fn trim_zeros(s: &str) -> String {
    if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        s.to_string()
    }
}

/// C's %.15g, which is what the Lua runtime uses.
fn g15(v: f64) -> String {
    if v == 0.0 {
        return "0".into();
    }
    let e = format!("{:.14e}", v);
    let (mant, exp) = e.split_once('e').unwrap();
    let exp: i32 = exp.parse().unwrap();
    if exp < -4 || exp >= 15 {
        let sign = if exp < 0 { '-' } else { '+' };
        format!("{}e{}{:02}", trim_zeros(mant), sign, exp.abs())
    } else {
        let decimals = (14 - exp).max(0) as usize;
        trim_zeros(&format!("{:.*}", decimals, v))
    }
}

pub fn ftext(v: f64) -> String {
    if v.is_nan() {
        return "not a number".into();
    }
    if v == f64::INFINITY {
        return "infinity".into();
    }
    if v == f64::NEG_INFINITY {
        return "-infinity".into();
    }
    if v == v.floor() && v > -1e15 && v < 1e15 {
        return format!("{:.1}", v);
    }
    let s = g15(v);
    if !s.contains('.') && !s.contains('e') {
        format!("{s}.0")
    } else {
        s
    }
}

pub fn btext(v: bool) -> String {
    if v { "true".into() } else { "false".into() }
}

pub fn qtext(s: &str) -> String {
    format!("\"{s}\"")
}

pub fn cat(a: String, b: String) -> String {
    a + &b
}

pub fn slen(s: &str) -> i64 {
    s.chars().count() as i64
}

pub fn print(s: String) {
    println!("{s}");
}

pub fn hshow<T: ?Sized>(h: &Option<Rc<T>>) -> String {
    if h.is_some() { "hook".into() } else { "nothing".into() }
}

// ----- numbers -----

pub fn idiv(a: i64, b: i64) -> i64 {
    if b == 0 {
        fail("dividing a whole number by zero doesn't work.".into());
    }
    a / b
}

pub fn imod(a: i64, b: i64) -> i64 {
    if b == 0 {
        fail("the remainder after dividing by zero doesn't exist.".into());
    }
    a % b
}

pub fn toint(v: f64) -> i64 {
    if v.is_nan() {
        fail("this float isn't a number, so it can't be an int.".into());
    }
    if v.is_infinite() || v >= 9.2233720368547758e18 || v < -9.2233720368547758e18 {
        fail(format!("{} is too big to be an int.", ftext(v)));
    }
    v.trunc() as i64
}

pub fn isnum(s: &str) -> bool {
    let t = s.trim();
    if t.is_empty() || t.contains(['x', 'X', '_']) || t.eq_ignore_ascii_case("inf") || t.to_lowercase().contains("nan") || t.to_lowercase().contains("inf") {
        return false;
    }
    t.parse::<f64>().map(|v| v.is_finite()).unwrap_or(false)
}

pub fn sint(s: &str) -> i64 {
    if !isnum(s) {
        fail(format!("\"{s}\" isn't a number. Check first with .isNumber()."));
    }
    toint(s.trim().parse::<f64>().unwrap())
}

pub fn sfloat(s: &str) -> f64 {
    if !isnum(s) {
        fail(format!("\"{s}\" isn't a number. Check first with .isNumber()."));
    }
    s.trim().parse::<f64>().unwrap()
}

pub fn lines(s: &str) -> Arr<String> {
    let mut v: Vec<String> = s.split('\n').map(|l| l.strip_suffix('\r').unwrap_or(l).to_string()).collect();
    if s.ends_with('\n') {
        v.pop();
    }
    arr(v)
}

pub fn random(lo: i64, hi: i64) -> i64 {
    if lo > hi {
        fail(format!("random's low number ({lo}) is bigger than its high one ({hi})."));
    }
    let mut x = RNG.with(|c| c.get());
    if x == 0 {
        x = std::env::var("LUST_SEED")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .map(|s| s.wrapping_mul(0x9E3779B97F4A7C15) | 1)
            .unwrap_or_else(|| {
                std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64 | 1).unwrap_or(1)
            });
    }
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    RNG.with(|c| c.set(x));
    let span = (hi - lo) as u64 + 1;
    lo + (x % span) as i64
}

// ----- states: options are their full names, like "ActiveGame.Combat" -----

/// Is a the option b, or inside it?
pub fn sin(a: &str, b: &str) -> bool {
    a == b || (a.len() > b.len() && a.starts_with(b) && a.as_bytes()[b.len()] == b'.')
}

// ----- arrays, counted from 0 -----

pub type Arr<T> = Rc<RefCell<Vec<T>>>;

pub fn arr<T>(v: Vec<T>) -> Arr<T> {
    Rc::new(RefCell::new(v))
}

fn where_(n: usize, i: i64) {
    if n == 0 {
        fail(format!("the array is empty, so there's no item [{i}]."));
    }
    if i < 0 || i as usize >= n {
        let s = if n == 1 { "" } else { "s" };
        fail(format!("there's no item [{i}]. The array has {n} item{s}, numbered 0 to {}.", n - 1));
    }
}

pub fn aget<T: Clone>(a: &Arr<T>, i: i64) -> T {
    let v = a.borrow();
    where_(v.len(), i);
    v[i as usize].clone()
}

pub fn aset<T>(a: &Arr<T>, i: i64, x: T) {
    let n = a.borrow().len();
    where_(n, i);
    a.borrow_mut()[i as usize] = x;
}

pub fn ainject<T>(a: &Arr<T>, x: T) {
    a.borrow_mut().push(x);
}

pub fn aremove<T>(a: &Arr<T>, i: i64) {
    let n = a.borrow().len();
    where_(n, i);
    a.borrow_mut().remove(i as usize);
}

pub fn alen<T>(a: &Arr<T>) -> i64 {
    a.borrow().len() as i64
}

pub fn acopy<T: Clone>(a: &Arr<T>) -> Vec<T> {
    a.borrow().clone()
}

pub fn ashow<T>(a: &Arr<T>, f: impl Fn(&T) -> String) -> String {
    let parts: Vec<String> = a.borrow().iter().map(|x| f(x)).collect();
    format!("[{}]", parts.join(", "))
}

// ----- maps: keys in the order they were added -----

pub struct LMap<T> {
    keys: Vec<String>,
    vals: HashMap<String, T>,
}

impl<T> Default for LMap<T> {
    fn default() -> Self {
        LMap { keys: Vec::new(), vals: HashMap::new() }
    }
}

pub type Map<T> = Rc<RefCell<LMap<T>>>;

pub fn map<T>(pairs: Vec<(String, T)>) -> Map<T> {
    let mut m = LMap::default();
    for (k, v) in pairs {
        if !m.vals.contains_key(&k) {
            m.keys.push(k.clone());
        }
        m.vals.insert(k, v);
    }
    Rc::new(RefCell::new(m))
}

pub fn mget<T: Clone>(m: &Map<T>, k: &str) -> T {
    match m.borrow().vals.get(k) {
        Some(v) => v.clone(),
        None => fail(format!("there's no key \"{k}\" in this map. Check first with :has(\"{k}\").")),
    }
}

pub fn mset<T>(m: &Map<T>, k: String, v: T) {
    let mut b = m.borrow_mut();
    if !b.vals.contains_key(&k) {
        b.keys.push(k.clone());
    }
    b.vals.insert(k, v);
}

pub fn mhas<T>(m: &Map<T>, k: &str) -> bool {
    m.borrow().vals.contains_key(k)
}

pub fn mremove<T>(m: &Map<T>, k: &str) {
    let mut b = m.borrow_mut();
    if b.vals.remove(k).is_some() {
        b.keys.retain(|x| x != k);
    }
}

pub fn mlen<T>(m: &Map<T>) -> i64 {
    m.borrow().keys.len() as i64
}

pub fn mentries<T: Clone>(m: &Map<T>) -> Vec<(String, T)> {
    let b = m.borrow();
    b.keys.iter().map(|k| (k.clone(), b.vals[k].clone())).collect()
}

pub fn mshow<T>(m: &Map<T>, f: impl Fn(&T) -> String) -> String {
    let b = m.borrow();
    let parts: Vec<String> = b.keys.iter().map(|k| format!("\"{k}\" {}", f(&b.vals[k]))).collect();
    format!("[{}]", parts.join(", "))
}

// ----- cMaps: rows found by their key, the first column -----

/// What a cMap's key can be: a whole number or text.
pub trait Key: Clone + PartialEq {
    fn show(&self) -> String;
    /// The key Lust counts up to by itself, when it can.
    fn counted(n: i64) -> Option<Self>;
    fn number(&self) -> Option<i64>;
}

impl Key for i64 {
    fn show(&self) -> String {
        itext(*self)
    }
    fn counted(n: i64) -> Option<Self> {
        Some(n)
    }
    fn number(&self) -> Option<i64> {
        Some(*self)
    }
}

impl Key for String {
    fn show(&self) -> String {
        qtext(self)
    }
    fn counted(_: i64) -> Option<Self> {
        None
    }
    fn number(&self) -> Option<i64> {
        None
    }
}

pub trait Row {
    type K: Key;
    fn key(&self) -> Self::K;
    fn set_key(&mut self, k: Self::K);
}

pub struct CMapData<R: Row> {
    rows: Vec<Rc<RefCell<R>>>,
    next: i64,
    key: &'static str,
    maker: Option<Rc<dyn Fn() -> R::K>>,
}

impl<R: Row> Default for CMapData<R> {
    fn default() -> Self {
        CMapData { rows: Vec::new(), next: 1, key: "id", maker: None }
    }
}

pub type CMap<R> = Rc<RefCell<CMapData<R>>>;

fn taken<R: Row>(c: &CMap<R>, k: &R::K) -> bool {
    c.borrow().rows.iter().any(|r| r.borrow().key() == *k)
}

fn add_row<R: Row>(c: &CMap<R>, k: Option<R::K>, mut r: R) {
    let key = c.borrow().key;
    let k = match k {
        Some(k) => {
            if taken(c, &k) {
                fail(format!("there's already a row with {key} {}.", k.show()));
            }
            k
        }
        None => {
            let maker = c.borrow().maker.clone();
            match maker {
                Some(m) => {
                    let mut tries = 0;
                    loop {
                        // Called with the cMap free, so the key-maker can look at it.
                        let k = m();
                        if !taken(c, &k) {
                            break k;
                        }
                        tries += 1;
                        if tries >= 100 {
                            fail(format!("the key-maker keeps making {key}s that are already taken."));
                        }
                    }
                }
                None => match R::K::counted(c.borrow().next) {
                    Some(k) => k,
                    None => fail(format!("this cMap's key ({key}) is text, so each new row needs it first.")),
                },
            }
        }
    };
    let mut b = c.borrow_mut();
    if let Some(n) = k.number() {
        if n >= b.next {
            b.next = n + 1;
        }
    }
    r.set_key(k);
    b.rows.push(Rc::new(RefCell::new(r)));
}

pub fn carr<R: Row>(key: &'static str, maker: Option<Option<Rc<dyn Fn() -> R::K>>>, rows: Vec<(Option<R::K>, R)>) -> CMap<R> {
    let mut c = Rc::new(RefCell::new(CMapData { rows: Vec::new(), next: 1, key, maker: None }));
    if let Some(m) = maker {
        c = cmaker(c, m);
    }
    for (k, r) in rows {
        add_row(&c, k, r);
    }
    c
}

/// Gives a cMap its key-maker, and gives the cMap back.
pub fn cmaker<R: Row>(c: CMap<R>, maker: Option<Rc<dyn Fn() -> R::K>>) -> CMap<R> {
    c.borrow_mut().maker = Some(match maker {
        Some(m) => m,
        None => Rc::new(|| fail("the key-maker is nothing, so it can't make keys.".into())),
    });
    c
}

fn keys<R: Row>(c: &CMapData<R>) -> String {
    if c.rows.is_empty() {
        return "The cMap is empty.".into();
    }
    let list: Vec<String> = c.rows.iter().map(|r| r.borrow().key().show()).collect();
    format!("The {}s are {}.", c.key, list.join(", "))
}

pub fn cget<R: Row>(c: &CMap<R>, k: R::K) -> Rc<RefCell<R>> {
    let b = c.borrow();
    match b.rows.iter().find(|r| r.borrow().key() == k) {
        Some(r) => r.clone(),
        None => fail(format!("there's no row with {} {}. {}", b.key, k.show(), keys(&b))),
    }
}

pub fn cinject<R: Row>(c: &CMap<R>, row: (Option<R::K>, R)) {
    add_row(c, row.0, row.1);
}

pub fn chas<R: Row>(c: &CMap<R>, k: R::K) -> bool {
    taken(c, &k)
}

pub fn cremove<R: Row>(c: &CMap<R>, k: R::K) {
    let mut b = c.borrow_mut();
    match b.rows.iter().position(|r| r.borrow().key() == k) {
        Some(i) => {
            b.rows.remove(i);
        }
        None => {
            let msg = format!("there's no row with {} {} to remove. {}", b.key, k.show(), keys(&b));
            drop(b);
            fail(msg)
        }
    }
}

pub fn clen<R: Row>(c: &CMap<R>) -> i64 {
    c.borrow().rows.len() as i64
}

pub fn rows<R: Row>(c: &CMap<R>) -> Vec<Rc<RefCell<R>>> {
    c.borrow().rows.clone()
}

pub fn cshow<R: Row>(c: &CMap<R>, f: impl Fn(&R) -> String) -> String {
    let parts: Vec<String> = c.borrow().rows.iter().map(|r| f(&r.borrow())).collect();
    format!("[{}]", parts.join(", "))
}

// ----- the outside world -----

pub fn ask(q: String) -> String {
    use std::io::Write;
    print!("{q} ");
    let _ = std::io::stdout().flush();
    let mut line = String::new();
    match std::io::stdin().read_line(&mut line) {
        Ok(0) | Err(_) => String::new(),
        Ok(_) => line.trim_end_matches(['\n', '\r']).to_string(),
    }
}

pub fn read_text(path: String) -> String {
    match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => fail(format!("can't read {path}: {e}")),
    }
}

pub fn write_text(path: String, text: String) {
    if let Err(e) = std::fs::write(&path, text) {
        fail(format!("can't write {path}: {e}"));
    }
}

pub fn wait(s: f64) {
    if s < 0.0 || s.is_nan() {
        fail(format!("can't wait {} seconds.", ftext(s)));
    }
    if s.is_infinite() || s > 24.0 * 24.0 * 3600.0 {
        fail(format!("can't wait {} seconds; the longest wait is 24 days.", ftext(s)));
    }
    std::thread::sleep(std::time::Duration::from_secs_f64(s));
}
