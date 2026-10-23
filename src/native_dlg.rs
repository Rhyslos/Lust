// The computer's own boxes for choosing files: Files:chooseOpen, chooseSave and chooseFolder.
//
// rfd shows the box the system itself has: Windows' and macOS' own, and on Linux the desktop's.
// Each waits until a file is chosen, and gives back its whole path, or "" if the box was
// cancelled.
//
// While testing (LUST_VIEW_TEST), no box opens. The answers come from the test's `choose`
// commands, kept in LUST_CHOSEN one to a line, and a box with no answer waiting is cancelled.

fn testing() -> bool {
    std::env::var("LUST_VIEW_TEST").is_ok()
}

/// The next answer a test gave, or "" (cancelled).
fn answer() -> String {
    let all = std::env::var("LUST_CHOSEN").unwrap_or_default();
    let mut lines = all.lines();
    let first = lines.next().unwrap_or("").to_string();
    let rest: Vec<&str> = lines.collect();
    std::env::set_var("LUST_CHOSEN", rest.join("\n"));
    if first == "cancel" {
        String::new()
    } else {
        first
    }
}

fn shown(p: Option<std::path::PathBuf>) -> String {
    p.map(|p| p.to_string_lossy().into_owned()).unwrap_or_default()
}

/// A box that starts in the folder the program runs in.
fn dialog(title: &str) -> rfd::FileDialog {
    let mut d = rfd::FileDialog::new().set_title(title);
    if let Ok(here) = std::env::current_dir() {
        d = d.set_directory(here);
    }
    d
}

/// Chooses a file to open, of these types ("png", "jpg"), or of any type if there are none.
pub fn choose_open(title: String, types: Vec<String>) -> String {
    if testing() {
        return answer();
    }
    let exts: Vec<String> = types
        .iter()
        .map(|t| t.trim().trim_start_matches("*.").trim_start_matches('.').to_string())
        .filter(|t| !t.is_empty())
        .collect();
    let mut d = dialog(&title);
    if !exts.is_empty() {
        d = d.add_filter(exts.join(", "), &exts);
    }
    shown(d.pick_file())
}

/// Chooses where to save a file, suggesting a name for it.
pub fn choose_save(title: String, name: String) -> String {
    if testing() {
        return answer();
    }
    let mut d = dialog(&title);
    if !name.is_empty() {
        d = d.set_file_name(name);
    }
    shown(d.save_file())
}

/// Chooses a folder.
pub fn choose_folder(title: String) -> String {
    if testing() {
        return answer();
    }
    shown(dialog(&title).pick_folder())
}
