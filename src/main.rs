mod builtins;
mod cli;
mod parser;
mod runtime;
mod vfs;

use anyhow::Result;
use clap::Parser as ClapParser;
use cli::NashCli;

fn main() -> Result<()> {
    let cli = NashCli::parse();
    cli.run()
}
