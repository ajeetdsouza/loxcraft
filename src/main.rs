use lox::cmd::Cmd;

use clap::Parser;

use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

fn main() {
    human_panic::setup_panic!();
    let cmd = Cmd::parse();
    cmd.run();
}
