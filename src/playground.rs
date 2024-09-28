#![cfg(feature = "playground")]

use std::net::{Ipv4Addr, SocketAddrV4};

use anyhow::{Context as _, Result};
use rust_embed::Embed;

#[derive(Debug, Embed)]
#[folder = "playground/out/"]
struct Asset;

pub fn serve(port: u16) -> Result<()> {
    let url = format!("http://127.0.0.1:{port}");

    let ip_address = Ipv4Addr::new(127, 0, 0, 1);
    let socket_address = SocketAddrV4::new(ip_address, port);

    let serve = warp_embed::embed(&Asset);
    let server = warp::serve(serve).run(socket_address);

    eprintln!("Running playground on {url}");
    if let Err(e) = webbrowser::open(&url) {
        eprintln!("Failed to open browser: {e}");
    }

    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("failed to start async runtime")?
        .block_on(server);
    Ok(())
}
