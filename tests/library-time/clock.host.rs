// The engine says the time zone is two hours ahead of UTC; Lust's dates and times follow it.
struct Host;

impl clock::Host for Host {
    fn local_time_offset(&self, _unix_seconds: i64) -> i64 {
        2 * 3600
    }
}

/// The date and time now, two hours ahead of UTC, worked out here.
fn expected() -> (String, String) {
    let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64 + 7200;
    let (days, secs) = (t.div_euclid(86400), t.rem_euclid(86400));
    // Days since 1970 to a date.
    let z = days + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    (format!("{y:04}-{m:02}-{d:02}"), format!("{:02}:{:02}:{:02}", secs / 3600, secs / 60 % 60, secs % 60))
}

fn main() {
    clock::set_host(Host);
    clock::start().unwrap();
    // A second can tick over between the two; then try again.
    for _ in 0..3 {
        let want = expected();
        let got = (clock::today().unwrap(), clock::now().unwrap());
        if got == want {
            println!("Time:date() and Time:time() follow the engine's time zone");
            return;
        }
    }
    println!("they don't follow the engine's time zone");
}
