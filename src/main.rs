mod cmd;
mod repl;

use anyhow::Result;
use clap::Parser;

use crate::cmd::Cmd;

fn main() -> Result<()> {
    human_panic::setup_panic!();
    Cmd::parse().run()
}
