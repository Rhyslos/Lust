// The computer's time zone, from its C library: for programs of their own (lust build) that use
// Time:date() or Time:time(), and for lust itself. A library for an engine asks the engine
// instead, and has no unsafe code at all.

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

/// The local date and time at t seconds since 1970, or None with no time zone to be had.
pub fn local_fields(t: i64) -> Option<[i32; 9]> {
    let mut tm = Tm { f: [0; 9], extra: [0; 4] };
    #[cfg(unix)]
    let ok = unsafe { !localtime_r(&t, &mut tm).is_null() };
    #[cfg(windows)]
    let ok = unsafe { _localtime64_s(&mut tm, &t) == 0 };
    #[cfg(not(any(unix, windows)))]
    let ok = false;
    ok.then_some(tm.f)
}
