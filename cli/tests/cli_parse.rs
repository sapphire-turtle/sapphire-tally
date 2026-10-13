//! The one binary's command line: the framework's verbs, flat at the top
//! level.

use clap::Parser as _;
use sapphire_tally::cli::{Cli, Command, FrameworkCommand};

fn parse(args: &[&str]) -> Cli {
    Cli::try_parse_from(std::iter::once("sapphire-tally").chain(args.iter().copied()))
        .unwrap_or_else(|e| panic!("{args:?} should parse: {e}"))
}

#[test]
fn a_bare_invocation_is_the_server() {
    // `None` is what `main` runs as `serve`; the service manager starts the
    // binary with `serve`, which must land in the same place.
    assert!(parse(&[]).command.is_none());
    assert!(matches!(
        parse(&["serve"]).command,
        Some(Command::Framework(FrameworkCommand::Serve))
    ));
}

#[test]
fn the_framework_verbs_sit_at_the_top_level() {
    for args in [
        &["status"][..],
        &["service", "status"],
        &["workspace", "show"],
        &["workspace", "init", "/tmp/tally", "--sync"],
        &["workspace", "select", "/tmp/tally", "--sync"],
        &["workspace", "sync", "enable"],
        &["workspace", "sync", "disable"],
        &["workspace", "sync", "status"],
        &["workgroup", "list"],
        &["device", "list"],
        &["external-device", "list"],
        &["mcp"],
    ] {
        assert!(
            matches!(parse(args).command, Some(Command::Framework(_))),
            "{args:?} should be a framework verb"
        );
    }
}
