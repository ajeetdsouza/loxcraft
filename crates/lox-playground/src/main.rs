use tokio::runtime;
use warp::Filter;

fn main() {
    let _ = runtime::Builder::new_multi_thread().enable_all().build().unwrap().block_on(serve());
}

async fn serve() {
    // GET /
    const INDEX: &str = include_str!("../res/index.html");
    let index = warp::path::end().map(|| warp::reply::html(INDEX));

    // GET /favicon.png
    const FAVICON: &[u8] = include_bytes!("../res/favicon.png");
    let favicon = warp::path("favicon.png").map(|| warp::reply::Response::new(FAVICON.into()));

    let routes = warp::get().and(index.or(favicon));
    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}
