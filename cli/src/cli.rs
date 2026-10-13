use clap::{Parser, Subcommand};

pub use sapphire_framework::server::FrameworkCommand;

/// The command line of the one binary this app ships.
///
/// The framework's verbs sit at the top level, where the tally's own verbs
/// will join them. A bare invocation parses to `command: None`, which is the
/// framework's default verb, `serve` — what the service manager starts.
#[derive(Parser)]
#[command(
    name = "sapphire-tally",
    about = "Stamp tracker with GitHub-style heatmaps",
    long_about = None,
    version
)]
pub struct Cli {
    /// Omit to run the server.
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand)]
pub enum Command {
    /// The framework's verbs: serve, status, service, workspace, workgroup,
    /// device, embedding, external-device, mcp.
    #[command(flatten)]
    Framework(FrameworkCommand),
}
