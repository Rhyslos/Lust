// Serving routes over HTTP, with tiny_http. It goes into programs that call Web:serve, as
// `rt::web`, and lust uses the same code for LustScript, so both answer the same way.

/// Answers requests on the port until the program is stopped. Each request goes to `answer`,
/// which gives a status and JSON text.
pub fn serve_with(port: i64, answer: &mut dyn FnMut(&str, &str, &str) -> (u16, String)) -> Result<(), String> {
    if !(1..=65535).contains(&port) {
        return Err(format!("{port} isn't a port: ports go from 1 to 65535."));
    }
    let server = tiny_http::Server::http(format!("0.0.0.0:{port}")).map_err(|e| format!("can't serve on port {port}: {e}."))?;
    {
        use std::io::Write;
        println!("Serving on http://localhost:{port}. Stop it with Ctrl+C.");
        let _ = std::io::stdout().flush();
    }
    for mut req in server.incoming_requests() {
        let mut body = String::new();
        let _ = std::io::Read::read_to_string(req.as_reader(), &mut body);
        let method = req.method().as_str().to_uppercase();
        let url = req.url().to_string();
        let (status, text) = answer(&method, &url, &body);
        let mut resp = tiny_http::Response::from_string(text).with_status_code(status);
        if status != 204 {
            if let Ok(h) = tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json; charset=utf-8"[..]) {
                resp.add_header(h);
            }
        }
        let _ = req.respond(resp);
    }
    Ok(())
}

pub fn serve(port: i64) {
    if let Err(e) = serve_with(port, &mut |m, p, b| web_handle(m, p, b)) {
        fail(e);
    }
}
