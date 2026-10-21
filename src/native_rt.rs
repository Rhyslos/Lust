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
pub fn main(program: fn(), files: &'static [&'static str]) {
    std::panic::set_hook(Box::new(|_| {}));
    let t = std::thread::Builder::new()
        .stack_size(512 * 1024 * 1024)
        .spawn(move || {
            match std::panic::catch_unwind(|| {
                started();
                program()
            }) {
                Ok(()) => 0,
                Err(p) => {
                    use std::io::Write;
                    let _ = std::io::stdout().flush();
                    let msg = payload_text(&*p);
                    // The line has the file's number in its top bits.
                    let packed = LINE.with(|c| c.get());
                    let (f, line) = ((packed >> 20) as usize, packed & 0xFFFFF);
                    let file = files.get(f).copied().unwrap_or("");
                    if line > 0 {
                        eprintln!("{file}:{line}  {msg}");
                    } else {
                        eprintln!("{file}  {msg}");
                    }
                    1
                }
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
    if v {
        "true".into()
    } else {
        "false".into()
    }
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
    if h.is_some() {
        "hook".into()
    } else {
        "nothing".into()
    }
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
    if t.is_empty()
        || t.contains(['x', 'X', '_'])
        || t.eq_ignore_ascii_case("inf")
        || t.to_lowercase().contains("nan")
        || t.to_lowercase().contains("inf")
    {
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
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_nanos() as u64 | 1)
                    .unwrap_or(1)
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
        fail(format!(
            "there's no item [{i}]. The array has {n} item{s}, numbered 0 to {}.",
            n - 1
        ));
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
        LMap {
            keys: Vec::new(),
            vals: HashMap::new(),
        }
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
        CMapData {
            rows: Vec::new(),
            next: 1,
            key: "id",
            maker: None,
        }
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
    let mut c = Rc::new(RefCell::new(CMapData {
        rows: Vec::new(),
        next: 1,
        key,
        maker: None,
    }));
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

pub fn wait(s: f64) {
    if s < 0.0 || s.is_nan() {
        fail(format!("can't wait {} seconds.", ftext(s)));
    }
    if s.is_infinite() || s > 24.0 * 24.0 * 3600.0 {
        fail(format!("can't wait {} seconds; the longest wait is 24 days.", ftext(s)));
    }
    std::thread::sleep(std::time::Duration::from_secs_f64(s));
}

// ----- the standard library -----
// The same functions, with the same messages, are in runtime.lua for LustScript.

// Text, counted in letters.
pub fn text_upper(s: String) -> String {
    // a–z and the accented letters of Latin-1 (à to þ), the same as in Lua.
    s.chars()
        .map(|c| match c {
            'a'..='z' => c.to_ascii_uppercase(),
            '\u{e0}'..='\u{fe}' if c != '\u{f7}' => char::from_u32(c as u32 - 32).unwrap_or(c),
            _ => c,
        })
        .collect()
}
pub fn text_lower(s: String) -> String {
    s.chars()
        .map(|c| match c {
            'A'..='Z' => c.to_ascii_lowercase(),
            '\u{c0}'..='\u{de}' if c != '\u{d7}' => char::from_u32(c as u32 + 32).unwrap_or(c),
            _ => c,
        })
        .collect()
}
pub fn text_trim(s: String) -> String {
    s.trim_matches([' ', '\t', '\r', '\n']).to_string()
}
pub fn text_contains(s: String, x: String) -> bool {
    s.contains(&x)
}
pub fn text_startsWith(s: String, x: String) -> bool {
    s.starts_with(&x)
}
pub fn text_endsWith(s: String, x: String) -> bool {
    s.ends_with(&x)
}
pub fn text_find(s: String, x: String) -> i64 {
    match s.find(&x) {
        Some(i) => s[..i].chars().count() as i64,
        None => -1,
    }
}
pub fn text_replace(s: String, old: String, new: String) -> String {
    if old.is_empty() {
        fail("replace needs some text to look for.".into());
    }
    s.replace(&old, &new)
}
pub fn text_split(s: String, sep: String) -> Arr<String> {
    if sep.is_empty() {
        return arr(s.chars().map(|c| c.to_string()).collect());
    }
    arr(s.split(sep.as_str()).map(|p| p.to_string()).collect())
}
pub fn text_slice(s: String, from: i64, to: i64) -> String {
    let list: Vec<char> = s.chars().collect();
    let n = list.len() as i64;
    if from < 0 || to > n || from > to {
        fail(format!(
            "can't slice from {from} to {to}: the text has {n} letter{}.",
            if n == 1 { "" } else { "s" }
        ));
    }
    list[from as usize..to as usize].iter().collect()
}

// Arrays.
pub fn array_join(a: Arr<String>, sep: String) -> String {
    a.borrow().join(&sep)
}
pub fn array_has<T: PartialEq>(a: Arr<T>, x: T) -> bool {
    a.borrow().iter().any(|y| *y == x)
}
pub fn array_find<T: PartialEq>(a: Arr<T>, x: T) -> i64 {
    a.borrow().iter().position(|y| *y == x).map(|i| i as i64).unwrap_or(-1)
}
pub fn array_sort<T: PartialOrd>(a: Arr<T>) {
    a.borrow_mut().sort_by(|x, y| x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal));
}
pub fn array_reverse<T>(a: Arr<T>) {
    a.borrow_mut().reverse();
}

// Maths.
pub fn maths_abs_int(x: i64) -> i64 {
    x.abs()
}
pub fn maths_abs_float(x: f64) -> f64 {
    x.abs()
}
pub fn maths_min_int(a: i64, b: i64) -> i64 {
    a.min(b)
}
pub fn maths_max_int(a: i64, b: i64) -> i64 {
    a.max(b)
}
pub fn maths_min_float(a: f64, b: f64) -> f64 {
    if b < a {
        b
    } else {
        a
    }
}
pub fn maths_max_float(a: f64, b: f64) -> f64 {
    if b > a {
        b
    } else {
        a
    }
}
pub fn maths_clamp_int(x: i64, lo: i64, hi: i64) -> i64 {
    if lo > hi {
        fail(format!("clamp's low ({lo}) is bigger than its high ({hi})."));
    }
    x.max(lo).min(hi)
}
pub fn maths_clamp_float(x: f64, lo: f64, hi: f64) -> f64 {
    if lo > hi {
        fail(format!("clamp's low ({}) is bigger than its high ({}).", ftext(lo), ftext(hi)));
    }
    if x < lo {
        lo
    } else if x > hi {
        hi
    } else {
        x
    }
}
pub fn maths_round(x: f64) -> i64 {
    if x >= 0.0 {
        toint((x + 0.5).floor())
    } else {
        toint(-((-x + 0.5).floor()))
    }
}
pub fn maths_floor(x: f64) -> i64 {
    toint(x.floor())
}
pub fn maths_ceil(x: f64) -> i64 {
    toint(x.ceil())
}
pub fn maths_sqrt(x: f64) -> f64 {
    if x < 0.0 {
        fail(format!("there's no square root of a number below zero ({}).", ftext(x)));
    }
    x.sqrt()
}
pub fn maths_power(a: f64, b: f64) -> f64 {
    a.powf(b)
}
pub fn maths_sin(x: f64) -> f64 {
    x.sin()
}
pub fn maths_cos(x: f64) -> f64 {
    x.cos()
}
pub fn maths_tan(x: f64) -> f64 {
    x.tan()
}
pub fn maths_pi() -> f64 {
    std::f64::consts::PI
}

// Files. The fs_ ones are also what `lust run` gives Lua, so both say the same things.
pub fn fs_remove(path: &str) -> Result<(), String> {
    let p = std::path::Path::new(path);
    let r = if p.is_dir() {
        std::fs::remove_dir(p)
    } else {
        std::fs::remove_file(p)
    };
    r.map_err(|e| match e.kind() {
        std::io::ErrorKind::NotFound => format!("can't remove {path}: there's no such file or folder."),
        _ if p.is_dir() => format!("can't remove {path}: the folder isn't empty."),
        _ => format!("can't remove {path}."),
    })
}
pub fn fs_list(folder: &str) -> Result<Vec<String>, String> {
    let rd = std::fs::read_dir(folder).map_err(|_| format!("there's no folder {folder}."))?;
    let mut names: Vec<String> = rd
        .filter_map(|e| e.ok().map(|e| e.file_name().to_string_lossy().to_string()))
        .collect();
    names.sort();
    Ok(names)
}
pub fn fs_mkdir(folder: &str) -> Result<(), String> {
    std::fs::create_dir_all(folder).map_err(|_| format!("can't make the folder {folder}."))
}
pub fn files_read(path: String) -> String {
    match std::fs::read(&path) {
        Ok(b) => String::from_utf8_lossy(&b).to_string(),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => fail(format!("can't read {path}: there's no such file.")),
        Err(_) => fail(format!("can't read {path}.")),
    }
}
pub fn files_write(path: String, text: String) {
    if std::fs::write(&path, text).is_err() {
        fail(format!("can't write {path}."));
    }
}
pub fn files_append(path: String, text: String) {
    use std::io::Write;
    let ok = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(&path)
        .and_then(|mut f| f.write_all(text.as_bytes()));
    if ok.is_err() {
        fail(format!("can't append {path}."));
    }
}
pub fn files_exists(path: String) -> bool {
    std::path::Path::new(&path).exists()
}
pub fn files_remove(path: String) {
    if let Err(e) = fs_remove(&path) {
        fail(e);
    }
}
pub fn files_list(folder: String) -> Arr<String> {
    match fs_list(&folder) {
        Ok(n) => arr(n),
        Err(e) => fail(e),
    }
}
pub fn files_makeFolder(folder: String) {
    if let Err(e) = fs_mkdir(&folder) {
        fail(e);
    }
}

// Time.
thread_local! {
    static STARTED: std::time::Instant = std::time::Instant::now();
}
pub fn started() {
    STARTED.with(|_| {});
}
pub fn time_now() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}
pub fn time_clock() -> f64 {
    STARTED.with(|s| s.elapsed().as_secs_f64())
}

#[repr(C)]
struct Tm {
    // sec, min, hour, day of the month, month from 0, years since 1900, weekday, day of the year, summer time
    f: [i32; 9],
    // Room for what some systems add after those.
    extra: [u64; 4],
}

#[cfg(unix)]
extern "C" {
    fn localtime_r(t: *const i64, out: *mut Tm) -> *mut Tm;
}
#[cfg(windows)]
extern "C" {
    fn _localtime64_s(out: *mut Tm, t: *const i64) -> i32;
}

/// The local date and time now, as the computer's clock and time zone say.
fn local_now() -> [i32; 9] {
    let t = time_now().floor() as i64;
    let mut tm = Tm { f: [0; 9], extra: [0; 4] };
    #[cfg(unix)]
    let ok = unsafe { !localtime_r(&t, &mut tm).is_null() };
    #[cfg(windows)]
    let ok = unsafe { _localtime64_s(&mut tm, &t) == 0 };
    #[cfg(not(any(unix, windows)))]
    let ok = false;
    if ok {
        return tm.f;
    }
    // No time zone to be had: the time in UTC.
    let days = t.div_euclid(86400);
    let secs = t.rem_euclid(86400) as i32;
    let (y, m, d) = civil(days);
    [
        secs % 60,
        secs / 60 % 60,
        secs / 3600,
        d as i32,
        m as i32 - 1,
        y as i32 - 1900,
        0,
        0,
        0,
    ]
}

/// Days since 1970-01-01 to year, month, day.
fn civil(z: i64) -> (i64, i64, i64) {
    let z = z + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}

pub fn time_date() -> String {
    let t = local_now();
    format!("{:04}-{:02}-{:02}", t[5] + 1900, t[4] + 1, t[3])
}
pub fn time_time() -> String {
    let t = local_now();
    format!("{:02}:{:02}:{:02}", t[2], t[1], t[0])
}

// ----- JSON -----

pub enum Json {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<Json>),
    Obj(Vec<(String, Json)>),
}

struct JP<'a> {
    b: &'a [u8],
    i: usize,
}

impl<'a> JP<'a> {
    fn fail(&self, at: usize, what: &str) -> ! {
        let line = self.b[..at.min(self.b.len())].iter().filter(|&&c| c == b'\n').count() + 1;
        fail(format!("this isn't valid JSON (line {line}): {what}."))
    }
    fn unexpected(&self, at: usize) -> ! {
        match self.b.get(at) {
            None => self.fail(at, "the text ends too early"),
            Some(&c) if c >= 128 => self.fail(at, "a letter is out of place"),
            Some(&c) => self.fail(at, &format!("\"{}\" is out of place", c as char)),
        }
    }
    fn skip(&mut self) {
        while matches!(self.b.get(self.i), Some(b' ' | b'\t' | b'\r' | b'\n')) {
            self.i += 1;
        }
    }
    fn at(&self, j: usize) -> u8 {
        self.b.get(j).copied().unwrap_or(0)
    }
    fn digits(&self, j: usize) -> usize {
        let mut k = j;
        while self.at(k).is_ascii_digit() {
            k += 1;
        }
        k - j
    }
    fn string(&mut self) -> String {
        let mut out: Vec<u8> = Vec::new();
        let mut j = self.i + 1;
        loop {
            let Some(&c) = self.b.get(j) else {
                self.fail(j, "the text ends too early")
            };
            if c == b'"' {
                self.i = j + 1;
                return String::from_utf8_lossy(&out).to_string();
            }
            if c == b'\\' {
                let e = self.at(j + 1);
                let simple = match e {
                    b'"' => Some(b'"'),
                    b'\\' => Some(b'\\'),
                    b'/' => Some(b'/'),
                    b'b' => Some(8),
                    b'f' => Some(12),
                    b'n' => Some(b'\n'),
                    b'r' => Some(b'\r'),
                    b't' => Some(b'\t'),
                    _ => None,
                };
                if let Some(x) = simple {
                    out.push(x);
                    j += 2;
                } else if e == b'u' {
                    let hex = |k: usize| -> Option<u32> {
                        let h = self.b.get(k..k + 4)?;
                        if !h.iter().all(|c| c.is_ascii_hexdigit()) {
                            return None;
                        }
                        u32::from_str_radix(std::str::from_utf8(h).ok()?, 16).ok()
                    };
                    let Some(mut n) = hex(j + 2) else {
                        self.fail(j, "a \\ in text is followed by something JSON doesn't allow")
                    };
                    j += 6;
                    if (0xD800..=0xDBFF).contains(&n) && self.at(j) == b'\\' && self.at(j + 1) == b'u' {
                        if let Some(lo) = hex(j + 2).filter(|lo| (0xDC00..=0xDFFF).contains(lo)) {
                            n = 0x10000 + (n - 0xD800) * 1024 + (lo - 0xDC00);
                            j += 6;
                        }
                    }
                    let ch = char::from_u32(n).unwrap_or('\u{fffd}');
                    let mut buf = [0u8; 4];
                    out.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
                } else {
                    self.fail(j, "a \\ in text is followed by something JSON doesn't allow");
                }
            } else if c < 32 {
                self.fail(j, "text can't have a line break or tab inside it; write \\n or \\t");
            } else {
                out.push(c);
                j += 1;
            }
        }
    }
    fn value(&mut self) -> Json {
        self.skip();
        let i = self.i;
        match self.at(i) {
            b'{' => {
                let mut o: Vec<(String, Json)> = Vec::new();
                self.i += 1;
                self.skip();
                if self.at(self.i) == b'}' {
                    self.i += 1;
                    return Json::Obj(o);
                }
                loop {
                    self.skip();
                    if self.at(self.i) != b'"' {
                        self.unexpected(self.i);
                    }
                    let k = self.string();
                    self.skip();
                    if self.at(self.i) != b':' {
                        self.unexpected(self.i);
                    }
                    self.i += 1;
                    let v = self.value();
                    match o.iter_mut().find(|(x, _)| *x == k) {
                        Some(slot) => slot.1 = v,
                        None => o.push((k, v)),
                    }
                    self.skip();
                    match self.at(self.i) {
                        b'}' => {
                            self.i += 1;
                            return Json::Obj(o);
                        }
                        b',' => self.i += 1,
                        _ => self.unexpected(self.i),
                    }
                }
            }
            b'[' => {
                let mut a = Vec::new();
                self.i += 1;
                self.skip();
                if self.at(self.i) == b']' {
                    self.i += 1;
                    return Json::Arr(a);
                }
                loop {
                    a.push(self.value());
                    self.skip();
                    match self.at(self.i) {
                        b']' => {
                            self.i += 1;
                            return Json::Arr(a);
                        }
                        b',' => self.i += 1,
                        _ => self.unexpected(self.i),
                    }
                }
            }
            b'"' => Json::Str(self.string()),
            _ if self.b[i..].starts_with(b"true") => {
                self.i += 4;
                Json::Bool(true)
            }
            _ if self.b[i..].starts_with(b"false") => {
                self.i += 5;
                Json::Bool(false)
            }
            _ if self.b[i..].starts_with(b"null") => {
                self.i += 4;
                Json::Null
            }
            _ => {
                let mut j = i;
                if self.at(j) == b'-' {
                    j += 1;
                }
                let d = self.digits(j);
                if d == 0 {
                    self.unexpected(j);
                }
                if d > 1 && self.at(j) == b'0' {
                    self.unexpected(j + 1);
                }
                j += d;
                if self.at(j) == b'.' {
                    let f = self.digits(j + 1);
                    if f == 0 {
                        self.unexpected(j + 1);
                    }
                    j += 1 + f;
                }
                if matches!(self.at(j), b'e' | b'E') {
                    j += 1;
                    if matches!(self.at(j), b'+' | b'-') {
                        j += 1;
                    }
                    let x = self.digits(j);
                    if x == 0 {
                        self.unexpected(j);
                    }
                    j += x;
                }
                let n: f64 = std::str::from_utf8(&self.b[i..j]).unwrap_or("0").parse().unwrap_or(0.0);
                self.i = j;
                Json::Num(n)
            }
        }
    }
}

fn kind(v: &Json) -> &'static str {
    match v {
        Json::Null => "nothing (null)",
        Json::Bool(_) => "a truth",
        Json::Num(_) => "a number",
        Json::Str(_) => "text",
        Json::Arr(_) => "an array",
        Json::Obj(_) => "an object",
    }
}
fn place(at: &str) -> String {
    if at.is_empty() {
        "the JSON".into()
    } else {
        format!("the JSON at {at}")
    }
}
fn misfit(at: &str, want: &str, v: &Json) -> ! {
    fail(format!("{} should be {want}, but it's {}.", place(at), kind(v)))
}
pub fn child(at: &str, k: &str) -> String {
    if at.is_empty() {
        k.to_string()
    } else {
        format!("{at}.{k}")
    }
}
pub fn jget<'a>(v: &'a Json, k: &str) -> Option<&'a Json> {
    match v {
        Json::Obj(o) => o.iter().find(|(x, _)| x == k).map(|(_, v)| v),
        _ => None,
    }
}
pub fn jneed<'a>(v: &'a Json, k: &str, at: &str) -> &'a Json {
    match jget(v, k) {
        Some(x) => x,
        None => fail(format!("the JSON at {at} is missing {k}.")),
    }
}
pub fn jr_int(v: &Json, at: &str) -> i64 {
    match v {
        Json::Num(n) => {
            if n.fract() != 0.0 || n.abs() > 9007199254740992.0 {
                fail(format!("{} should be an int, but it's a number with a fraction.", place(at)));
            }
            *n as i64
        }
        _ => misfit(at, "an int", v),
    }
}
pub fn jr_float(v: &Json, at: &str) -> f64 {
    match v {
        Json::Num(n) => *n,
        _ => misfit(at, "a number", v),
    }
}
pub fn jr_string(v: &Json, at: &str) -> String {
    match v {
        Json::Str(s) => s.clone(),
        _ => misfit(at, "text", v),
    }
}
pub fn jr_truth(v: &Json, at: &str) -> bool {
    match v {
        Json::Bool(b) => *b,
        _ => misfit(at, "a truth (true or false)", v),
    }
}
pub fn jr_state(v: &Json, at: &str, opts: &[&'static str]) -> &'static str {
    if let Json::Str(s) = v {
        if let Some(o) = opts.iter().find(|o| **o == s.as_str()) {
            return o;
        }
    }
    let shown = match v {
        Json::Str(s) => format!("\"{s}\""),
        _ => kind(v).to_string(),
    };
    fail(format!("{} should be one of {}, but it's {shown}.", place(at), opts.join(", ")))
}
pub fn jr_array<T>(v: &Json, at: &str, f: &dyn Fn(&Json, &str) -> T) -> Arr<T> {
    match v {
        Json::Arr(a) => arr(a.iter().enumerate().map(|(i, x)| f(x, &format!("{at}[{i}]"))).collect()),
        _ => misfit(at, "an array", v),
    }
}
pub fn jr_map<T>(v: &Json, at: &str, f: &dyn Fn(&Json, &str) -> T) -> Map<T> {
    match v {
        Json::Obj(o) => map(o.iter().map(|(k, x)| (k.clone(), f(x, &child(at, k)))).collect()),
        _ => misfit(at, "an object", v),
    }
}
pub fn jr_cmap<R: Row>(v: &Json, at: &str, key: &'static str, f: &dyn Fn(&Json, &str) -> (Option<R::K>, R)) -> CMap<R> {
    match v {
        Json::Arr(a) => {
            let mut rows = Vec::new();
            for (i, r) in a.iter().enumerate() {
                let rat = format!("{at}[{i}]");
                if !matches!(r, Json::Obj(_)) {
                    misfit(&rat, "an object", r);
                }
                rows.push(f(r, &rat));
            }
            carr(key, None, rows)
        }
        _ => misfit(at, "an array", v),
    }
}
pub fn jr_object(v: &Json, at: &str) -> bool {
    match v {
        Json::Null => false,
        Json::Obj(_) => true,
        _ => misfit(at, "an object", v),
    }
}
pub fn json_read<T>(text: String, f: &dyn Fn(&Json, &str) -> T) -> T {
    let mut p = JP { b: text.as_bytes(), i: 0 };
    let v = p.value();
    p.skip();
    if p.i < p.b.len() {
        p.unexpected(p.i);
    }
    f(&v, "")
}

fn jquote(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            c if (c as u32) < 32 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
pub fn deeper(d: Option<usize>) -> Option<usize> {
    d.map(|d| d + 1)
}
fn block(open: &str, close: &str, parts: Vec<String>, d: Option<usize>) -> String {
    if parts.is_empty() {
        return format!("{open}{close}");
    }
    match d {
        None => format!("{open}{}{close}", parts.join(",")),
        Some(d) => {
            let pad = "  ".repeat(d + 1);
            format!("{open}\n{pad}{}\n{}{close}", parts.join(&format!(",\n{pad}")), "  ".repeat(d))
        }
    }
}
fn pair(k: &str, v: String, d: Option<usize>) -> String {
    if d.is_some() {
        format!("{}: {v}", jquote(k))
    } else {
        format!("{}:{v}", jquote(k))
    }
}
pub fn jw_int(v: &i64, _: Option<usize>) -> String {
    itext(*v)
}
pub fn jw_float(v: &f64, _: Option<usize>) -> String {
    if !v.is_finite() {
        fail(format!("JSON can't hold {}.", ftext(*v)));
    }
    ftext(*v)
}
pub fn jw_string(v: &String, _: Option<usize>) -> String {
    jquote(v)
}
pub fn jw_truth(v: &bool, _: Option<usize>) -> String {
    btext(*v)
}
pub fn jw_state(v: &&'static str, _: Option<usize>) -> String {
    jquote(v)
}
pub fn jw_array<T>(a: &Arr<T>, d: Option<usize>, f: &dyn Fn(&T, Option<usize>) -> String) -> String {
    let parts = a.borrow().iter().map(|x| f(x, deeper(d))).collect();
    block("[", "]", parts, d)
}
pub fn jw_map<T>(m: &Map<T>, d: Option<usize>, f: &dyn Fn(&T, Option<usize>) -> String) -> String {
    let b = m.borrow();
    let parts = b.keys.iter().map(|k| pair(k, f(&b.vals[k], deeper(d)), deeper(d))).collect();
    block("{", "}", parts, d)
}
/// One thing as a JSON object: its parts already written one level deeper.
pub fn jw_fields(parts: Vec<(&str, String)>, d: Option<usize>) -> String {
    let parts = parts.into_iter().map(|(k, v)| pair(k, v, deeper(d))).collect();
    block("{", "}", parts, d)
}
pub fn jw_cmap<R: Row>(c: &CMap<R>, d: Option<usize>, f: &dyn Fn(&R, Option<usize>) -> String) -> String {
    let parts = c.borrow().rows.iter().map(|r| f(&r.borrow(), deeper(d))).collect();
    block("[", "]", parts, d)
}
thread_local! {
    static JDEPTH: Cell<u32> = Cell::new(0);
}
/// Guards against things that hold themselves, which would go on forever.
pub fn jw_enter() {
    let n = JDEPTH.with(|c| c.get()) + 1;
    JDEPTH.with(|c| c.set(n));
    if n > 500 {
        fail("this holds itself (somewhere inside it, something points back to it), so it can't be JSON.".into());
    }
}
pub fn jw_leave() {
    JDEPTH.with(|c| c.set(c.get() - 1));
}
pub fn json_write<T>(v: &T, f: &dyn Fn(&T, Option<usize>) -> String, pretty: bool) -> String {
    JDEPTH.with(|c| c.set(0));
    f(v, if pretty { Some(0) } else { None })
}
