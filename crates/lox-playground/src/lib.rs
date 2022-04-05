use warp::Filter;

pub fn serve(port: u16) {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(playground(port));
}

async fn playground(port: u16) {
    // Statically include all files required by the playground.
    // Some of these need to be generated first using `cargo xtask codegen`.
    const INDEX_HTML: &str = include_str!("../res/index.html");
    const LOX_BG_WASM: &[u8] = include_bytes!("../../lox-wasm/pkg/lox_bg.wasm");
    const LOX_JS: &str = include_str!("../../lox-wasm/pkg/lox.js");
    const LOX_PNG: &[u8] = include_bytes!("../res/lox.png");
    const WORKER_JS: &str = include_str!("../res/worker.js");

    // Configure routes.
    let index_html = warp::path::end().map(|| {
        http::response::Builder::new().header("Content-Type", "text/html").body(INDEX_HTML)
    });
    let lox_bg_wasm = warp::path("lox_bg.wasm").map(|| {
        http::response::Builder::new().header("Content-Type", "application/wasm").body(LOX_BG_WASM)
    });
    let lox_js = warp::path("lox.js").map(|| {
        http::response::Builder::new().header("Content-Type", "text/javascript").body(LOX_JS)
    });
    let lox_png = warp::path("lox.png")
        .map(|| http::response::Builder::new().header("Content-Type", "image/png").body(LOX_PNG));
    let worker_js = warp::path("worker.js").map(|| {
        http::response::Builder::new().header("Content-Type", "text/javascript").body(WORKER_JS)
    });

    // Serve routes.
    let routes = warp::get().and(index_html.or(lox_bg_wasm).or(lox_js).or(lox_png).or(worker_js));
    let url = format!("http://127.0.0.1:{port}");
    eprintln!("Running playground on {url}");
    if let Err(e) = webbrowser::open(&url) {
        eprintln!("Failed to open browser: {e}");
    }
    warp::serve(routes).run(([127, 0, 0, 1], port)).await;
}
