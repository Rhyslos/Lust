//! lust update: getting the newest Lust from its releases, and a note, at most once a day, when
//! a newer one is out. Releases are made by GitHub when a new version reaches main
//! (.github/workflows/release.yml): lust.exe, its installer, and SHA256SUMS to check them by.

use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Where Lust's releases are listed, newest first, test versions (like 2.0.0-alpha.38) included.
pub const RELEASES: &str = "https://api.github.com/repos/Rhyslos/Lust/releases?per_page=20";

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The file a release keeps Lust in.
const EXE: &str = "lust.exe";

/// A release that can be installed: its version, where lust.exe and its checksums are, and its
/// page.
#[derive(Debug, Clone, PartialEq)]
pub struct Release {
    pub version: String,
    pub exe: String,
    pub sums: String,
    pub page: String,
}

// ----- versions -----

/// One part of a version's test label: alpha.38 is "alpha" and 38.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Part {
    // Numbers come before words, as semantic versioning says.
    Num(u64),
    Word(String),
}

/// 2.0.0-alpha.38 as (2, 0, 0) and its label. None if it isn't a version.
fn parse(v: &str) -> Option<((u64, u64, u64), Vec<Part>)> {
    let v = v.trim().trim_start_matches('v');
    let v = v.split('+').next()?;
    let (core, label) = match v.split_once('-') {
        Some((c, l)) => (c, Some(l)),
        None => (v, None),
    };
    let nums: Vec<u64> = core.split('.').map(|n| n.parse().ok()).collect::<Option<_>>()?;
    let [a, b, c] = nums[..] else { return None };
    let parts = match label {
        None => vec![],
        Some(l) => l
            .split('.')
            .map(|p| p.parse().map(Part::Num).unwrap_or_else(|_| Part::Word(p.to_string())))
            .collect(),
    };
    Some(((a, b, c), parts))
}

/// Whether version a comes after version b. A test version comes before the version it's a test
/// of: 2.0.0-alpha.38 before 2.0.0.
pub fn newer(a: &str, b: &str) -> bool {
    let (Some((ca, la)), Some((cb, lb))) = (parse(a), parse(b)) else {
        return false;
    };
    if ca != cb {
        return ca > cb;
    }
    match (la.is_empty(), lb.is_empty()) {
        (true, true) => false,
        (true, false) => true,
        (false, true) => false,
        (false, false) => la > lb,
    }
}

// ----- releases -----

fn agent(timeout: Duration) -> ureq::Agent {
    ureq::Agent::config_builder()
        .http_status_as_error(false)
        .timeout_global(Some(timeout))
        .build()
        .into()
}

fn get(url: &str, timeout: Duration) -> Result<Vec<u8>, String> {
    let mut res = agent(timeout)
        .get(url)
        .header("User-Agent", "lust-update")
        .header("Accept", "application/vnd.github+json")
        .call()
        .map_err(|_| "can't reach GitHub, where Lust's releases are. Check the internet connection, and try again.".to_string())?;
    let status = res.status().as_u16();
    let body = res
        .body_mut()
        .with_config()
        .limit(200 << 20)
        .read_to_vec()
        .map_err(|_| "the download stopped part way. Try again.".to_string())?;
    if status >= 400 {
        return Err(format!("GitHub answered {status} for {url}. Try again later."));
    }
    Ok(body)
}

/// The newest release that has a lust.exe, from GitHub's list of releases (as JSON).
pub fn newest(list: &str) -> Option<Release> {
    use crate::native_rt::{json_parse, Json};
    let parsed = std::panic::catch_unwind(|| json_parse(list)).ok()?;
    let Json::Arr(releases) = parsed else { return None };
    fn field<'a>(o: &'a Json, k: &str) -> Option<&'a Json> {
        match o {
            Json::Obj(f) => f.iter().find(|(n, _)| n == k).map(|(_, v)| v),
            _ => None,
        }
    }
    fn text(o: &Json, k: &str) -> Option<String> {
        match field(o, k) {
            Some(Json::Str(s)) => Some(s.clone()),
            _ => None,
        }
    }
    let mut best: Option<Release> = None;
    for r in &releases {
        if matches!(field(r, "draft"), Some(Json::Bool(true))) {
            continue;
        }
        let Some(tag) = text(r, "tag_name") else { continue };
        let version = tag.trim_start_matches('v').to_string();
        if parse(&version).is_none() {
            continue;
        }
        let Some(Json::Arr(assets)) = field(r, "assets") else { continue };
        let url = |name: &str| {
            assets
                .iter()
                .find(|a| text(a, "name").as_deref() == Some(name))
                .and_then(|a| text(a, "browser_download_url"))
        };
        let (Some(exe), Some(sums)) = (url(EXE), url("SHA256SUMS")) else {
            continue;
        };
        let page = text(r, "html_url").unwrap_or_default();
        if best.as_ref().is_none_or(|b| newer(&version, &b.version)) {
            best = Some(Release { version, exe, sums, page });
        }
    }
    best
}

/// The newest release, asked of GitHub.
pub fn ask(api: &str, timeout: Duration) -> Result<Option<Release>, String> {
    let list = get(api, timeout)?;
    Ok(newest(&String::from_utf8_lossy(&list)))
}

/// SHA256SUMS's line for a file: its checksum, in lowercase hex.
fn sum_for(sums: &str, name: &str) -> Option<String> {
    sums.lines().find_map(|l| {
        let mut parts = l.split_whitespace();
        let sum = parts.next()?;
        let file = parts.next()?.trim_start_matches('*');
        (file == name).then(|| sum.to_lowercase())
    })
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Puts a release's lust.exe in place of `exe`, after checking it against SHA256SUMS. The
/// running Lust can't be written over on Windows, but it can be renamed, so the old one moves
/// to lust.old.exe, and goes the next time Lust starts.
pub fn install(release: &Release, exe: &Path) -> Result<(), String> {
    let new = get(&release.exe, Duration::from_secs(600))?;
    let sums = String::from_utf8_lossy(&get(&release.sums, Duration::from_secs(60))?).to_string();
    let Some(want) = sum_for(&sums, EXE) else {
        return Err(format!(
            "release {} doesn't say what {EXE}'s checksum is, so it isn't installed.",
            release.version
        ));
    };
    let got = hex(&Sha256::digest(&new));
    if got != want {
        return Err(format!(
            "the {EXE} downloaded for {} isn't the one the release lists (its checksum is different), so it isn't installed. Try again.",
            release.version
        ));
    }
    let dir = exe.parent().unwrap_or(Path::new("."));
    let fresh = dir.join("lust.new.exe");
    let old = dir.join("lust.old.exe");
    let cant = |e: std::io::Error| {
        format!(
            "can't change the files in {} ({e}). If Lust was put somewhere only an administrator can change, install the new one with lust-setup.exe from {}",
            dir.display(),
            release.page
        )
    };
    std::fs::write(&fresh, &new).map_err(cant)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&fresh, std::fs::Permissions::from_mode(0o755));
    }
    let _ = std::fs::remove_file(&old);
    if exe.exists() {
        if let Err(e) = std::fs::rename(exe, &old) {
            let _ = std::fs::remove_file(&fresh);
            return Err(cant(e));
        }
    }
    if let Err(e) = std::fs::rename(&fresh, exe) {
        // Put the old one back, so there's still a Lust.
        let _ = std::fs::rename(&old, exe);
        return Err(cant(e));
    }
    Ok(())
}

/// A Lust replaced by lust update leaves its old self behind; it goes on the next start.
pub fn tidy() {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let old = dir.join("lust.old.exe");
            if old.exists() && old != exe {
                let _ = std::fs::remove_file(old);
            }
        }
    }
}

// ----- the command -----

/// lust update, lust update check, lust update off, lust update on.
pub fn command(what: Option<&str>) -> std::process::ExitCode {
    use std::process::ExitCode;
    match what {
        Some("off") | Some("on") => {
            let on = what == Some("on");
            let mut s = Settings::load();
            s.notes = on;
            s.save();
            if on {
                println!("Lust will say, at most once a day, when a newer Lust is out.");
            } else {
                println!("Lust won't say when a newer Lust is out. lust update still gets it, and lust update on turns the notes back on.");
            }
            return ExitCode::SUCCESS;
        }
        None | Some("check") => {}
        Some(other) => {
            eprintln!("lust update doesn't know {other}. It's lust update, lust update check, lust update off or lust update on.");
            return ExitCode::from(2);
        }
    }
    let release = match ask(RELEASES, Duration::from_secs(30)) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(1);
        }
    };
    let mut s = Settings::load();
    s.checked = now();
    s.latest = release.as_ref().map(|r| r.version.clone()).unwrap_or_default();
    s.save();
    let Some(release) = release.filter(|r| newer(&r.version, VERSION)) else {
        println!("Lust {VERSION} is the newest.");
        return ExitCode::SUCCESS;
    };
    if what == Some("check") {
        println!(
            "Lust {} is out (this is {VERSION}). lust update gets it. What's new: {}",
            release.version, release.page
        );
        return ExitCode::SUCCESS;
    }
    if !cfg!(windows) {
        println!(
            "Lust {} is out (this is {VERSION}). Releases are made for Windows; here, build it from the source: https://github.com/Rhyslos/Lust",
            release.version
        );
        return ExitCode::SUCCESS;
    }
    let Ok(exe) = std::env::current_exe() else {
        eprintln!(
            "can't tell where this Lust is, so it can't be replaced. Install the new one with lust-setup.exe from {}",
            release.page
        );
        return ExitCode::from(1);
    };
    println!("Getting Lust {}…", release.version);
    match install(&release, &exe) {
        Ok(()) => {
            println!("Lust is now {} (it was {VERSION}). What's new: {}", release.version, release.page);
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{e}");
            ExitCode::from(1)
        }
    }
}

// ----- the note -----

/// What lust remembers about updates, in a small file of its own.
#[derive(Debug, Default, PartialEq)]
pub struct Settings {
    /// Whether to say when a newer Lust is out.
    pub notes: bool,
    /// When releases were last asked about, in seconds since 1970.
    pub checked: u64,
    /// The newest version then.
    pub latest: String,
}

/// The folder lust keeps its own settings in: %LOCALAPPDATA%\Lust, or ~/.config/lust.
fn settings_file() -> Option<PathBuf> {
    if let Ok(dir) = std::env::var("LUST_SETTINGS") {
        return Some(PathBuf::from(dir).join("update.txt"));
    }
    let dir = if cfg!(windows) {
        PathBuf::from(std::env::var("LOCALAPPDATA").ok()?).join("Lust")
    } else if let Ok(x) = std::env::var("XDG_CONFIG_HOME") {
        PathBuf::from(x).join("lust")
    } else {
        PathBuf::from(std::env::var("HOME").ok()?).join(".config").join("lust")
    };
    Some(dir.join("update.txt"))
}

impl Settings {
    pub fn read(text: &str) -> Settings {
        let mut s = Settings {
            notes: true,
            ..Default::default()
        };
        for line in text.lines() {
            match line.split_once('=') {
                Some(("notes", v)) => s.notes = v.trim() != "off",
                Some(("checked", v)) => s.checked = v.trim().parse().unwrap_or(0),
                Some(("latest", v)) => s.latest = v.trim().to_string(),
                _ => {}
            }
        }
        s
    }

    pub fn write(&self) -> String {
        format!(
            "# lust update's notes: whether to say when a newer Lust is out, and when it last asked.\nnotes={}\nchecked={}\nlatest={}\n",
            if self.notes { "on" } else { "off" },
            self.checked,
            self.latest
        )
    }

    fn load() -> Settings {
        settings_file()
            .and_then(|f| std::fs::read_to_string(f).ok())
            .map(|t| Settings::read(&t))
            .unwrap_or(Settings {
                notes: true,
                ..Default::default()
            })
    }

    fn save(&self) {
        if let Some(f) = settings_file() {
            if let Some(d) = f.parent() {
                let _ = std::fs::create_dir_all(d);
            }
            let _ = std::fs::write(f, self.write());
        }
    }
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// A check started alongside a command, so the command never waits for it.
pub struct Note(Option<std::thread::JoinHandle<Option<String>>>);

impl Note {
    /// No check, for commands that don't need one.
    pub fn none() -> Note {
        Note(None)
    }
}

/// Starts asking about releases, if it's been a day, notes are on, and a person is watching (not
/// CI, not a pipe). Never slows the command down: it runs beside it, and is dropped if it hasn't
/// finished when the command has.
pub fn start() -> Note {
    use std::io::IsTerminal;
    let s = Settings::load();
    let quiet = std::env::var_os("CI").is_some() || std::env::var_os("LUST_NO_UPDATE_NOTES").is_some();
    if !s.notes || quiet || !std::io::stderr().is_terminal() || now().saturating_sub(s.checked) < 24 * 3600 {
        return Note(None);
    }
    Note(Some(std::thread::spawn(|| {
        let release = ask(RELEASES, Duration::from_secs(5)).ok()?;
        let mut s = Settings::load();
        s.checked = now();
        s.latest = release.as_ref().map(|r| r.version.clone()).unwrap_or_default();
        s.save();
        release.map(|r| r.version)
    })))
}

/// After the command: the note, if the check found a newer Lust.
pub fn finish(note: Note) {
    let Some(h) = note.0 else { return };
    if !h.is_finished() {
        return;
    }
    if let Ok(Some(v)) = h.join() {
        if newer(&v, VERSION) {
            eprintln!("\nLust {v} is out (this is {VERSION}). lust update gets it; lust update off stops these notes.");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_in_order() {
        let order = [
            "1.9.9",
            "2.0.0-alpha.2",
            "2.0.0-alpha.10",
            "2.0.0-alpha.38",
            "2.0.0-beta",
            "2.0.0-beta.1",
            "2.0.0-rc.1",
            "2.0.0",
            "2.0.1",
            "2.1.0",
            "10.0.0",
        ];
        for (i, a) in order.iter().enumerate() {
            for (j, b) in order.iter().enumerate() {
                assert_eq!(newer(a, b), i > j, "{a} after {b}?");
            }
        }
        assert!(newer("v2.0.0-alpha.38", "2.0.0-alpha.37"));
        assert!(!newer("not a version", "1.0.0"));
        assert!(!newer("2.0", "1.0.0"));
    }

    fn release_json(tag: &str, draft: bool, assets: &[&str]) -> String {
        let a: Vec<String> = assets
            .iter()
            .map(|n| format!(r#"{{"name": "{n}", "browser_download_url": "https://example.com/{tag}/{n}"}}"#))
            .collect();
        format!(
            r#"{{"tag_name": "{tag}", "draft": {draft}, "html_url": "https://example.com/{tag}", "assets": [{}]}}"#,
            a.join(", ")
        )
    }

    #[test]
    fn newest_release_with_lust_in_it() {
        let list = format!(
            "[{}, {}, {}, {}]",
            release_json("v2.0.0-alpha.40", true, &["lust.exe", "SHA256SUMS"]),
            release_json("v2.0.0-alpha.39", false, &["lust-setup.exe"]),
            release_json("v2.0.0-alpha.38", false, &["lust.exe", "lust-setup.exe", "SHA256SUMS"]),
            release_json("v2.0.0-alpha.37", false, &["lust.exe", "SHA256SUMS"]),
        );
        let r = newest(&list).unwrap();
        assert_eq!(r.version, "2.0.0-alpha.38");
        assert_eq!(r.exe, "https://example.com/v2.0.0-alpha.38/lust.exe");
        assert_eq!(r.sums, "https://example.com/v2.0.0-alpha.38/SHA256SUMS");
        assert_eq!(newest("[]"), None);
        assert_eq!(newest("not json"), None);
        assert_eq!(newest(r#"{"message": "API rate limit exceeded"}"#), None);
    }

    #[test]
    fn settings_round_trip() {
        let s = Settings {
            notes: false,
            checked: 1761400000,
            latest: "2.0.0-alpha.38".into(),
        };
        assert_eq!(Settings::read(&s.write()), s);
        // A missing or broken file means notes on, never asked.
        assert_eq!(
            Settings::read(""),
            Settings {
                notes: true,
                ..Default::default()
            }
        );
        assert_eq!(
            Settings::read("checked=soon\nnotes=on"),
            Settings {
                notes: true,
                ..Default::default()
            }
        );
    }

    #[test]
    fn checksums() {
        let sums = "abc123  lust-setup.exe\nDEF456 *lust.exe\n";
        assert_eq!(sum_for(sums, "lust.exe").as_deref(), Some("def456"));
        assert_eq!(sum_for(sums, "lust-setup.exe").as_deref(), Some("abc123"));
        assert_eq!(sum_for(sums, "other"), None);
        assert_eq!(
            hex(&Sha256::digest(b"abc")),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    /// A stand-in for GitHub on this computer: serves a release whose lust.exe is `exe`, with
    /// `sums` as SHA256SUMS.
    fn serve(exe: Vec<u8>, sums: String) -> String {
        let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
        let addr = format!("http://{}", server.server_addr().to_ip().unwrap());
        std::thread::spawn(move || {
            for req in server.incoming_requests() {
                let body = match req.url() {
                    "/lust.exe" => exe.clone(),
                    "/SHA256SUMS" => sums.clone().into_bytes(),
                    _ => b"not here".to_vec(),
                };
                let _ = req.respond(tiny_http::Response::from_data(body));
            }
        });
        addr
    }

    fn folder(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("lust-update-test-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn install_replaces_lust_after_checking_it() {
        let new = b"the new lust".to_vec();
        let addr = serve(new.clone(), format!("{}  lust.exe\n", hex(&Sha256::digest(&new))));
        let d = folder("good");
        let exe = d.join("lust.exe");
        std::fs::write(&exe, b"the old lust").unwrap();
        let r = Release {
            version: "9.9.9".into(),
            exe: format!("{addr}/lust.exe"),
            sums: format!("{addr}/SHA256SUMS"),
            page: String::new(),
        };
        install(&r, &exe).unwrap();
        assert_eq!(std::fs::read(&exe).unwrap(), new);
        assert_eq!(std::fs::read(d.join("lust.old.exe")).unwrap(), b"the old lust");
        assert!(!d.join("lust.new.exe").exists());
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn install_refuses_a_wrong_download() {
        let addr = serve(
            b"something else".to_vec(),
            format!("{}  lust.exe\n", hex(&Sha256::digest(b"the real one"))),
        );
        let d = folder("bad");
        let exe = d.join("lust.exe");
        std::fs::write(&exe, b"the old lust").unwrap();
        let r = Release {
            version: "9.9.9".into(),
            exe: format!("{addr}/lust.exe"),
            sums: format!("{addr}/SHA256SUMS"),
            page: String::new(),
        };
        let e = install(&r, &exe).unwrap_err();
        assert!(e.contains("checksum is different"), "{e}");
        // Nothing changed.
        assert_eq!(std::fs::read(&exe).unwrap(), b"the old lust");
        assert!(!d.join("lust.new.exe").exists());
        let _ = std::fs::remove_dir_all(d);
    }

    #[test]
    fn install_refuses_without_a_checksum() {
        let addr = serve(b"new".to_vec(), "abc  lust-setup.exe\n".into());
        let d = folder("nosum");
        let exe = d.join("lust.exe");
        std::fs::write(&exe, b"old").unwrap();
        let r = Release {
            version: "9.9.9".into(),
            exe: format!("{addr}/lust.exe"),
            sums: format!("{addr}/SHA256SUMS"),
            page: String::new(),
        };
        assert!(install(&r, &exe).unwrap_err().contains("checksum"));
        assert_eq!(std::fs::read(&exe).unwrap(), b"old");
        let _ = std::fs::remove_dir_all(d);
    }
}

#[cfg(test)]
mod real_answer {
    /// A real answer from GitHub's releases list (another project's, its assets renamed), read
    /// from a file named in LUST_RELEASES_JSON, when there is one.
    #[test]
    fn reads_a_real_answer() {
        let Ok(path) = std::env::var("LUST_RELEASES_JSON") else { return };
        let text = std::fs::read_to_string(path).unwrap();
        let r = super::newest(&text);
        println!("{r:?}");
        assert!(r.is_some());
    }
}
