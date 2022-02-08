use lox::app::App;

use clap::Parser;

use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

fn main() {
    human_panic::setup_panic!();
    let app = App::parse();
    app.run();
}
