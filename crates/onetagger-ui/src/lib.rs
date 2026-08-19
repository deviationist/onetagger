#[macro_use] extern crate log;
#[macro_use] extern crate anyhow;
#[macro_use] extern crate include_dir;
#[macro_use] extern crate onetagger_shared;

use std::net::SocketAddr;
use std::time::Duration;
use anyhow::Error;
use axum::body::Body;
use axum::extract::{Query, WebSocketUpgrade, State, Request};
use axum::http::StatusCode;
use axum::http::header::{CONTENT_TYPE, ACCEPT_RANGES, CONTENT_RANGE, RANGE};
use axum::http::HeaderMap;
use axum::response::Response;
use std::sync::{Arc, Mutex};
use axum::response::IntoResponse;
use axum::Router;
use axum::routing::get;
use include_dir::Dir;
use onetagger_player::AudioSources;
use serde::{Serialize, Deserialize};
use quicktag::QuickTagFile;
use tokio::runtime::Builder;
use tokio::net::TcpListener;
use onetagger_shared::{PORT, WEBSERVER_CALLBACKS};

pub mod socket;
pub mod browser;
pub mod quicktag;
pub mod tageditor;

static CLIENT_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../../client/dist");

// Should have data from arguments and other flags (eg. port / host in future)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartContext {
    pub server_mode: bool,
    pub start_path: Option<String>,
    pub expose: bool,
    pub browser: bool,
}

fn start_async_runtime(context: StartContext) -> Result<(), Error> {
    let expose = context.expose;
    Builder::new_multi_thread().enable_all().build()?.block_on(async move {
        // Register routes
        let app = Router::new()
            .route("/thumb", get(get_thumb))
            .route("/audio", get(get_audio))
            .route("/ws", get(get_ws))
            .route("/spotify", get(get_spotify_callback))
            .route("/{*path}", get(get_static_file))
            .route("/", get(get_static_file))
            .with_state(context);

        // Start http server
        let host = match expose {
            true => format!("0.0.0.0:{PORT}"),
            false => format!("127.0.0.1:{PORT}")
        };
        info!("Starting web server on: http://{host}");
        let listener = TcpListener::bind(host).await?;
        axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>()).await?;

        Ok::<(), Error>(())
    })?;
    Ok(())
}

/// Serve assets file
async fn get_static_file(request: Request<Body>) -> impl IntoResponse {
    let mut path = request.uri().to_string();
    // Index HTML
    if path == "/" {
        path = "/index.html".to_string();
    }
    path = path[1..].to_string();

    // Static files
    if let Some(file) = CLIENT_DIR.get_file(&path) {
        let mime = mime_guess::from_path(&path).first().unwrap_or(mime::APPLICATION_OCTET_STREAM);
        return (StatusCode::OK, [(CONTENT_TYPE, mime.to_string())], file.contents().to_vec());
    }

    (StatusCode::NOT_FOUND, [(CONTENT_TYPE, "text/plain".to_string())], "Not found".as_bytes().to_vec())
}

#[derive(Debug, Clone, Deserialize)]
struct GetQueryPath {
    path: String
}

/// Serve thumbnail
async fn get_thumb(Query(GetQueryPath { path }): Query<GetQueryPath>) -> impl IntoResponse {
    match QuickTagFile::get_art(&path) {
        Ok(art) => (StatusCode::OK, [(CONTENT_TYPE, "image/jpeg".to_string())], art),
        Err(e) => {
            warn!("Error loading album art: {} File: {}", e, path);
            (StatusCode::NOT_FOUND, [(CONTENT_TYPE, "text/plain".to_string())], format!("Error loading album art: {} File: {}", e, path).into_bytes())
        }
    }
}

/// Most recently decoded WAV, keyed by source path.
///
/// `generate_wav` decodes the whole file into memory, and a browser seek is a
/// fresh HTTP Range request for the same path. Without this, every seek would
/// re-decode the entire file (a 64MB AIFF becomes a 67MB WAV), which is what
/// made seeking unusable. One entry is enough: the player only ever holds one
/// track open at a time.
static AUDIO_CACHE: Mutex<Option<(String, Arc<Vec<u8>>)>> = Mutex::new(None);

/// Parse a single-range `Range: bytes=..` header against a known total size.
///
/// Returns an inclusive `(start, end)`. Multi-range requests are not supported
/// (browsers do not use them for media), and anything unsatisfiable or
/// malformed returns `None` so the caller can fall back to a full response.
fn parse_range(header: &str, total: u64) -> Option<(u64, u64)> {
    let spec = header.trim().strip_prefix("bytes=")?;
    if spec.contains(',') || total == 0 {
        return None;
    }
    let (start, end) = spec.split_once('-')?;
    let (start, end) = match (start.trim(), end.trim()) {
        // "bytes=-N" -> the final N bytes
        ("", n) => (total.checked_sub(n.parse::<u64>().ok()?)?, total - 1),
        // "bytes=N-" -> from N to the end
        (n, "") => (n.parse().ok()?, total - 1),
        (a, b) => (a.parse().ok()?, b.parse::<u64>().ok()?.min(total - 1)),
    };
    (start <= end && start < total).then_some((start, end))
}

/// Serve audio, with Range support so the client-side player can seek.
async fn get_audio(headers: HeaderMap, Query(GetQueryPath { path }): Query<GetQueryPath>) -> Response {
    // Reuse the last decode if it is the same file
    let cached = AUDIO_CACHE.lock().ok()
        .and_then(|c| c.as_ref().filter(|(p, _)| *p == path).map(|(_, d)| d.clone()));

    let data = match cached {
        Some(data) => data,
        None => {
            let load_path = path.clone();
            let decoded = tokio::task::spawn_blocking(move || {
                match AudioSources::from_path(&load_path).map(|s| s.generate_wav()) {
                    Ok(Ok(wav)) => wav,
                    Ok(Err(e)) => {
                        warn!("Failed generating wav: {e}");
                        vec![]
                    },
                    Err(e) => {
                        warn!("Failed opening audio file {load_path}: {e}");
                        vec![]
                    }
                }
            }).await.unwrap_or_default();

            // Empty 404 on error
            if decoded.is_empty() {
                return (StatusCode::NOT_FOUND, [(CONTENT_TYPE, "text/plain")], Vec::new()).into_response();
            }
            let decoded = Arc::new(decoded);
            if let Ok(mut cache) = AUDIO_CACHE.lock() {
                *cache = Some((path.clone(), decoded.clone()));
            }
            decoded
        }
    };

    let total = data.len() as u64;
    let range = headers.get(RANGE).and_then(|v| v.to_str().ok())
        .and_then(|h| parse_range(h, total));

    match range {
        Some((start, end)) => {
            let body = data[start as usize..=end as usize].to_vec();
            (
                StatusCode::PARTIAL_CONTENT,
                [
                    (CONTENT_TYPE, "audio/wav".to_string()),
                    (ACCEPT_RANGES, "bytes".to_string()),
                    (CONTENT_RANGE, format!("bytes {start}-{end}/{total}")),
                ],
                body
            ).into_response()
        },
        None => (
            StatusCode::OK,
            [
                (CONTENT_TYPE, "audio/wav".to_string()),
                (ACCEPT_RANGES, "bytes".to_string()),
            ],
            data.as_ref().clone()
        ).into_response()
    }
}

/// WS connection
async fn get_ws(ws: WebSocketUpgrade, State(context): State<StartContext>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| {
        debug!("WS Connected!");
        async move {
            match socket::handle_ws_connection(socket, context).await {
                Ok(_) => {},
                Err(e) => warn!("WS connection error: {e}"),
            }
            debug!("WS Disconnected!");
        }
    })
}

/// Spotify token callback
async fn get_spotify_callback(request: Request<Body>) -> impl IntoResponse {
    info!("Got Spotify token from callback");
    WEBSERVER_CALLBACKS.lock().unwrap().insert("spotify".to_string(), request.uri().to_string());
    (StatusCode::OK, [(CONTENT_TYPE, "text/html")], include_str!("../../../assets/spotify_callback.html"))
}

// Start everything
pub fn start_all(context: StartContext) -> Result<(), Error> {
    if context.expose {
        warn!("Server is exposed to public!");
    }

    // Open in browser with 1s delay to allow the srever to load
    if context.browser {
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_secs(1));
            webbrowser::open(&format!("http://127.0.0.1:{PORT}")).ok();
        });
    }

    start_async_runtime(context.clone())?;
    Ok(())
}

