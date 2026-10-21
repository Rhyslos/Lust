// Text patterns, with the regex crate. It goes into programs that use :matches, :findAll,
// :replaceAll or :capture, as `rt::pat`, and lust uses the same code for LustScript.

fn pattern(p: &str) -> Result<regex::Regex, String> {
    regex::Regex::new(p).map_err(|e| {
        let first = e.to_string().lines().last().unwrap_or("").trim().to_string();
        format!("\"{p}\" isn't a pattern Lust understands: {first}")
    })
}

pub fn try_matches(s: &str, p: &str) -> Result<bool, String> {
    Ok(pattern(p)?.is_match(s))
}
pub fn try_find_all(s: &str, p: &str) -> Result<Vec<String>, String> {
    Ok(pattern(p)?.find_iter(s).map(|m| m.as_str().to_string()).collect())
}
pub fn try_replace_all(s: &str, p: &str, with: &str) -> Result<String, String> {
    Ok(pattern(p)?.replace_all(s, with).to_string())
}
/// The parts in ( ) of the first match, or nothing if it doesn't match.
pub fn try_capture(s: &str, p: &str) -> Result<Vec<String>, String> {
    let re = pattern(p)?;
    Ok(match re.captures(s) {
        Some(c) => c.iter().skip(1).map(|m| m.map(|m| m.as_str().to_string()).unwrap_or_default()).collect(),
        None => vec![],
    })
}

fn ok<T>(r: Result<T, String>) -> T {
    match r {
        Ok(v) => v,
        Err(e) => fail(e),
    }
}
pub fn matches(s: String, p: String) -> bool {
    ok(try_matches(&s, &p))
}
pub fn find_all(s: String, p: String) -> Arr<String> {
    arr(ok(try_find_all(&s, &p)))
}
pub fn replace_all(s: String, p: String, with: String) -> String {
    ok(try_replace_all(&s, &p, &with))
}
pub fn capture(s: String, p: String) -> Arr<String> {
    arr(ok(try_capture(&s, &p)))
}
