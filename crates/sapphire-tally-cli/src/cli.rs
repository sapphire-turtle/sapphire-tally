use clap::{Parser, Subcommand};

pub use sapphire_framework::server::FrameworkCommand;

/// The command line both editions run: the headless edition's `sapphire-tally`,
/// and the desktop app's binary when given a subcommand.
///
/// The framework's verbs sit at the top level, where the tally's own verbs
/// will join them. A bare invocation prints the help (the desktop app's opens its
/// window instead); the service manager starts the binary with `serve`.
#[derive(Parser)]
#[command(
    name = "sapphire-tally",
    arg_required_else_help = true,
    about = "Stamp tracker with GitHub-style heatmaps",
    long_about = None,
    version
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// The framework's verbs: serve, status, service, workspace, workgroup,
    /// device, embedding, external-device, mcp.
    #[command(flatten)]
    Framework(FrameworkCommand),
}
