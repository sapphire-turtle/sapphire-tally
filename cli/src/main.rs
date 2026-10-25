use anyhow::Result;
use clap::Parser as _;
use sapphire_tally::cli::{Cli, Command, FrameworkCommand};
use sapphire_tally::serve::{self, TallySettings};
use sapphire_tally_core::{AppKind, TALLY_CTX};

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        // `serve` is what the service manager starts; the rest are clients
        // of it — `mcp` among them: its stdout is the MCP stream, and the
        // framework's console log goes to stderr (sapphire-framework#232).
        Command::Framework(command) => {
            let kind = match command {
                FrameworkCommand::Serve => AppKind::Server,
                _ => AppKind::Cli,
            };
            sapphire_tally_core::init_app_context(kind);
            let code = runtime()?.block_on(command.dispatch(
                &TALLY_CTX,
                TallySettings,
                serve::build_with,
                serve::VERSION,
            ))?;
            if code != 0 {
                std::process::exit(code);
            }
            Ok(())
        }
    }
}

fn runtime() -> Result<tokio::runtime::Runtime> {
    Ok(tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?)
}
