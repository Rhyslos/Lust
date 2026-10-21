// Asking other websites, with ureq. It goes into programs that use Web:get or Web:post, as
// `rt::net`, and lust uses the same code for LustScript, so both say the same things.

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .http_status_as_error(false)
        .timeout_global(Some(std::time::Duration::from_secs(30)))
        .build()
        .into()
}

/// GET, or POST with a JSON body. The answer's text, or why there isn't one.
pub fn fetch(url: &str, body: Option<&str>) -> Result<String, String> {
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err(format!("{url} isn't a web address. Addresses start with http:// or https://."));
    }
    let a = agent();
    let res = match body {
        None => a.get(url).call(),
        Some(b) => a.post(url).header("Content-Type", "application/json").send(b),
    };
    match res {
        Err(_) => Err(format!("can't reach {url}.")),
        Ok(mut r) => {
            let status = r.status().as_u16();
            let text = r.body_mut().read_to_string().unwrap_or_default();
            if status >= 400 {
                let short: String = text.chars().take(200).collect();
                Err(format!("{url} answered {status}: {}", short.trim()))
            } else {
                Ok(text)
            }
        }
    }
}

pub fn get(url: String) -> String {
    match fetch(&url, None) {
        Ok(t) => t,
        Err(e) => fail(e),
    }
}

pub fn post(url: String, body: String) -> String {
    match fetch(&url, Some(&body)) {
        Ok(t) => t,
        Err(e) => fail(e),
    }
}
