//! `sapphire-tally`, the headless edition: the command line of
//! [`sapphire_tally_cli`] and nothing else. The desktop app's binary runs the
//! same commands.

use anyhow::Result;
use clap::Parser as _;
use sapphire_tally_cli::cli::Cli;

fn main() -> Result<()> {
    sapphire_tally_cli::run(Cli::parse().command)
}
