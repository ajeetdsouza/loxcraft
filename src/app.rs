use clap::{AppSettings, Parser};

use std::path::PathBuf;

#[derive(Debug, Parser)]
#[clap(
    about,
    author,
    version,
    global_setting(AppSettings::DisableHelpSubcommand),
    global_setting(AppSettings::PropagateVersion)
)]
pub enum App {
    Repl,
    Run { path: PathBuf },
}
