mod cmd;
mod repl;

use crate::cmd::Cmd;

use anyhow::Result;
use clap::Parser;
use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

fn main() -> Result<()> {
    human_panic::setup_panic!();
    Cmd::parse().run()
}
