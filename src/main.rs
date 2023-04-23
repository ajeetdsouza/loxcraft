use anyhow::Result;
use clap::Parser;

use loxcraft::cmd::Cmd;

fn main() -> Result<()> {
    Cmd::parse().run()
}
