use anyhow::Result;
use clap::Parser as _;
use sapphire_tally::cli::{Cli, Command, FrameworkCommand};
use sapphire_tally::serve;
use sapphire_tally_core::AppKind;

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        // A bare invocation *is* `serve`: what the service manager starts.
        None | Some(Command::Framework(FrameworkCommand::Serve)) => {
            sapphire_tally_core::init_app_context(AppKind::Server);
            runtime()?.block_on(serve::run())
        }
        // `mcp` among them: its stdout is the MCP stream, and the
        // framework's console log goes to stderr (sapphire-framework#232).
        Some(Command::Framework(command)) => {
            sapphire_tally_core::init_app_context(AppKind::Cli);
            let code =
                runtime()?.block_on(command.dispatch(serve::build_oneshot(), serve::VERSION))?;
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
