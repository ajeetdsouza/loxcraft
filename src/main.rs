mod cmd;

use anyhow::Result;
use clap::Parser;

use crate::cmd::Cmd;

fn main() -> Result<()> {
    Cmd::parse().run()
}
