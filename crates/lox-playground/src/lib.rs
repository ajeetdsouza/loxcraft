use http::header::CONTENT_TYPE;
use http::HeaderValue;
use include_dir::{include_dir, Dir};
use warp::reply::Response;
use warp::{path::Tail, Filter, Rejection, Reply};

use std::path::Path;

pub fn serve(port: u16) {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(serve_async(port));
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
    static ASSETS: Dir = include_dir!("$CARGO_MANIFEST_DIR/ui/dist");
    let contents = ASSETS.get_file(path).ok_or_else(warp::reject::not_found)?.contents();
    let mut response = Response::new(contents.into());
    if let Some(mime) = guess_mime(path) {
        response.headers_mut().insert(CONTENT_TYPE, HeaderValue::from_static(mime));
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
