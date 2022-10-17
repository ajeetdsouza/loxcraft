use std::path::Path;

use anyhow::{Context, Result};
use http::header::{CONTENT_ENCODING, CONTENT_TYPE};
use http::HeaderValue;
use rust_embed::RustEmbed;
use warp::path::Tail;
use warp::reply::Response;
use warp::{Filter, Rejection, Reply};

#[derive(RustEmbed)]
#[folder = "ui/dist/"]
#[exclude = "*.css"]
#[exclude = "*.js"]
#[exclude = "*.wasm"]
#[exclude = "*.woff"]
struct Asset;

pub fn serve(port: u16) -> Result<()> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("failed to start async runtime")?
        .block_on(serve_async(port));
    Ok(())
}

async fn serve_async(port: u16) {
    let routes =
        warp::path::end().and_then(serve_index).or(warp::path::tail().and_then(serve_asset));
    let url = format!("http://127.0.0.1:{port}");
    eprintln!("Running playground on {url}");
    if let Err(e) = webbrowser::open(&url) {
        eprintln!("Failed to open browser: {e}");
    }
    warp::serve(routes).run(([127, 0, 0, 1], port)).await;
}

async fn serve_index() -> Result<impl Reply, Rejection> {
    serve_impl("index.html")
}

async fn serve_asset(path: Tail) -> Result<impl Reply, Rejection> {
    serve_impl(path.as_str())
}

fn serve_impl(path: &str) -> Result<impl Reply, Rejection> {
    let compressed_br = [".css", ".js", ".wasm"].iter().any(|ext| path.ends_with(ext));
    let contents = if compressed_br { Asset::get(&format!("{path}.br")) } else { Asset::get(path) }
        .ok_or_else(warp::reject::not_found)?
        .data;

    let mut response = Response::new(contents.into());
    let headers = response.headers_mut();

    if let Some(mime) = guess_mime(path) {
        headers.insert(CONTENT_TYPE, HeaderValue::from_static(mime));
    }
    if compressed_br {
        headers.insert(CONTENT_ENCODING, HeaderValue::from_static("br"));
    }

    Ok(response)
}

fn guess_mime(path: &str) -> Option<&'static str> {
    match Path::new(path).extension()?.to_str()? {
        "html" => Some("text/html"),
        "js" => Some("text/javascript"),
        "png" => Some("image/png"),
        "wasm" => Some("application/wasm"),
        _ => None,
    }
}
