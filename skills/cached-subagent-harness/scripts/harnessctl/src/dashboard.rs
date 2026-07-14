use crate::domain::Language;
use crate::status::{build_status, render_json};
use crate::store::Store;
use std::net::{IpAddr, SocketAddr};
use std::path::Path;
use tiny_http::{Header, Method, Request, Response, Server, StatusCode};

const INDEX_HTML: &str = include_str!("../assets/index.html");
const STYLES_CSS: &str = include_str!("../assets/styles.css");
const APP_JS: &str = include_str!("../assets/app.js");

#[derive(Debug, Clone, Copy)]
pub(crate) struct DashboardOptions {
    pub bind: IpAddr,
    pub port: u16,
    pub language: Language,
    pub allow_remote: bool,
}

pub(crate) fn serve(
    store_path: &Path,
    run_id: &str,
    options: DashboardOptions,
) -> Result<SocketAddr, String> {
    if !options.bind.is_loopback() && !options.allow_remote {
        return Err("non-loopback dashboard bind requires --allow-remote true".into());
    }
    let server = Server::http(SocketAddr::new(options.bind, options.port))
        .map_err(|error| error.to_string())?;
    let address = server
        .server_addr()
        .to_ip()
        .ok_or_else(|| "dashboard did not bind an IP socket".to_string())?;
    let store_path = store_path.to_path_buf();
    let run_id = run_id.to_string();
    std::thread::Builder::new()
        .name("harness-dashboard".into())
        .spawn(move || {
            for request in server.incoming_requests() {
                handle_request(request, &store_path, &run_id, options.language);
            }
        })
        .map_err(|error| error.to_string())?;
    Ok(address)
}

fn handle_request(request: Request, store_path: &Path, run_id: &str, language: Language) {
    if request.method() != &Method::Get {
        respond(
            request,
            405,
            "method not allowed".into(),
            "text/plain; charset=utf-8",
            true,
        );
        return;
    }
    match request.url() {
        "/" => respond(
            request,
            200,
            INDEX_HTML.replace("{{LANG}}", language.as_str()),
            "text/html; charset=utf-8",
            false,
        ),
        "/assets/styles.css" => respond(
            request,
            200,
            STYLES_CSS.into(),
            "text/css; charset=utf-8",
            false,
        ),
        "/assets/app.js" => respond(
            request,
            200,
            APP_JS.into(),
            "text/javascript; charset=utf-8",
            false,
        ),
        "/health" => respond(
            request,
            200,
            "{\"status\":\"ok\"}".into(),
            "application/json; charset=utf-8",
            true,
        ),
        "/api/status" => {
            let result = Store::open(store_path)
                .and_then(|store| build_status(&store, run_id))
                .and_then(|view| render_json(&view));
            match result {
                Ok(json) => respond(request, 200, json, "application/json; charset=utf-8", true),
                Err(error) => respond(
                    request,
                    500,
                    serde_json::json!({ "error": error }).to_string(),
                    "application/json; charset=utf-8",
                    true,
                ),
            }
        }
        _ => respond(
            request,
            404,
            "not found".into(),
            "text/plain; charset=utf-8",
            true,
        ),
    }
}

fn respond(request: Request, status: u16, body: String, content_type: &str, no_store: bool) {
    let mut response = Response::from_string(body)
        .with_status_code(StatusCode(status))
        .with_header(header("Content-Type", content_type))
        .with_header(header(
            "Content-Security-Policy",
            "default-src 'self'; object-src 'none'; frame-ancestors 'none'",
        ))
        .with_header(header("X-Content-Type-Options", "nosniff"))
        .with_header(header("Referrer-Policy", "no-referrer"));
    if no_store {
        response.add_header(header("Cache-Control", "no-store"));
    }
    let _ = request.respond(response);
}

fn header(name: &str, value: &str) -> Header {
    Header::from_bytes(name.as_bytes(), value.as_bytes()).expect("static HTTP header is valid")
}

#[cfg(test)]
mod tests {
    use super::{DashboardOptions, serve};
    use crate::domain::Language;
    use crate::store::Store;
    use std::fs;
    use std::io::{Read, Write};
    use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream};
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    fn get(address: SocketAddr, path: &str) -> String {
        let mut stream = TcpStream::connect_timeout(&address, Duration::from_secs(2)).unwrap();
        write!(
            stream,
            "GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n"
        )
        .unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        response
    }

    #[test]
    fn dashboard_serves_embedded_assets_status_and_security_headers() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("harness-dashboard-{nonce}.db"));
        let mut store = Store::open(&path).unwrap();
        store
            .create_run("run-1", "visible work", "/repo", "/report")
            .unwrap();
        drop(store);
        let address = serve(
            &path,
            "run-1",
            DashboardOptions {
                bind: IpAddr::V4(Ipv4Addr::LOCALHOST),
                port: 0,
                language: Language::ZhCn,
                allow_remote: false,
            },
        )
        .unwrap();
        let html = get(address, "/");
        assert!(html.starts_with("HTTP/1.1 200"));
        assert!(html.contains("Content-Security-Policy: default-src 'self'"));
        assert!(html.contains("data-panel=\"tasks\""));
        assert!(get(address, "/assets/styles.css").contains("--moonlight"));
        let app = get(address, "/assets/app.js");
        assert!(app.contains("textContent"));
        assert!(app.contains("current_task_id"));
        assert!(app.contains("actual_model"));
        assert!(app.contains("estimate_sample_count"));
        let api = get(address, "/api/status");
        assert!(api.contains("Cache-Control: no-store"));
        assert!(api.contains("\"total_effective\": null"));
        assert!(get(address, "/health").contains("{\"status\":\"ok\"}"));
        assert!(get(address, "/missing").starts_with("HTTP/1.1 404"));
        let _ = fs::remove_file(&path);
        let _ = fs::remove_file(path.with_extension("db-shm"));
        let _ = fs::remove_file(path.with_extension("db-wal"));
    }

    #[test]
    fn remote_bind_requires_explicit_permission() {
        let error = serve(
            std::path::Path::new("/tmp/not-opened.db"),
            "run-1",
            DashboardOptions {
                bind: IpAddr::V4(Ipv4Addr::UNSPECIFIED),
                port: 0,
                language: Language::EnUs,
                allow_remote: false,
            },
        )
        .unwrap_err();
        assert!(error.contains("allow-remote"));
    }
}
