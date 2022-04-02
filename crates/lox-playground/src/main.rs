use warp::Filter;

fn main() {
    tokio::runtime::Builder::new_multi_thread().enable_all().build().unwrap().block_on(serve());
}

async fn serve() {
    // GET /
    const INDEX: &str = include_str!("../res/index.html");
    let index = warp::path::end().map(|| warp::reply::html(INDEX));

    // GET /favicon.png
    const FAVICON: &[u8] = include_bytes!("../res/favicon.png");
    let favicon = warp::path("favicon.png").map(|| warp::reply::Response::new(FAVICON.into()));

    // GET /lox.wasm
    const LOX_JS: &[u8] = include_bytes!("../res/lox.js");
    let lox_js = warp::path("lox.js").map(|| {
        http::response::Builder::new().header("Content-Type", "text/javascript").body(LOX_JS)
    });

    // GET /lox.wasm
    const LOX_WASM: &[u8] = include_bytes!("../res/lox.wasm");
    let lox_wasm = warp::path("lox.wasm").map(|| {
        http::response::Builder::new().header("Content-Type", "application/wasm").body(LOX_WASM)
    });

    let routes = warp::get().and(index.or(favicon).or(lox_js).or(lox_wasm));
    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}
