// Lust's runtime for compiled programs. It goes into every program Lust builds,
// as `mod rt`, and prints and fails exactly the way the Lua runtime does.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

thread_local! {
    /// The file and line (packed, like a Lust position's) in the top half, the column below.
    static LINE: Cell<u64> = const { Cell::new(0) };
    static DEPTH: Cell<u32> = const { Cell::new(0) };
    static RNG: Cell<u64> = const { Cell::new(0) };
}

/// The Lust line and column now running, for error messages.
pub fn at(line: u32, col: u32) {
    LINE.with(|c| c.set(((line as u64) << 32) | col as u64));
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
/// macOS only opens windows from the main thread, so programs with views run there.
pub fn main(program: fn(), files: &'static [&'static str]) {
    std::panic::set_hook(Box::new(|_| {}));
    if cfg!(target_os = "macos") && UI_PROGRAM {
        std::process::exit(run_program(program, files));
    }
    let t = std::thread::Builder::new()
        .stack_size(512 * 1024 * 1024)
        .spawn(move || run_program(program, files))
        .expect("starting the program");
    let code = t.join().unwrap_or(1);
    std::process::exit(code);
}

fn run_program(program: fn(), files: &'static [&'static str]) -> i32 {
            match std::panic::catch_unwind(|| {
                started();
                program()
            }) {
                Ok(()) => 0,
                Err(p) => {
                    use std::io::Write;
                    let _ = std::io::stdout().flush();
                    eprintln!("{}", failure_text(&*p, files));
                    1
                }
            }
}

/// A failure as Lust shows it: "player.lst:12  there's no row with id 7."
fn failure_text(p: &(dyn std::any::Any + Send), files: &[&str]) -> String {
    failure(p, files).to_string()
}

/// A failure in Lust code, for the program using it as a library: the file and line it stopped
/// at, and why. Shown, it reads like lust run's: `gun.lst:14  ammo can't be below 0`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LustError {
    /// The file, as the first file sees it: `gun.lst`, `parts/aim.lst`.
    pub file: String,
    /// From 1. None when the failure isn't on a line of its own.
    pub line: Option<u32>,
    /// From 1: the column of the statement that failed (for `int each = 12 / ways;`, where
    /// `each` is). None with `line`.
    pub column: Option<u32>,
    pub message: String,
}

impl std::fmt::Display for LustError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self.line {
            Some(l) => write!(f, "{}:{l}  {}", self.file, self.message),
            None => write!(f, "{}  {}", self.file, self.message),
        }
    }
}

impl std::error::Error for LustError {}

/// A failure at a line packed with its file's number in the top bits.
pub fn error_at(files: &[&str], packed: u32, col: u32, message: String) -> LustError {
    let (f, line) = ((packed >> 20) as usize, packed & 0xFFFFF);
    LustError {
        file: files.get(f).copied().unwrap_or("").to_string(),
        line: (line > 0).then_some(line),
        column: (line > 0 && col > 0).then_some(col),
        message,
    }
}

fn failure(p: &(dyn std::any::Any + Send), files: &[&str]) -> LustError {
    let at = LINE.with(|c| c.get());
    error_at(files, (at >> 32) as u32, at as u32, payload_text(p))
}

/// For Lust built as a library: runs Lust code for the program using it, and gives back a
/// failure, with where it happened, instead of stopping that program.
pub fn catch<R>(files: &[&str], f: impl FnOnce() -> R) -> Result<R, LustError> {
    quiet();
    let depth = DEPTH.with(|c| c.get());
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
        Ok(r) => Ok(r),
        Err(p) => {
            DEPTH.with(|c| c.set(depth));
            Err(failure(&*p, files))
        }
    }
}

/// Lust's own failures are caught and passed back as text, so Rust doesn't print them too. The
/// program's own panics still print as they did.
fn quiet() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let before = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            if info.payload().downcast_ref::<Fail>().is_none() {
                before(info);
            }
        }));
    });
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

pub fn obj<T: HasClass>(o: &Option<Rc<RefCell<T>>>) -> Rc<RefCell<T>> {
    match o {
        Some(r) => {
            if let Ok(b) = r.try_borrow() {
                if let Some(m) = b.ended() {
                    fail(m.to_string());
                }
            }
            r.clone()
        }
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
    /// Why it's gone, once it has ended with the state it was made in.
    fn ended(&self) -> Option<&str>;
}

/// Things that can end: their fields are let go, and using them says why.
pub trait Endable {
    fn end(&mut self, msg: String);
}

// ----- things made in a state end with it -----

struct Owned {
    owner: usize,
    state: &'static str,
    thing: usize,
    owner_alive: Box<dyn Fn() -> bool>,
    /// Ends the thing. False when it's already gone.
    end: Box<dyn Fn(&str) -> bool>,
}

thread_local! {
    static OWNED: RefCell<Vec<Owned>> = const { RefCell::new(Vec::new()) };
}

fn addr<T>(r: &Rc<T>) -> usize {
    Rc::as_ptr(r) as *const u8 as usize
}

pub fn ended_msg(state: &str) -> String {
    format!(
        "this was made during {state}, and ended with it. To hold onto something made in a state, keep it, like keep sword in player.bag;"
    )
}

/// Something made in a method of an object that's in a state: it belongs to that state.
pub fn own<O: 'static, T: Endable + 'static>(
    owner: &Rc<RefCell<O>>,
    state: &'static str,
    thing: Option<Rc<RefCell<T>>>,
) -> Option<Rc<RefCell<T>>> {
    if let Some(t) = &thing {
        let wo = Rc::downgrade(owner);
        let wt = Rc::downgrade(t);
        let entry = Owned {
            owner: addr(owner),
            state,
            thing: addr(t),
            owner_alive: Box::new(move || wo.strong_count() > 0),
            end: Box::new(move |msg| match wt.upgrade() {
                Some(t) => match t.try_borrow_mut() {
                    Ok(mut b) => {
                        b.end(msg.to_string());
                        true
                    }
                    Err(_) => false,
                },
                None => false,
            }),
        };
        OWNED.with(|o| {
            let mut o = o.borrow_mut();
            // Now and then, forget things that are gone anyway.
            if o.len() >= 256 && o.len().is_power_of_two() {
                o.retain(|x| (x.owner_alive)());
            }
            o.push(entry);
        });
    }
    thing
}

fn take_owned(pick: impl Fn(&Owned) -> bool) -> Vec<Owned> {
    OWNED.with(|o| {
        let mut o = o.borrow_mut();
        let all = std::mem::take(&mut *o);
        let (picked, rest): (Vec<Owned>, Vec<Owned>) = all.into_iter().partition(|x| pick(x));
        *o = rest;
        picked
    })
}

fn end_owned_by(owner: usize, msg: &str) {
    for x in take_owned(|x| x.owner == owner && (x.owner_alive)()) {
        if (x.end)(msg) {
            end_owned_by(x.thing, msg);
        }
    }
}

/// An object's state changed: what it made in an option that's over now ends.
pub fn set_state<O>(owner: &Rc<RefCell<O>>, new: &'static str) {
    let a = addr(owner);
    let over = take_owned(|x| x.owner == a && !sin(new, x.state) && (x.owner_alive)());
    for x in over {
        let msg = ended_msg(x.state);
        if (x.end)(&msg) {
            end_owned_by(x.thing, &msg);
        }
    }
}

/// A node closed: everything it made ends.
pub fn end_owned<O>(owner: &Rc<RefCell<O>>, msg: &str) {
    end_owned_by(addr(owner), msg);
}

/// keep: it no longer belongs to the state it was made in.
pub fn keep<T>(t: &Option<Rc<RefCell<T>>>) {
    if let Some(t) = t {
        let a = addr(t);
        take_owned(|x| x.thing == a);
    }
}

pub fn like<T: HasClass>(o: &Option<Rc<RefCell<T>>>, ids: &[u32]) -> bool {
    match o {
        None => false,
        Some(r) => {
            let b = r.borrow();
            b.ended().is_none() && ids.contains(&b.class_id())
        }
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
    if !(-4..15).contains(&exp) {
        let sign = if exp < 0 { '-' } else { '+' };
        format!("{}e{}{:02}", trim_zeros(mant), sign, exp.abs())
    } else {
        let decimals = (14 - exp).max(0) as usize;
        trim_zeros(&format!("{:.*}", decimals, v))
    }
}

// ----- vectors and rotations -----
// A vec3 and a rotation (a quaternion), each number kept as an f32, as engines keep them. Every
// step is worked out in f64 and rounded to f32, the same way the Lua runtime does it, so both
// give exactly the same numbers.

#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Rotation {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Default for Rotation {
    fn default() -> Rotation {
        Rotation { x: 0.0, y: 0.0, z: 0.0, w: 1.0 }
    }
}

pub fn vec3(x: f64, y: f64, z: f64) -> Vec3 {
    Vec3 { x: x as f32, y: y as f32, z: z as f32 }
}
fn v64(v: Vec3) -> (f64, f64, f64) {
    (v.x as f64, v.y as f64, v.z as f64)
}
fn q4(q: Rotation) -> (f64, f64, f64, f64) {
    (q.x as f64, q.y as f64, q.z as f64, q.w as f64)
}
pub fn vwith(v: Vec3, f: usize, n: f64) -> Vec3 {
    let mut w = v;
    match f {
        0 => w.x = n as f32,
        1 => w.y = n as f32,
        _ => w.z = n as f32,
    }
    w
}
pub fn vadd(a: Vec3, b: Vec3) -> Vec3 {
    let ((ax, ay, az), (bx, by, bz)) = (v64(a), v64(b));
    vec3(ax + bx, ay + by, az + bz)
}
pub fn vsub(a: Vec3, b: Vec3) -> Vec3 {
    let ((ax, ay, az), (bx, by, bz)) = (v64(a), v64(b));
    vec3(ax - bx, ay - by, az - bz)
}
pub fn vmul(a: Vec3, n: f64) -> Vec3 {
    let (x, y, z) = v64(a);
    vec3(x * n, y * n, z * n)
}
pub fn vdiv(a: Vec3, n: f64) -> Vec3 {
    let (x, y, z) = v64(a);
    vec3(x / n, y / n, z / n)
}
pub fn vneg(a: Vec3) -> Vec3 {
    Vec3 { x: -a.x, y: -a.y, z: -a.z }
}
pub fn vlength(a: Vec3) -> f64 {
    let (x, y, z) = v64(a);
    (x * x + y * y + z * z).sqrt()
}
pub fn vnormalized(a: Vec3) -> Vec3 {
    let (x, y, z) = v64(a);
    let l = (x * x + y * y + z * z).sqrt();
    if l == 0.0 {
        return a;
    }
    vec3(x / l, y / l, z / l)
}
pub fn vdot(a: Vec3, b: Vec3) -> f64 {
    let ((ax, ay, az), (bx, by, bz)) = (v64(a), v64(b));
    ax * bx + ay * by + az * bz
}
pub fn vcross(a: Vec3, b: Vec3) -> Vec3 {
    let ((ax, ay, az), (bx, by, bz)) = (v64(a), v64(b));
    vec3(ay * bz - az * by, az * bx - ax * bz, ax * by - ay * bx)
}
pub fn vdistance(a: Vec3, b: Vec3) -> f64 {
    let ((ax, ay, az), (bx, by, bz)) = (v64(a), v64(b));
    let (dx, dy, dz) = (ax - bx, ay - by, az - bz);
    (dx * dx + dy * dy + dz * dz).sqrt()
}
pub fn vlerp(a: Vec3, b: Vec3, t: f64) -> Vec3 {
    let ((ax, ay, az), (bx, by, bz)) = (v64(a), v64(b));
    vec3(ax + (bx - ax) * t, ay + (by - ay) * t, az + (bz - az) * t)
}
#[allow(clippy::too_many_arguments)]
fn q64(ax: f64, ay: f64, az: f64, aw: f64, bx: f64, by: f64, bz: f64, bw: f64) -> (f64, f64, f64, f64) {
    (
        aw * bx + ax * bw + ay * bz - az * by,
        aw * by - ax * bz + ay * bw + az * bx,
        aw * bz + ax * by - ay * bx + az * bw,
        aw * bw - ax * bx - ay * by - az * bz,
    )
}
const RAD: f64 = std::f64::consts::PI / 180.0;
fn qround((x, y, z, w): (f64, f64, f64, f64)) -> Rotation {
    Rotation { x: x as f32, y: y as f32, z: z as f32, w: w as f32 }
}
/// Degrees around x, y and z, turned in the order z, x, y (as Solis and glam's EulerRot::ZXY do).
pub fn rot(dx: f64, dy: f64, dz: f64) -> Rotation {
    let (hx, hy, hz) = (dx * RAD / 2.0, dy * RAD / 2.0, dz * RAD / 2.0);
    let (x, y, z, w) = q64(0.0, 0.0, hz.sin(), hz.cos(), hx.sin(), 0.0, 0.0, hx.cos());
    qround(q64(x, y, z, w, 0.0, hy.sin(), 0.0, hy.cos()))
}
pub fn qmul(a: Rotation, b: Rotation) -> Rotation {
    let ((ax, ay, az, aw), (bx, by, bz, bw)) = (q4(a), q4(b));
    qround(q64(ax, ay, az, aw, bx, by, bz, bw))
}
pub fn qmulv(q: Rotation, v: Vec3) -> Vec3 {
    let ((qx, qy, qz, qw), (vx, vy, vz)) = (q4(q), v64(v));
    let tx = 2.0 * (qy * vz - qz * vy);
    let ty = 2.0 * (qz * vx - qx * vz);
    let tz = 2.0 * (qx * vy - qy * vx);
    vec3(vx + qw * tx + (qy * tz - qz * ty), vy + qw * ty + (qz * tx - qx * tz), vz + qw * tz + (qx * ty - qy * tx))
}
pub fn qinverse(q: Rotation) -> Rotation {
    Rotation { x: -q.x, y: -q.y, z: -q.z, w: q.w }
}
pub fn qdegrees(q: Rotation) -> Vec3 {
    let (qx, qy, qz, qw) = q4(q);
    let m21 = (2.0 * (qy * qz + qx * qw)).clamp(-1.0, 1.0);
    let x = m21.asin();
    let z = (-(2.0 * (qx * qy - qz * qw))).atan2(1.0 - 2.0 * (qx * qx + qz * qz));
    let y = (-(2.0 * (qx * qz - qy * qw))).atan2(1.0 - 2.0 * (qx * qx + qy * qy));
    vec3(x / RAD, y / RAD, z / RAD)
}
/// C's %.7g: an f32's number to the 7 digits it holds.
fn g7(v: f64) -> f64 {
    format!("{:.6e}", v).parse().unwrap_or(v)
}
pub fn vtext(v: Vec3) -> String {
    let (x, y, z) = v64(v);
    format!("[{}, {}, {}]", ftext(g7(x) + 0.0), ftext(g7(y) + 0.0), ftext(g7(z) + 0.0))
}
pub fn rtext(q: Rotation) -> String {
    vtext(qdegrees(q))
}

/// An entity's ID as Solis writes it: 13 Crockford base-32 letters and digits. 0 is nothing.
pub fn etext(e: u64) -> String {
    if e == 0 {
        return "nothing".into();
    }
    const ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
    let mut out = [0u8; 13];
    let mut n = e;
    for slot in out.iter_mut().rev() {
        *slot = ALPHABET[(n & 31) as usize];
        n >>= 5;
    }
    String::from_utf8_lossy(&out).into_owned()
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
    if v.is_infinite() || !(-9.223_372_036_854_776e18..9.223_372_036_854_776e18).contains(&v) {
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
        fail(format!("Random's lowest number ({lo}) is bigger than its highest ({hi})."));
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

/// A decimal from lo up to (not quite) hi.
pub fn frandom(lo: f64, hi: f64) -> f64 {
    if lo > hi {
        fail(format!(
            "Random's lowest number ({}) is bigger than its highest ({}).",
            ftext(lo),
            ftext(hi)
        ));
    }
    let n = random(0, (1i64 << 53) - 1) as f64 / (1u64 << 53) as f64;
    lo + n * (hi - lo)
}

// ----- states: options are their full names, like "ActiveGame.Combat" -----

/// Is a the option b, or inside it?
/// == on state options: the same option, or one inside the other, whichever side it's written on.
/// match's when: the option a is b, or one inside b (not the other way round).
pub fn within(a: &str, b: &str) -> bool {
    a == b || (a.len() > b.len() && a.starts_with(b) && a.as_bytes()[b.len()] == b'.')
}

pub fn sin(a: &str, b: &str) -> bool {
    let inside = |a: &str, b: &str| a.len() > b.len() && a.starts_with(b) && a.as_bytes()[b.len()] == b'.';
    a == b || inside(a, b) || inside(b, a)
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
    let parts: Vec<String> = a.borrow().iter().map(f).collect();
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

/// Puts the rows in order by one value each row has. Rows with the same value keep their order.
pub fn csort<R: Row, K: PartialOrd>(c: &CMap<R>, key: impl Fn(&R) -> K) {
    let mut b = c.borrow_mut();
    let mut d: Vec<(K, Rc<RefCell<R>>)> = b
        .rows
        .drain(..)
        .map(|r| {
            let k = key(&r.borrow());
            (k, r)
        })
        .collect();
    d.sort_by(|x, y| x.0.partial_cmp(&y.0).unwrap_or(std::cmp::Ordering::Equal));
    b.rows = d.into_iter().map(|x| x.1).collect();
}

pub fn creverse<R: Row>(c: &CMap<R>) {
    c.borrow_mut().rows.reverse();
}

/// Puts an array of things in order by one of their fields.
pub fn asort_by<T: Clone, K: PartialOrd>(a: &Arr<T>, key: impl Fn(&T) -> K) {
    let items: Vec<T> = a.borrow().clone();
    let mut d: Vec<(K, T)> = items.into_iter().map(|x| (key(&x), x)).collect();
    d.sort_by(|x, y| x.0.partial_cmp(&y.0).unwrap_or(std::cmp::Ordering::Equal));
    *a.borrow_mut() = d.into_iter().map(|x| x.1).collect();
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
    a.borrow().contains(&x)
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

/// The local date and time now, as the computer's clock and time zone say: sec, min, hour, day
/// of the month, month from 0, years since 1900. local_fields, which knows the time zone, comes
/// with the program: from the computer in a program of its own, from the engine in a library.
fn local_now() -> [i32; 9] {
    let t = time_now().floor() as i64;
    local_fields(t).unwrap_or_else(|| utc_fields(t))
}

/// The date and time in UTC at t seconds since 1970, laid out like local_now's.
pub fn utc_fields(t: i64) -> [i32; 9] {
    let days = t.div_euclid(86400);
    let secs = t.rem_euclid(86400) as i32;
    let (y, m, d) = civil(days);
    [secs % 60, secs / 60 % 60, secs / 3600, d as i32, m as i32 - 1, y as i32 - 1900, 0, 0, 0]
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

// Dates are text like "2026-10-21". Days are counted from 1 January 1970.
fn days_from(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = if m > 2 { m - 3 } else { m + 9 };
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}
fn day_of(s: &str) -> Option<i64> {
    let b = s.as_bytes();
    if b.len() != 10 || b[4] != b'-' || b[7] != b'-' || !s.chars().enumerate().all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit()) {
        return None;
    }
    let (y, m, d): (i64, i64, i64) = (s[0..4].parse().ok()?, s[5..7].parse().ok()?, s[8..10].parse().ok()?);
    let z = days_from(y, m, d);
    if civil(z) != (y, m, d) {
        return None;
    }
    Some(z)
}
fn need_day(s: &str) -> i64 {
    match day_of(s) {
        Some(z) => z,
        None => fail(format!("\"{s}\" isn't a date. Dates are written like \"2026-10-21\".")),
    }
}
fn date_text(z: i64) -> String {
    let (y, m, d) = civil(z);
    format!("{y:04}-{m:02}-{d:02}")
}
pub fn time_isDate(s: String) -> bool {
    day_of(&s).is_some()
}
pub fn time_addDays(s: String, n: i64) -> String {
    date_text(need_day(&s) + n)
}
pub fn time_daysBetween(a: String, b: String) -> i64 {
    let x = need_day(&a);
    need_day(&b) - x
}
pub fn time_weekday(s: String) -> String {
    let names = ["Thursday", "Friday", "Saturday", "Sunday", "Monday", "Tuesday", "Wednesday"];
    names[need_day(&s).rem_euclid(7) as usize].to_string()
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
        Json::Bool(_) => "a bool",
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
        _ => misfit(at, "a bool (true or false)", v),
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
/// A schema's fields with no starting value must be in the JSON.
pub fn jr_required(v: &Json, at: &str, required: &[&str]) {
    for r in required {
        if jget(v, r).is_none() {
            fail(format!("{} is missing {r}.", place(at)));
        }
    }
}

pub fn jr_object(v: &Json, at: &str) -> bool {
    match v {
        Json::Null => false,
        Json::Obj(_) => true,
        _ => misfit(at, "an object", v),
    }
}
pub fn json_parse(text: &str) -> Json {
    let mut p = JP { b: text.as_bytes(), i: 0 };
    let v = p.value();
    p.skip();
    if p.i < p.b.len() {
        p.unexpected(p.i);
    }
    v
}

pub fn json_read<T>(text: String, f: &dyn Fn(&Json, &str) -> T) -> T {
    f(&json_parse(&text), "")
}

// ----- web: routes answered by methods -----

pub type Handler = fn(&str, &str, &str) -> (u16, String);

thread_local! {
    /// The status a failing request answers with: set by Web:fail, and for bad requests.
    static STATUS: Cell<u16> = const { Cell::new(0) };
    static HANDLER: Cell<Option<Handler>> = Cell::new(None);
}

pub fn web_routes(h: Handler) {
    HANDLER.with(|x| x.set(Some(h)));
}

pub fn web_fail(status: i64, msg: String) {
    STATUS.with(|s| s.set(status.clamp(100, 999) as u16));
    fail(msg)
}

fn bad(msg: String) -> ! {
    STATUS.with(|s| s.set(400));
    fail(msg)
}

/// The path without its ?query, and whether it fits a route's pattern: the :names' values if it does.
pub fn web_match(pattern: &str, path: &str) -> Option<Vec<(String, String)>> {
    let path = path.split('?').next().unwrap_or("");
    let want: Vec<&str> = pattern.split('/').filter(|s| !s.is_empty()).collect();
    let got: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    if want.len() != got.len() {
        return None;
    }
    let mut p = Vec::new();
    for (w, g) in want.iter().zip(&got) {
        if let Some(name) = w.strip_prefix(':') {
            p.push((name.to_string(), unescape(g)));
        } else if w != g {
            return None;
        }
    }
    Some(p)
}

fn unescape(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let Ok(n) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(n);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}

fn param<'a>(p: &'a [(String, String)], name: &str) -> &'a str {
    p.iter().find(|(n, _)| n == name).map(|(_, v)| v.as_str()).unwrap_or("")
}
pub fn web_string(p: &[(String, String)], name: &str) -> String {
    param(p, name).to_string()
}
pub fn web_int(p: &[(String, String)], name: &str) -> i64 {
    let v = param(p, name);
    if !isnum(v) {
        bad(format!("the path's {name} should be a number, but it's \"{v}\"."));
    }
    let n: f64 = v.trim().parse().unwrap_or(0.0);
    if n.fract() != 0.0 {
        bad(format!("the path's {name} should be a whole number, but it's \"{v}\"."));
    }
    n as i64
}
pub fn web_float(p: &[(String, String)], name: &str) -> f64 {
    let v = param(p, name);
    if !isnum(v) {
        bad(format!("the path's {name} should be a number, but it's \"{v}\"."));
    }
    v.trim().parse().unwrap_or(0.0)
}
pub fn web_body<T>(body: &str, f: &dyn Fn(&Json, &str) -> T) -> T {
    let mut out = None;
    if let Some(e) = attempt(|| out = Some(f(&json_parse(body), ""))) {
        if STATUS.with(|s| s.get()) == 0 {
            bad(format!("the request's body doesn't fit: {e}"));
        }
        fail(e);
    }
    out.unwrap()
}

fn errjson(msg: &str) -> String {
    format!("{{\"error\":{}}}", jquote(msg))
}

/// Runs one route's method: 200 with what it gives back, 204 when it gives nothing, and the
/// error's status otherwise.
pub fn web_run(f: &dyn Fn() -> Option<String>) -> (u16, String) {
    STATUS.with(|s| s.set(0));
    let mut out = None;
    match attempt(|| out = Some(f())) {
        None => match out.unwrap() {
            Some(s) => (200, s),
            None => (204, String::new()),
        },
        Some(e) => {
            let s = STATUS.with(|s| s.get());
            (if s == 0 { 500 } else { s }, errjson(&e))
        }
    }
}

pub fn web_unanswered(method: &str, path: &str, allowed: &[&str]) -> (u16, String) {
    let path = path.split('?').next().unwrap_or("");
    if allowed.is_empty() {
        (404, errjson(&format!("there's nothing at {path}.")))
    } else {
        (405, errjson(&format!("{path} answers {}, not {method}.", allowed.join(" and "))))
    }
}

pub fn web_handle(method: &str, path: &str, body: &str) -> (u16, String) {
    match HANDLER.with(|h| h.get()) {
        Some(h) => h(method, path, body),
        None => web_unanswered(method, path, &[]),
    }
}

pub fn web_request(method: String, path: String, body: String) -> String {
    let (status, text) = web_handle(&method.to_uppercase(), &path, &body);
    if text.is_empty() {
        status.to_string()
    } else {
        format!("{status} {text}")
    }
}

// ----- saves: readable JSON files, name.save -----

pub fn save_exists(name: &str) -> bool {
    std::path::Path::new(&format!("{name}.save")).is_file()
}

pub fn save_write(name: &str, text: String) {
    if std::fs::write(format!("{name}.save"), text + "\n").is_err() {
        fail(format!("can't write the save {name}.save."));
    }
}

pub fn save_read(name: &str) -> Json {
    let text = match std::fs::read_to_string(format!("{name}.save")) {
        Ok(t) => t,
        Err(_) => fail(format!("there's no saved {name} yet. Check first: is saved {name}?")),
    };
    let mut out = None;
    if let Some(e) = attempt(|| out = Some(json_parse(&text))) {
        fail(format!("{name}.save isn't a save Lust can read: {e}"));
    }
    out.unwrap()
}

/// One thing from a save. Anything that doesn't fit stops the load before anything changes.
pub fn save_get<T>(d: &Json, name: &str, entry: &str, f: &dyn Fn(&Json, &str) -> T) -> T {
    let Some(v) = jget(d, entry) else {
        fail(format!(
            "{name}'s save doesn't fit this program, so nothing was loaded: it has no {entry}."
        ))
    };
    let mut out = None;
    if let Some(e) = attempt(|| out = Some(f(v, entry))) {
        fail(format!("{name}'s save doesn't fit this program, so nothing was loaded: {e}"));
    }
    out.unwrap()
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
    static JDEPTH: Cell<u32> = const { Cell::new(0) };
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
