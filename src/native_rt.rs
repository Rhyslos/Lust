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

/// A component's field the engine fills in, read: before the engine has filled it, the program
/// stops, naming the field.
pub fn filled<T>(v: Option<T>, name: &str) -> Option<T> {
    match v {
        Some(v) => Some(v),
        None => fail(format!("{name} is filled in by the engine before the component's first update, and hasn't been yet.")),
    }
}

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

/// An int going into a sized number (int8 hp): it has to fit.
pub fn narrow<T: TryFrom<i64>>(v: i64, lo: i64, hi: i64, name: &str) -> T {
    match T::try_from(v) {
        Ok(x) => x,
        Err(_) => fail(format!("{v} doesn't fit in {} {name}, which holds {lo} to {hi}.", if name.starts_with('u') { "a" } else { "an" })),
    }
}

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
/// Rotation:toward(dir): forward (+x) along dir, up kept as near +z as it can be: a turn up or
/// down (around y), then around z like a compass. No roll. Straight up or down faces x (yaw 0).
pub fn rotation_toward(d: Vec3) -> Rotation {
    let (x, y, z) = v64(d);
    if x == 0.0 && y == 0.0 && z == 0.0 {
        fail("toward needs a direction, but [0.0, 0.0, 0.0] has no length.".into());
    }
    let yaw = if x != 0.0 || y != 0.0 { y.atan2(x) } else { 0.0 };
    let pitch = z.atan2((x * x + y * y).sqrt());
    let (hz, hy) = (yaw / 2.0, -pitch / 2.0);
    let (sz, cz, sy, cy) = (hz.sin(), hz.cos(), hy.sin(), hy.cos());
    qround((-sz * sy, cz * sy, sz * cy, cz * cy))
}
/// Rotation:around(axis, degrees): that many degrees around the axis, turning the way a right
/// hand's fingers curl when its thumb points along the axis.
pub fn rotation_around(a: Vec3, deg: f64) -> Rotation {
    let (x, y, z) = v64(a);
    let l = (x * x + y * y + z * z).sqrt();
    if l == 0.0 {
        fail("around needs an axis to turn around, but [0.0, 0.0, 0.0] has no length.".into());
    }
    let h = deg * RAD / 2.0;
    let s = h.sin();
    qround((x / l * s, y / l * s, z / l * s, h.cos()))
}
/// a:lerp(b, t): the smooth turn from a to b, the shortest way round. glam's Quat::slerp's steps,
/// worked out in f64 and rounded once, as the Lua runtime does.
pub fn qlerp(a: Rotation, b: Rotation, t: f64) -> Rotation {
    let ((ax, ay, az, aw), (mut bx, mut by, mut bz, mut bw)) = (q4(a), q4(b));
    let mut dot = ax * bx + ay * by + az * bz + aw * bw;
    if dot < 0.0 {
        (bx, by, bz, bw, dot) = (-bx, -by, -bz, -bw, -dot);
    }
    if dot > 1.0 - f32::EPSILON as f64 {
        let (x, y, z, w) = (ax + (bx - ax) * t, ay + (by - ay) * t, az + (bz - az) * t, aw + (bw - aw) * t);
        let l = (x * x + y * y + z * z + w * w).sqrt();
        return qround((x / l, y / l, z / l, w / l));
    }
    let theta = dot.acos();
    let (s1, s2) = ((theta * (1.0 - t)).sin(), (theta * t).sin());
    let inv = 1.0 / theta.sin();
    qround((
        (ax * s1 + bx * s2) * inv,
        (ay * s1 + by * s2) * inv,
        (az * s1 + bz * s2) * inv,
        (aw * s1 + bw * s2) * inv,
    ))
}
/// a:angle(b): the angle between two directions, in degrees, from 0 to 180.
pub fn vangle(a: Vec3, b: Vec3) -> f64 {
    let ((ax, ay, az), (bx, by, bz)) = (v64(a), v64(b));
    if (ax == 0.0 && ay == 0.0 && az == 0.0) || (bx == 0.0 && by == 0.0 && bz == 0.0) {
        fail("angle needs two directions, but one of them is [0.0, 0.0, 0.0], which has no length.".into());
    }
    let (cx, cy, cz) = (ay * bz - az * by, az * bx - ax * bz, ax * by - ay * bx);
    let dot = ax * bx + ay * by + az * bz;
    (cx * cx + cy * cy + cz * cz).sqrt().atan2(dot) / RAD
}
// ----- vec2s -----
// Two f32s, for 2D. Worked out in f64 and rounded to f32, as the Lua runtime does.

#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

pub fn vec2(x: f64, y: f64) -> Vec2 {
    Vec2 { x: x as f32, y: y as f32 }
}
fn w64(v: Vec2) -> (f64, f64) {
    (v.x as f64, v.y as f64)
}
pub fn v2with(v: Vec2, f: usize, n: f64) -> Vec2 {
    let mut w = v;
    if f == 0 {
        w.x = n as f32;
    } else {
        w.y = n as f32;
    }
    w
}
pub fn v2add(a: Vec2, b: Vec2) -> Vec2 {
    let ((ax, ay), (bx, by)) = (w64(a), w64(b));
    vec2(ax + bx, ay + by)
}
pub fn v2sub(a: Vec2, b: Vec2) -> Vec2 {
    let ((ax, ay), (bx, by)) = (w64(a), w64(b));
    vec2(ax - bx, ay - by)
}
pub fn v2mul(a: Vec2, n: f64) -> Vec2 {
    let (x, y) = w64(a);
    vec2(x * n, y * n)
}
pub fn v2div(a: Vec2, n: f64) -> Vec2 {
    let (x, y) = w64(a);
    vec2(x / n, y / n)
}
pub fn v2neg(a: Vec2) -> Vec2 {
    Vec2 { x: -a.x, y: -a.y }
}
pub fn v2length(a: Vec2) -> f64 {
    let (x, y) = w64(a);
    (x * x + y * y).sqrt()
}
pub fn v2normalized(a: Vec2) -> Vec2 {
    let (x, y) = w64(a);
    let l = (x * x + y * y).sqrt();
    if l == 0.0 {
        return a;
    }
    vec2(x / l, y / l)
}
pub fn v2dot(a: Vec2, b: Vec2) -> f64 {
    let ((ax, ay), (bx, by)) = (w64(a), w64(b));
    ax * bx + ay * by
}
/// a:cross(b): which way b turns from a, above 0 to the left (counter-clockwise).
pub fn v2cross(a: Vec2, b: Vec2) -> f64 {
    let ((ax, ay), (bx, by)) = (w64(a), w64(b));
    ax * by - ay * bx
}
pub fn v2distance(a: Vec2, b: Vec2) -> f64 {
    let ((ax, ay), (bx, by)) = (w64(a), w64(b));
    let (dx, dy) = (ax - bx, ay - by);
    (dx * dx + dy * dy).sqrt()
}
pub fn v2lerp(a: Vec2, b: Vec2, t: f64) -> Vec2 {
    let ((ax, ay), (bx, by)) = (w64(a), w64(b));
    vec2(ax + (bx - ax) * t, ay + (by - ay) * t)
}
/// a:angle(b): the angle between two directions, in degrees, from 0 to 180.
pub fn v2angle(a: Vec2, b: Vec2) -> f64 {
    let ((ax, ay), (bx, by)) = (w64(a), w64(b));
    if (ax == 0.0 && ay == 0.0) || (bx == 0.0 && by == 0.0) {
        fail("angle needs two directions, but one of them is [0.0, 0.0], which has no length.".into());
    }
    (ax * by - ay * bx).abs().atan2(ax * bx + ay * by) / RAD
}
/// The sine and cosine of a number of degrees, exactly 0 and 1 on a quarter turn: d % 360 worked
/// out as Lua does (d - floor(d / 360) * 360).
fn sincos(d: f64) -> (f64, f64) {
    let r = d - (d / 360.0).floor() * 360.0;
    if r == 0.0 {
        (0.0, 1.0)
    } else if r == 90.0 {
        (1.0, 0.0)
    } else if r == 180.0 {
        (0.0, -1.0)
    } else if r == 270.0 {
        (-1.0, 0.0)
    } else {
        ((d * RAD).sin(), (d * RAD).cos())
    }
}
/// v:rotated(degrees): turned around its origin, counter-clockwise for more than 0.
pub fn v2rotated(v: Vec2, d: f64) -> Vec2 {
    let (x, y) = w64(v);
    let (s, c) = sincos(d);
    vec2(x * c - y * s, x * s + y * c)
}
pub fn v2withZ(v: Vec2, z: f64) -> Vec3 {
    Vec3 { x: v.x, y: v.y, z: z as f32 }
}
/// pos.xy: a vec3's x and y.
pub fn vxy(v: Vec3) -> Vec2 {
    Vec2 { x: v.x, y: v.y }
}
pub fn v2text(v: Vec2) -> String {
    let (x, y) = w64(v);
    format!("[{}, {}]", ftext(g7(x) + 0.0), ftext(g7(y) + 0.0))
}

// ----- colours -----
// Red, green, blue and see-through, each an f32 from 0 to 1. Made from numbers worked out while
// running, each part is kept between 0 and 1 (and not a number becomes 0), as in the Lua runtime.

#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub struct Colour {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Colour {
    /// Its parts as [r, g, b, a].
    pub fn to_array(self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }
}

impl From<[f32; 4]> for Colour {
    /// From [r, g, b, a], each kept between 0 and 1.
    fn from(c: [f32; 4]) -> Colour {
        colour(c[0] as f64, c[1] as f64, c[2] as f64, c[3] as f64)
    }
}

fn c01(x: f64) -> f32 {
    if !(x > 0.0) {
        0.0
    } else if x > 1.0 {
        1.0
    } else {
        x as f32
    }
}
pub fn colour(r: f64, g: f64, b: f64, a: f64) -> Colour {
    Colour { r: c01(r), g: c01(g), b: c01(b), a: c01(a) }
}
/// A colour the program using a library made: each part kept between 0 and 1.
pub fn cfix(c: Colour) -> Colour {
    colour(c.r as f64, c.g as f64, c.b as f64, c.a as f64)
}
/// From bytes 0 to 255: a colour word's, or hex text's.
pub fn rgb(r: f64, g: f64, b: f64, a: f64) -> Colour {
    colour(r / 255.0, g / 255.0, b / 255.0, a / 255.0)
}
pub fn colwith(c: Colour, f: usize, n: f64) -> Colour {
    let mut w = c;
    let v = c01(n);
    match f {
        0 => w.r = v,
        1 => w.g = v,
        2 => w.b = v,
        _ => w.a = v,
    }
    w
}
pub fn colfade(c: Colour, a: f64) -> Colour {
    colour(c.r as f64, c.g as f64, c.b as f64, a)
}
pub fn collerp(a: Colour, b: Colour, t: f64) -> Colour {
    let p = |x: f32, y: f32| x as f64 + (y as f64 - x as f64) * t;
    colour(p(a.r, b.r), p(a.g, b.g), p(a.b, b.b), p(a.a, b.a))
}
fn byte(x: f32) -> u32 {
    (x as f64 * 255.0 + 0.5).floor() as u32
}
pub fn colhex(c: Colour) -> String {
    let mut s = format!("#{:02x}{:02x}{:02x}", byte(c.r), byte(c.g), byte(c.b));
    if c.a != 1.0 {
        s.push_str(&format!("{:02x}", byte(c.a)));
    }
    s
}
/// Colour:hex("#3a8f2a") and Colour:hex("#3a8f2a80").
pub fn colour_hex(text: String) -> Colour {
    let hex = text.strip_prefix('#').unwrap_or("");
    if !text.starts_with('#') || !matches!(hex.len(), 6 | 8) || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        fail(format!(
            "hex needs a colour written as \"#rrggbb\" or \"#rrggbbaa\", like \"#3a8f2a\", but this is \"{text}\"."
        ));
    }
    let part = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).unwrap_or(0) as f64;
    rgb(part(0), part(2), part(4), if hex.len() == 8 { part(6) } else { 255.0 })
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

/// a << n: stops, as * does, when the bits pushed out the top would change the number.
pub fn shl(a: i64, n: i64) -> i64 {
    if !(0..64).contains(&n) {
        fail(format!("<< moves bits by 0 to 63 places, but this is {n}."));
    }
    let v = a << n;
    if v >> n != a {
        fail("a number got too big for an int.".into());
    }
    v
}

/// a >> n: keeps the sign, so -8 >> 1 is -4.
pub fn shr(a: i64, n: i64) -> i64 {
    if !(0..64).contains(&n) {
        fail(format!(">> moves bits by 0 to 63 places, but this is {n}."));
    }
    a >> n
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

/// Random:seed(42): from here on, the same numbers every run, as LUST_SEED gives.
pub fn random_seed(s: i64) {
    RNG.with(|c| c.set((s as u64).wrapping_mul(0x9E3779B97F4A7C15) | 1));
}

/// Random:chance(0.25): true about a quarter of the time. It always rolls, even for 0 and 1, so
/// the numbers after it are the same whatever the chance was.
pub fn random_chance(p: f64) -> bool {
    if !(0.0..=1.0).contains(&p) {
        fail(format!("chance takes a number from 0 to 1, but this is {}.", ftext(p)));
    }
    frandom(0.0, 1.0) < p
}

/// Random:direction(): a point on a ball of size 1, every way equally likely. The same steps as
/// the Lua runtime's, so both give the same vec3.
pub fn random_direction() -> Vec3 {
    let z = frandom(-1.0, 1.0);
    let a = frandom(0.0, 2.0 * std::f64::consts::PI);
    let r = (1.0 - z * z).sqrt();
    vec3(r * a.cos(), r * a.sin(), z)
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
    let start = std::time::Instant::now();
    let until = |secs: f64| {
        let due = start + std::time::Duration::from_secs_f64(secs.max(0.0));
        let now = std::time::Instant::now();
        if due > now {
            std::thread::sleep(due - now);
        }
    };
    // Tweens move on during the wait, a 1/60 s step at a time, on the clock wait keeps.
    let steps = TWEENS.with(|t| {
        let mut t = t.borrow_mut();
        if t.stepping || t.viewing > 0 {
            return None;
        }
        let before = t.clock_now;
        t.clock_now += s;
        let last = ((t.clock_now + 1e-9) * 60.0).floor() as i64;
        let first = t.clock_ticks + 1;
        t.clock_ticks = t.clock_ticks.max(last);
        Some((before, first, last))
    });
    if let Some((before, first, last)) = steps {
        for k in first..=last {
            if TWEENS.with(|t| t.borrow().live) == 0 {
                break;
            }
            until(k as f64 / 60.0 - before);
            tween_step(1.0 / 60.0);
        }
    }
    until(s);
}

// ----- easing and tweens -----
// The same steps, in the same order, are in runtime.lua, so both give exactly the same numbers.

/// 2 to the power y, worked out by the C library's pow as Lua's `2 ^ y` is. black_box keeps the
/// compiler from turning it into exp2, which can differ in the last bit.
fn pow2(y: f64) -> f64 {
    std::hint::black_box(2.0f64).powf(y)
}

fn out_bounce(x: f64) -> f64 {
    let (n1, d1) = (7.5625, 2.75);
    if x < 1.0 / d1 {
        n1 * x * x
    } else if x < 2.0 / d1 {
        let x = x - 1.5 / d1;
        n1 * x * x + 0.75
    } else if x < 2.5 / d1 {
        let x = x - 2.25 / d1;
        n1 * x * x + 0.9375
    } else {
        let x = x - 2.625 / d1;
        n1 * x * x + 0.984375
    }
}

/// The easing curves: the standard formulas from https://easings.net, each taking x from 0 to 1.
/// Whole powers are written as multiplications, and 2 ^ y as pow, the same way in Lua.
pub fn ease(name: &str, x: f64) -> f64 {
    use std::f64::consts::PI;
    const C1: f64 = 1.70158;
    const C2: f64 = C1 * 1.525;
    const C3: f64 = C1 + 1.0;
    let c4 = (2.0 * PI) / 3.0;
    let c5 = (2.0 * PI) / 4.5;
    match name {
        "InSine" => 1.0 - ((x * PI) / 2.0).cos(),
        "OutSine" => ((x * PI) / 2.0).sin(),
        "InOutSine" => -((PI * x).cos() - 1.0) / 2.0,
        "InQuad" => x * x,
        "OutQuad" => {
            let u = 1.0 - x;
            1.0 - u * u
        }
        "InOutQuad" if x < 0.5 => 2.0 * x * x,
        "InOutQuad" => {
            let u = -2.0 * x + 2.0;
            1.0 - u * u / 2.0
        }
        "InCubic" => x * x * x,
        "OutCubic" => {
            let u = 1.0 - x;
            1.0 - u * u * u
        }
        "InOutCubic" if x < 0.5 => 4.0 * x * x * x,
        "InOutCubic" => {
            let u = -2.0 * x + 2.0;
            1.0 - u * u * u / 2.0
        }
        "InQuart" => x * x * x * x,
        "OutQuart" => {
            let u = 1.0 - x;
            1.0 - u * u * u * u
        }
        "InOutQuart" if x < 0.5 => 8.0 * x * x * x * x,
        "InOutQuart" => {
            let u = -2.0 * x + 2.0;
            1.0 - u * u * u * u / 2.0
        }
        "InQuint" => x * x * x * x * x,
        "OutQuint" => {
            let u = 1.0 - x;
            1.0 - u * u * u * u * u
        }
        "InOutQuint" if x < 0.5 => 16.0 * x * x * x * x * x,
        "InOutQuint" => {
            let u = -2.0 * x + 2.0;
            1.0 - u * u * u * u * u / 2.0
        }
        "InExpo" if x == 0.0 => 0.0,
        "InExpo" => pow2(10.0 * x - 10.0),
        "OutExpo" if x == 1.0 => 1.0,
        "OutExpo" => 1.0 - pow2(-10.0 * x),
        "InOutExpo" if x == 0.0 => 0.0,
        "InOutExpo" if x == 1.0 => 1.0,
        "InOutExpo" if x < 0.5 => pow2(20.0 * x - 10.0) / 2.0,
        "InOutExpo" => (2.0 - pow2(-20.0 * x + 10.0)) / 2.0,
        "InCirc" => 1.0 - (1.0 - x * x).sqrt(),
        "OutCirc" => {
            let u = x - 1.0;
            (1.0 - u * u).sqrt()
        }
        "InOutCirc" if x < 0.5 => {
            let u = 2.0 * x;
            (1.0 - (1.0 - u * u).sqrt()) / 2.0
        }
        "InOutCirc" => {
            let u = -2.0 * x + 2.0;
            ((1.0 - u * u).sqrt() + 1.0) / 2.0
        }
        "InBack" => C3 * x * x * x - C1 * x * x,
        "OutBack" => {
            let u = x - 1.0;
            1.0 + C3 * u * u * u + C1 * u * u
        }
        "InOutBack" if x < 0.5 => {
            let u = 2.0 * x;
            (u * u * ((C2 + 1.0) * 2.0 * x - C2)) / 2.0
        }
        "InOutBack" => {
            let u = 2.0 * x - 2.0;
            (u * u * ((C2 + 1.0) * (x * 2.0 - 2.0) + C2) + 2.0) / 2.0
        }
        "InElastic" | "OutElastic" | "InOutElastic" if x == 0.0 => 0.0,
        "InElastic" | "OutElastic" | "InOutElastic" if x == 1.0 => 1.0,
        "InElastic" => -(pow2(10.0 * x - 10.0)) * ((x * 10.0 - 10.75) * c4).sin(),
        "OutElastic" => pow2(-10.0 * x) * ((x * 10.0 - 0.75) * c4).sin() + 1.0,
        "InOutElastic" if x < 0.5 => -(pow2(20.0 * x - 10.0) * ((20.0 * x - 11.125) * c5).sin()) / 2.0,
        "InOutElastic" => (pow2(-20.0 * x + 10.0) * ((20.0 * x - 11.125) * c5).sin()) / 2.0 + 1.0,
        "InBounce" => 1.0 - out_bounce(1.0 - x),
        "OutBounce" => out_bounce(x),
        "InOutBounce" if x < 0.5 => (1.0 - out_bounce(1.0 - 2.0 * x)) / 2.0,
        "InOutBounce" => (1.0 + out_bounce(2.0 * x - 1.0)) / 2.0,
        _ => x,
    }
}

/// Ease:apply(Ease.OutBack, t): t is kept from 0 to 1.
pub fn ease_apply(e: &str, t: f64) -> f64 {
    let t = if t < 0.0 {
        0.0
    } else if t > 1.0 {
        1.0
    } else {
        t
    };
    ease(e, t)
}

/// What a tween can move, and how each blends from where it started to where it's going.
pub trait Tweenable: Clone + 'static {
    fn blend(a: &Self, b: &Self, e: f64) -> Self;
}
impl Tweenable for f64 {
    fn blend(a: &f64, b: &f64, e: f64) -> f64 {
        a + (b - a) * e
    }
}
impl Tweenable for i64 {
    fn blend(a: &i64, b: &i64, e: f64) -> i64 {
        let (a, b) = (*a as f64, *b as f64);
        maths_round(a + (b - a) * e)
    }
}
impl Tweenable for Vec3 {
    fn blend(a: &Vec3, b: &Vec3, e: f64) -> Vec3 {
        vlerp(*a, *b, e)
    }
}
impl Tweenable for Vec2 {
    fn blend(a: &Vec2, b: &Vec2, e: f64) -> Vec2 {
        v2lerp(*a, *b, e)
    }
}
impl Tweenable for Colour {
    fn blend(a: &Colour, b: &Colour, e: f64) -> Colour {
        collerp(*a, *b, e)
    }
}
impl Tweenable for Rotation {
    fn blend(a: &Rotation, b: &Rotation, e: f64) -> Rotation {
        qlerp(*a, *b, e)
    }
}

/// A variable at the top of the file that a tween changes: the tween shares it. Copying one (as
/// a hook does) copies its value, as for any other variable.
pub struct Tcell<T>(Rc<RefCell<T>>);
impl<T: Clone> Clone for Tcell<T> {
    fn clone(&self) -> Self {
        Tcell::new(self.get())
    }
}
impl<T: Clone> Tcell<T> {
    pub fn new(v: T) -> Self {
        Tcell(Rc::new(RefCell::new(v)))
    }
    pub fn get(&self) -> T {
        self.0.borrow().clone()
    }
    pub fn set(&self, v: T) {
        *self.0.borrow_mut() = v;
    }
    pub fn share(&self) -> Rc<RefCell<T>> {
        self.0.clone()
    }
}

/// Which place a tween changes: the thing (or a top-level variable's cell), and the field.
pub type TweenKey = (usize, &'static str);
pub fn tween_key<T>(owner: &Rc<T>, field: &'static str) -> TweenKey {
    (Rc::as_ptr(owner) as *const () as usize, field)
}

struct Tween {
    key: TweenKey,
    secs: f64,
    /// How long it has run: the 1/60 s steps of wait's clock and a test's, counted on their own so
    /// 15 of them are exactly 0.25 s, and other steps (a window's frames) added up as they are.
    ticks: u64,
    extra: f64,
    ease: &'static str,
    /// Sets the place: Some(eased) on the way, None exactly where it's going.
    apply: Box<dyn FnMut(Option<f64>)>,
    done: Option<Rc<dyn Fn()>>,
}

#[derive(Default)]
struct Tweens {
    /// The running tweens, in the order they started. One that's replaced leaves None in its
    /// slot until the end of the step.
    list: Vec<Option<Tween>>,
    places: HashMap<TweenKey, usize>,
    live: usize,
    /// A step is running (its done blocks can't step again).
    stepping: bool,
    /// A window is open, and its clock moves the tweens, not wait's.
    viewing: i64,
    /// The clock wait moves: the seconds waited, and the 1/60 s steps taken.
    clock_now: f64,
    clock_ticks: i64,
}

thread_local! {
    static TWEENS: RefCell<Tweens> = RefCell::new(Tweens::default());
}

/// tween place to value over seconds: replaces a tween already moving the place (its done block
/// doesn't run). Over 0 seconds, it sets the place and runs the done block straight away.
pub fn tween<T: Tweenable>(key: TweenKey, from: T, to: T, secs: f64, ease: &'static str, mut set: Box<dyn FnMut(T)>, done: Option<Rc<dyn Fn()>>) {
    if !(secs >= 0.0) {
        fail(format!("a tween can't take {} seconds. It takes 0 seconds or more.", ftext(secs)));
    }
    TWEENS.with(|t| {
        let mut t = t.borrow_mut();
        if let Some(i) = t.places.remove(&key) {
            if t.list[i].take().is_some() {
                t.live -= 1;
            }
        }
    });
    if secs == 0.0 {
        set(to);
        if let Some(d) = done {
            d();
        }
        return;
    }
    let apply = Box::new(move |e: Option<f64>| match e {
        Some(e) => set(T::blend(&from, &to, e)),
        None => set(to.clone()),
    });
    TWEENS.with(|t| {
        let mut t = t.borrow_mut();
        let i = t.list.len();
        t.list.push(Some(Tween { key, secs, ticks: 0, extra: 0.0, ease, apply, done }));
        t.places.insert(key, i);
        t.live += 1;
    });
}

/// Moves every tween on by dt seconds, in the order they started; one that reaches its time is
/// set exactly to where it's going, then its done block runs. Tweens started during a step wait
/// for the next. Gives back whether any are still running (dt 0 only asks).
pub fn tween_step(dt: f64) -> bool {
    let n = TWEENS.with(|t| {
        let mut t = t.borrow_mut();
        if !(dt > 0.0) || t.stepping || t.live == 0 {
            return None;
        }
        t.stepping = true;
        Some(t.list.len())
    });
    if let Some(n) = n {
        // Closes the gaps, keeping the order, even if a done block fails.
        struct Tidy;
        impl Drop for Tidy {
            fn drop(&mut self) {
                TWEENS.with(|t| {
                    let mut t = t.borrow_mut();
                    let list: Vec<Option<Tween>> = std::mem::take(&mut t.list).into_iter().filter(Option::is_some).collect();
                    t.places = list.iter().enumerate().filter_map(|(i, w)| w.as_ref().map(|w| (w.key, i))).collect();
                    t.list = list;
                    t.stepping = false;
                });
            }
        }
        let _tidy = Tidy;
        for i in 0..n {
            let Some(mut w) = TWEENS.with(|t| t.borrow_mut().list[i].take()) else { continue };
            if dt == 1.0 / 60.0 {
                w.ticks += 1;
            } else {
                w.extra += dt;
            }
            let t = w.ticks as f64 / 60.0 + w.extra;
            if t + 1e-9 >= w.secs {
                TWEENS.with(|t| {
                    let mut t = t.borrow_mut();
                    t.places.remove(&w.key);
                    t.live -= 1;
                });
                (w.apply)(None);
                if let Some(d) = w.done {
                    d();
                }
            } else {
                (w.apply)(Some(ease(w.ease, t / w.secs)));
                TWEENS.with(|t| t.borrow_mut().list[i] = Some(w));
            }
        }
    }
    TWEENS.with(|t| t.borrow().live > 0)
}

/// The window's clock moving the tweens: a failure in a done block comes back as its message.
pub fn tween_view(dt: f64) -> Result<bool, String> {
    let mut busy = false;
    match attempt(|| busy = tween_step(dt)) {
        Some(e) => Err(e),
        None => Ok(busy),
    }
}

/// A window opens (1) or shuts (-1): while one is open, its clock moves the tweens.
pub fn viewing(d: i64) {
    TWEENS.with(|t| t.borrow_mut().viewing += d);
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
/// Shuffled in place, each item swapped with one at or before it (Fisher and Yates), from the
/// end down, the way the Lua runtime does it.
pub fn array_shuffle<T>(a: Arr<T>) {
    let mut v = a.borrow_mut();
    for i in (1..v.len()).rev() {
        let j = random(0, i as i64) as usize;
        v.swap(i, j);
    }
}
pub fn array_pick<T: Clone>(a: Arr<T>) -> T {
    let n = a.borrow().len();
    if n == 0 {
        fail("the array has no items, so there's nothing to pick.".into());
    }
    let i = random(0, n as i64 - 1) as usize;
    let v = a.borrow()[i].clone();
    v
}
pub fn array_sum_int(a: Arr<i64>) -> i64 {
    let mut s = 0i64;
    for x in a.borrow().iter() {
        s += *x;
    }
    s
}
pub fn array_sum_float(a: Arr<f64>) -> f64 {
    let mut s = 0.0;
    for x in a.borrow().iter() {
        s += *x;
    }
    s
}
fn none<T>(a: &Arr<T>, what: &str) {
    if a.borrow().is_empty() {
        fail(format!("the array has no items, so it has no {what}."));
    }
}
pub fn array_average_int(a: Arr<i64>) -> f64 {
    none(&a, "average");
    let n = a.borrow().len() as f64;
    array_sum_int(a) as f64 / n
}
pub fn array_average_float(a: Arr<f64>) -> f64 {
    none(&a, "average");
    let n = a.borrow().len() as f64;
    array_sum_float(a) / n
}
fn pick_by<T: PartialOrd + Copy>(a: Arr<T>, what: &str, better: fn(&T, &T) -> bool) -> T {
    none(&a, what);
    let v = a.borrow();
    let mut m = v[0];
    for x in v.iter().skip(1) {
        if better(x, &m) {
            m = *x;
        }
    }
    m
}
pub fn array_smallest_int(a: Arr<i64>) -> i64 {
    pick_by(a, "smallest item", |x, m| x < m)
}
pub fn array_largest_int(a: Arr<i64>) -> i64 {
    pick_by(a, "largest item", |x, m| x > m)
}
pub fn array_smallest_float(a: Arr<f64>) -> f64 {
    pick_by(a, "smallest item", |x, m| x < m)
}
pub fn array_largest_float(a: Arr<f64>) -> f64 {
    pick_by(a, "largest item", |x, m| x > m)
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
fn unit(name: &str, x: f64) {
    if !(-1.0..=1.0).contains(&x) {
        fail(format!("{name} takes a number from -1 to 1, but this is {}.", ftext(x)));
    }
}
pub fn maths_asin(x: f64) -> f64 {
    unit("asin", x);
    x.asin()
}
pub fn maths_acos(x: f64) -> f64 {
    unit("acos", x);
    x.acos()
}
pub fn maths_atan(x: f64) -> f64 {
    x.atan()
}
pub fn maths_atan2(y: f64, x: f64) -> f64 {
    y.atan2(x)
}
/// Worked out the same way as the Lua runtime does, so both give the same number.
pub fn maths_degrees(r: f64) -> f64 {
    r * 180.0 / std::f64::consts::PI
}
pub fn maths_radians(d: f64) -> f64 {
    d * std::f64::consts::PI / 180.0
}
pub fn maths_pi() -> f64 {
    std::f64::consts::PI
}
pub fn maths_e() -> f64 {
    std::f64::consts::E
}
pub fn maths_sign_int(x: i64) -> i64 {
    x.signum()
}
pub fn maths_sign_float(x: f64) -> f64 {
    if x > 0.0 {
        1.0
    } else if x < 0.0 {
        -1.0
    } else if x == 0.0 {
        0.0
    } else {
        x
    }
}
pub fn maths_trunc(x: f64) -> i64 {
    toint(x)
}
/// What's after the point, with x's sign. + 0.0 turns -0.0 (from -2.0) into 0.0.
pub fn maths_fraction(x: f64) -> f64 {
    x % 1.0 + 0.0
}
pub fn maths_log(x: f64) -> f64 {
    if x <= 0.0 {
        fail(format!("there's no log of zero or a number below zero ({}).", ftext(x)));
    }
    x.ln()
}
pub fn maths_exp(x: f64) -> f64 {
    x.exp()
}
pub fn maths_isFinite(x: f64) -> bool {
    x.is_finite()
}
pub fn maths_lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}
pub fn maths_moveTowards(c: f64, t: f64, step: f64) -> f64 {
    if step < 0.0 {
        fail(format!("moveTowards's step can't be below zero, but this is {}.", ftext(step)));
    }
    if (t - c).abs() <= step {
        t
    } else if t > c {
        c + step
    } else {
        c - step
    }
}
pub fn maths_smoothstep(lo: f64, hi: f64, x: f64) -> f64 {
    if lo == hi {
        fail(format!("smoothstep needs low and high to be different, but both are {}.", ftext(lo)));
    }
    let mut t = (x - lo) / (hi - lo);
    if t < 0.0 {
        t = 0.0;
    } else if t > 1.0 {
        t = 1.0;
    }
    t * t * (3.0 - 2.0 * t)
}
/// Ken Perlin's own table of 0 to 255, for his improved noise (2002).
const PERM: [u8; 256] = [
    151, 160, 137, 91, 90, 15, 131, 13, 201, 95, 96, 53, 194, 233, 7, 225, 140, 36, 103, 30, 69, 142, 8, 99, 37, 240,
    21, 10, 23, 190, 6, 148, 247, 120, 234, 75, 0, 26, 197, 62, 94, 252, 219, 203, 117, 35, 11, 32, 57, 177, 33, 88,
    237, 149, 56, 87, 174, 20, 125, 136, 171, 168, 68, 175, 74, 165, 71, 134, 139, 48, 27, 166, 77, 146, 158, 231, 83,
    111, 229, 122, 60, 211, 133, 230, 220, 105, 92, 41, 55, 46, 245, 40, 244, 102, 143, 54, 65, 25, 63, 161, 1, 216, 80,
    73, 209, 76, 132, 187, 208, 89, 18, 169, 200, 196, 135, 130, 116, 188, 159, 86, 164, 100, 109, 198, 173, 186, 3, 64,
    52, 217, 226, 250, 124, 123, 5, 202, 38, 147, 118, 126, 255, 82, 85, 212, 207, 206, 59, 227, 47, 16, 58, 17, 182,
    189, 28, 42, 223, 183, 170, 213, 119, 248, 152, 2, 44, 154, 163, 70, 221, 153, 101, 155, 167, 43, 172, 9, 129, 22,
    39, 253, 19, 98, 108, 110, 79, 113, 224, 232, 178, 185, 112, 104, 218, 246, 97, 228, 251, 34, 242, 193, 238, 210,
    144, 12, 191, 179, 162, 241, 81, 51, 145, 235, 249, 14, 239, 107, 49, 192, 214, 31, 181, 199, 106, 157, 184, 84, 204,
    176, 115, 121, 50, 45, 127, 4, 150, 254, 138, 236, 205, 93, 222, 114, 67, 29, 24, 72, 243, 141, 128, 195, 78, 66,
    215, 61, 156, 180,
];
fn perm(i: usize) -> usize {
    PERM[i & 255] as usize
}
fn fade(t: f64) -> f64 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}
fn nlerp(t: f64, a: f64, b: f64) -> f64 {
    a + t * (b - a)
}
fn grad(h: usize, x: f64, y: f64, z: f64) -> f64 {
    let h = h & 15;
    let mut u = if h < 8 { x } else { y };
    let mut v = if h < 4 {
        y
    } else if h == 12 || h == 14 {
        x
    } else {
        z
    };
    if h & 1 == 1 {
        u = -u;
    }
    if h & 2 == 2 {
        v = -v;
    }
    u + v
}
fn ordinary(n: f64) {
    if !n.is_finite() {
        fail(format!("noise needs ordinary numbers, but this is {}.", ftext(n)));
    }
}
/// Perlin noise: smooth hills and valleys from -1 to 1, the same for the same numbers on every
/// run. The same steps as the Lua runtime's, so both give the same number.
fn perlin(x: f64, y: f64, z: f64) -> f64 {
    let (fx, fy, fz) = (x.floor(), y.floor(), z.floor());
    let (xi, yi, zi) = (
        (fx.rem_euclid(256.0)) as usize,
        (fy.rem_euclid(256.0)) as usize,
        (fz.rem_euclid(256.0)) as usize,
    );
    let (x, y, z) = (x - fx, y - fy, z - fz);
    let (u, v, w) = (fade(x), fade(y), fade(z));
    let a = perm(xi) + yi;
    let (aa, ab) = (perm(a) + zi, perm(a + 1) + zi);
    let b = perm(xi + 1) + yi;
    let (ba, bb) = (perm(b) + zi, perm(b + 1) + zi);
    let r = nlerp(
        w,
        nlerp(
            v,
            nlerp(u, grad(perm(aa), x, y, z), grad(perm(ba), x - 1.0, y, z)),
            nlerp(u, grad(perm(ab), x, y - 1.0, z), grad(perm(bb), x - 1.0, y - 1.0, z)),
        ),
        nlerp(
            v,
            nlerp(u, grad(perm(aa + 1), x, y, z - 1.0), grad(perm(ba + 1), x - 1.0, y, z - 1.0)),
            nlerp(u, grad(perm(ab + 1), x, y - 1.0, z - 1.0), grad(perm(bb + 1), x - 1.0, y - 1.0, z - 1.0)),
        ),
    );
    r.clamp(-1.0, 1.0)
}
pub fn maths_noise(x: f64, y: f64) -> f64 {
    ordinary(x);
    ordinary(y);
    perlin(x, y, 0.0)
}
pub fn maths_noise3(x: f64, y: f64, z: f64) -> f64 {
    ordinary(x);
    ordinary(y);
    ordinary(z);
    perlin(x, y, z)
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
/// vec3s, vec2s, rotations and colours: numbers in [ ], as read, each to be rounded to f32.
fn jr_list(v: &Json, at: &str, want: &str, counts: &[usize]) -> Vec<f64> {
    let Json::Arr(a) = v else { misfit(at, want, v) };
    if !counts.contains(&a.len()) {
        fail(format!(
            "{} should be {want}, but it has {} item{}.",
            place(at),
            a.len(),
            if a.len() == 1 { "" } else { "s" }
        ));
    }
    a.iter()
        .enumerate()
        .map(|(i, x)| {
            let iat = format!("{at}[{i}]");
            let Json::Num(n) = x else { misfit(&iat, "a number", x) };
            if (*n as f32).is_infinite() {
                fail(format!("the JSON at {iat} is too big to keep in 32 bits."));
            }
            *n
        })
        .collect()
}
pub fn jr_vec3(v: &Json, at: &str) -> Vec3 {
    let n = jr_list(v, at, "a vec3, three numbers in [ ]", &[3]);
    vec3(n[0], n[1], n[2])
}
pub fn jr_vec2(v: &Json, at: &str) -> Vec2 {
    let n = jr_list(v, at, "a vec2, two numbers in [ ]", &[2]);
    vec2(n[0], n[1])
}
/// Four numbers are a quaternion, as Json:write writes one; three are degrees around x, y and
/// z, as Lust writes a rotation.
pub fn jr_rotation(v: &Json, at: &str) -> Rotation {
    let n = jr_list(v, at, "a rotation, four numbers in [ ] (or three degrees)", &[3, 4]);
    if n.len() == 3 {
        return rot(n[0], n[1], n[2]);
    }
    let q = qround((n[0], n[1], n[2], n[3]));
    let (x, y, z, w) = q4(q);
    let len = (x * x + y * y + z * z + w * w).sqrt();
    if !((len - 1.0).abs() <= 0.001) {
        fail(format!(
            "{} is a rotation's four numbers, which should be 1 long together, but these are {} long.",
            place(at),
            ftext(len)
        ));
    }
    q
}
/// Hex text, "#rrggbb" or "#rrggbbaa", or three or four numbers from 0 to 1.
pub fn jr_colour(v: &Json, at: &str) -> Colour {
    let want = "a colour, \"#rrggbb\", \"#rrggbbaa\" or numbers in [ ]";
    if let Json::Str(s) = v {
        let hex = s.strip_prefix('#').unwrap_or("");
        if s.starts_with('#') && matches!(hex.len(), 6 | 8) && hex.bytes().all(|b| b.is_ascii_hexdigit()) {
            return colour_hex(s.clone());
        }
        fail(format!("{} should be {want}, but it's \"{s}\".", place(at)));
    }
    let n = jr_list(v, at, want, &[3, 4]);
    for (i, p) in n.iter().enumerate() {
        if !(*p >= 0.0 && *p <= 1.0) {
            fail(format!("the JSON at {at}[{i}] is {}, but a colour's parts go from 0 to 1.", ftext(*p)));
        }
    }
    colour(n[0], n[1], n[2], if n.len() == 4 { n[3] } else { 1.0 })
}
/// An f32 as the shortest text that reads back as exactly the same f32, written the same way as
/// the Lua runtime: plain from 1e-7 to below 1e21 (with ".0" when it's whole), and 1e-45 or
/// 3.4028235e38 beyond. Each try is correctly rounded, read back through an f64 and rounded to
/// f32, as Json:read does.
pub fn f32json(x: f32) -> String {
    if !x.is_finite() {
        fail(format!("JSON can't hold {}.", ftext(x as f64)));
    }
    if x == 0.0 {
        return if x.is_sign_negative() { "-0.0".into() } else { "0.0".into() };
    }
    let mut s = String::new();
    for p in 0..9 {
        s = format!("{:.*e}", p, x as f64);
        if s.parse::<f64>().map(|r| r as f32 == x).unwrap_or(false) {
            break;
        }
    }
    let (sign, s) = match s.strip_prefix('-') {
        Some(r) => ("-", r.to_string()),
        None => ("", s),
    };
    let (m, e) = s.split_once('e').unwrap_or((&s, "0"));
    let e: i32 = e.parse().unwrap_or(0);
    let digits = m.replace('.', "");
    let digits = digits.trim_end_matches('0');
    let digits = if digits.is_empty() { "0" } else { digits };
    let n = digits.len() as i32;
    if !(-7..21).contains(&e) {
        let mut out = format!("{sign}{}", &digits[..1]);
        if n > 1 {
            out.push('.');
            out.push_str(&digits[1..]);
        }
        return format!("{out}e{e}");
    }
    if e >= n - 1 {
        return format!("{sign}{digits}{}.0", "0".repeat((e - n + 1) as usize));
    }
    if e >= 0 {
        let (a, b) = digits.split_at((e + 1) as usize);
        return format!("{sign}{a}.{b}");
    }
    format!("{sign}0.{}{digits}", "0".repeat((-e - 1) as usize))
}
fn jnums(d: Option<usize>, xs: &[f32]) -> String {
    let parts: Vec<String> = xs.iter().map(|x| f32json(*x)).collect();
    format!("[{}]", parts.join(if d.is_some() { ", " } else { "," }))
}
pub fn jw_vec3(v: &Vec3, d: Option<usize>) -> String {
    jnums(d, &[v.x, v.y, v.z])
}
pub fn jw_vec2(v: &Vec2, d: Option<usize>) -> String {
    jnums(d, &[v.x, v.y])
}
/// A rotation is its quaternion, x, y, z and w: its degrees can't always give back exactly the
/// same turn.
pub fn jw_rotation(q: &Rotation, d: Option<usize>) -> String {
    jnums(d, &[q.x, q.y, q.z, q.w])
}
/// A colour is its hex text when that gives back exactly the same colour (every part a whole
/// number of 255ths), and its four numbers when it doesn't.
pub fn jw_colour(c: &Colour, d: Option<usize>) -> String {
    let parts = c.to_array();
    if parts.iter().all(|p| (byte(*p) as f64 / 255.0) as f32 == *p) {
        jquote(&colhex(*c))
    } else {
        jnums(d, &parts)
    }
}
/// A state's option a native method's body gave back, by its full name: one the state doesn't
/// have stops the program, on the line that called it.
pub fn host_state(v: &str, opts: &[&'static str], state: &str) -> &'static str {
    match opts.iter().find(|o| **o == v) {
        Some(o) => o,
        None => fail(format!(
            "the program gave back \"{v}\" for a {state}, but {state} has no such option: it has {}.",
            opts.join(", ")
        )),
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
