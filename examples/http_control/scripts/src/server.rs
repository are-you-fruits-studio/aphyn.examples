// The HTTP side: a loopback server on its own thread that hands colors to the game through a
// one-slot mailbox. Systems run once per frame and must not block, so the socket cannot live in one.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use aphyn::*;
use tiny_http::{Header, Response, Server};

use crate::color;

pub const DEFAULT_PORT: u16 = 8787;
pub const PORT_VARIABLE: &str = "HTTP_CONTROL_PORT";

struct Inbox {
    // latest request wins: two colors set between frames would never both be visible anyway
    pending: Mutex<Option<[f32; 4]>>,
    // written by the game after painting, so `GET /color` reports what is on screen
    applied: Mutex<[f32; 4]>,
    accepted: AtomicU64,
}

// A `Mutex` mailbox rather than `mpsc`: a resource must be `Sync`, and `mpsc::Receiver` is not.
#[derive(Resource)]
pub struct HttpControlResource {
    inbox: Arc<Inbox>,
    pub address: String,
}

impl HttpControlResource {
    // Only 127.0.0.1: the server has no authentication and is meant for tools on this machine.
    pub fn start(initial: [f32; 4]) -> Result<Self, String> {
        let port = match std::env::var(PORT_VARIABLE) {
            Ok(value) => value.parse().unwrap_or_else(|_| {
                eprintln!("http_control: {PORT_VARIABLE}={value:?} is not a port number, using {DEFAULT_PORT}");
                DEFAULT_PORT
            }),
            Err(_) => DEFAULT_PORT,
        };

        let server = Server::http(("127.0.0.1", port)).map_err(|e| format!("could not bind 127.0.0.1:{port}: {e}"))?;
        let address = server
            .server_addr()
            .to_ip()
            .map_or_else(|| format!("127.0.0.1:{port}"), |a| a.to_string());

        let inbox = Arc::new(Inbox {
            pending: Mutex::new(None),
            applied: Mutex::new(initial),
            accepted: AtomicU64::new(0),
        });

        let thread_inbox = Arc::clone(&inbox);
        let thread_address = address.clone();
        // never joined: the thread blocks on the socket until the process exits
        std::thread::Builder::new()
            .name("http_control".to_string())
            .spawn(move || serve(server, thread_inbox, thread_address))
            .map_err(|e| format!("could not spawn the server thread: {e}"))?;

        Ok(Self { inbox, address })
    }

    pub fn take_pending(&self) -> Option<[f32; 4]> {
        self.inbox.pending.lock().unwrap().take()
    }

    pub fn set_applied(&self, color: [f32; 4]) {
        *self.inbox.applied.lock().unwrap() = color;
    }

    pub fn accepted_count(&self) -> u64 {
        self.inbox.accepted.load(Ordering::Relaxed)
    }
}

fn serve(server: Server, inbox: Arc<Inbox>, address: String) {
    for request in server.incoming_requests() {
        // everything is in the path; a `?query` (e.g. a cache buster) is ignored
        let url = request.url().to_string();
        let path = url.split('?').next().unwrap_or("/");

        let (status, body) = route(&inbox, &address, path);

        let content_type = Header::from_bytes(&b"Content-Type"[..], &b"text/plain; charset=utf-8"[..]).unwrap();
        let response = Response::from_string(body).with_status_code(status).with_header(content_type);

        if let Err(e) = request.respond(response) {
            println!("[http_control] failed to respond to {path}: {e}");
        }
    }
}

// The method is not checked: the whole command is in the path.
fn route(inbox: &Inbox, address: &str, path: &str) -> (u16, String) {
    match path {
        "/" => (200, usage(address)),
        "/health" => (200, "ok\n".to_string()),
        "/color" => {
            let applied = *inbox.applied.lock().unwrap();
            (200, format!("{}\n", color::to_hex(applied)))
        }
        _ => match path.strip_prefix("/color/") {
            // a literal `#` cannot be sent in a URL path, so it arrives as `%23`
            Some(raw) => match percent_decode(raw).as_deref().and_then(color::parse) {
                Some(rgba) => {
                    *inbox.pending.lock().unwrap() = Some(rgba);
                    let n = inbox.accepted.fetch_add(1, Ordering::Relaxed) + 1;
                    println!("[http_control] #{n} accepted /color/{raw}");
                    // 202: the color becomes visible on the next frame, not yet
                    (202, format!("{}\n", color::to_hex(rgba)))
                }
                None => (400, format!("bad color code: {raw:?}\n\n{}", usage(address))),
            },
            None => (404, format!("no such endpoint: {path:?}\n\n{}", usage(address))),
        },
    }
}

// `None` for a truncated (`%2`) or non-hex (`%zz`) escape, or for bytes that are not UTF-8.
// `+` stays a literal `+`: this is a path segment, not a form.
fn percent_decode(segment: &str) -> Option<String> {
    let bytes = segment.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = segment.get(i + 1..i + 3)?;
            // `from_str_radix` alone would also accept a sign, as in `%+1`
            if !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
                return None;
            }
            out.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }

    String::from_utf8(out).ok()
}

fn usage(address: &str) -> String {
    format!(
        concat!(
            "http_control: the window color is set over HTTP.\n",
            "\n",
            "  GET /color/<code>   set the window color (202)\n",
            "  GET /color          read back the color on screen, as rrggbbaa\n",
            "  GET /health         liveness check\n",
            "\n",
            "<code> is rgb, rrggbb or rrggbbaa in hex (a leading '#' is optional) or a name:\n",
            "  black white red green blue yellow cyan magenta orange gray\n",
            "\n",
            "A literal '#' cannot appear in a URL path; percent-encode it as %23.\n",
            "\n",
            "  curl http://{address}/color/ff0000\n",
            "  curl http://{address}/color/%23336699\n",
        ),
        address = address
    )
}
