mod cmd;
mod repl;

use anyhow::Result;
use clap::Parser;
use mimalloc::MiMalloc;

use crate::cmd::Cmd;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

fn main() -> Result<()> {
    human_panic::setup_panic!();
    Cmd::parse().run()
}
