//! sapphire-tally's desktop app: the heatmap and its stamps on the
//! framework's shell, with its Settings — and, given a subcommand, the CLI.
//!
//! This one binary is a superset of the CLI's: bare, it opens the window;
//! with `serve`, `mcp`, `status`, `workspace`, … it runs the CLI's own code
//! (`sapphire_tally_cli::run`), so the service manager and MCP clients can
//! start it as they start `sapphire-tally`.
//!
//! The window is a pure client, like sapphire-ledger's: it talks to the
//! service-managed `sapphire-tally serve` and `sapphire-bridge` over IPC and
//! never starts either itself. The Tally screen calls the server's `sapphire-tally.*`
//! methods, so it shows the workspace the server has selected; Settings, at
//! the rail's foot, picks that workspace, manages its API keys and opens the
//! Sapphire Bridge window. Closing the window stops nothing.
//!
//! With the `package` feature (the release workflow's build) it is a cargo-packager package
//! (src/package.rs): it updates itself, and its first start asks to keep the app server
//! running.

// In the package, a GUI binary on Windows: no console window behind the window or the
// service. The command line goes through the console shim (src/bin/console.rs).
#![cfg_attr(all(windows, feature = "package"), windows_subsystem = "windows")]

mod client;
mod form;
mod heatmap;
#[cfg(feature = "package")]
mod package;
mod tally;

use clap::Parser;
use corundum_framework_platform::launcher::LauncherSpec;
use sapphire_framework::gui::app::{AppOptions, AppSetup};
use sapphire_framework::gui::client::AppIdentity;
use sapphire_framework::gui::{Destination, Settings, Shell, ShellApp, Side, eframe, egui, i18n};
use sapphire_framework::server::FrameworkCommand;
use sapphire_tally_cli::cli::Command;

use crate::client::TallyClient;
use crate::tally::TallyView;

/// Sapphire Tally in the OS's app menu, and the window's id and icon. As the package's
/// (`release-binaries.yml`): `org.sapphire-turtle.sapphire-tally` is its macOS bundle id and
/// `Utility` its Linux category. Not the packId (`sapphire-tally-desktop`), as corundum's demo.
const LAUNCHER: LauncherSpec = LauncherSpec {
    id: "sapphire-tally",
    name: "Sapphire Tally",
    description: "Stamp tracker with heatmaps",
    category: "Utility",
    icon_svg: include_bytes!("../../docs/icon.svg"),
    bundle_prefix: "org.sapphire-turtle",
};

const APP: AppIdentity = AppIdentity {
    app_name: "sapphire-tally",
    version: env!("CARGO_PKG_VERSION"),
};

/// The tally's own strings: the rail's labels are message ids, looked up in
/// these after the framework's.
const RESOURCES: i18n::Resources = i18n::Resources {
    name: "sapphire-tally-desktop",
    files: &[("en", include_str!("../locales/en/tally.ftl"))],
};

/// The app's own screens, on the framework's rail.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Screen {
    Tally,
}

struct App {
    shell: Shell<Screen>,
    tally: TallyView,
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.shell.show(ui, &mut self.tally);
    }
}

impl ShellApp<Screen> for TallyView {
    fn screen_ui(&mut self, ui: &mut egui::Ui, screen: Screen, side: &mut Side) {
        match screen {
            Screen::Tally => self.ui(ui, side),
        }
    }

    /// The stamps of what is picked on the Tally screen, Settings or not.
    fn side_ui(&mut self, ui: &mut egui::Ui, _: Option<Screen>, side: &mut Side) {
        TallyView::side_ui(self, ui, side);
    }
}

/// The CLI's command line, with the subcommand optional: none opens the window.
#[derive(Parser)]
#[command(
    name = "sapphire-tally",
    about = "Stamp tracker with GitHub-style heatmaps: the desktop app, or the CLI's commands",
    version
)]
struct DesktopCli {
    #[command(subcommand)]
    command: Option<Command>,
}

/// Whether this start goes through the package's start: every start but `serve`, the
/// package's service (the start starts the services an update stopped, and that is this
/// process). The installer's hooks (`--corundum-package <hook>`) pass arguments clap
/// rejects, so this is decided before clap may exit on them.
#[cfg_attr(not(feature = "package"), allow(dead_code))]
fn runs_package_start(parsed: &Result<DesktopCli, clap::Error>) -> bool {
    !matches!(
        parsed,
        Ok(DesktopCli {
            command: Some(Command::Framework(FrameworkCommand::Serve)),
        })
    )
}

fn main() -> anyhow::Result<()> {
    let parsed = DesktopCli::try_parse();
    // The package's start: an install hook runs and exits here. The window's console
    // first, so the hooks log; a command installs its own.
    #[cfg(feature = "package")]
    if runs_package_start(&parsed) {
        if !matches!(parsed, Ok(DesktopCli { command: Some(_) })) {
            sapphire_framework::gui::app::install_logging(APP);
        }
        let _ = sapphire_framework::gui::package::runtime::start(&package::PACKAGE);
    }
    let cli = parsed.unwrap_or_else(|e| e.exit());
    match cli.command {
        // In the package, the package's updater replaces it, not the desktop release file.
        #[cfg(feature = "package")]
        Some(Command::Framework(FrameworkCommand::Update(args)))
            if corundum_framework_platform::package::current().is_some() =>
        {
            std::process::exit(package_update(&args))
        }
        Some(command) => sapphire_tally_cli::run(command, sapphire_tally_cli::Edition::Desktop),
        None => Ok(window()?),
    }
}

/// `update` in the package: `sapphire-bridge` first (unless `--no-bridge`), as the package's
/// own update ends this process, then the package through its updater. The exit code.
#[cfg(feature = "package")]
fn package_update(args: &sapphire_framework::server::AppUpdateArgs) -> i32 {
    let bridge = if args.no_bridge {
        0
    } else {
        match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(runtime) => runtime
                .block_on(sapphire_framework::server::update_bridge(&args.update))
                .unwrap_or_else(|err| {
                    eprintln!("sapphire-bridge: {err}");
                    1
                }),
            Err(err) => {
                eprintln!("sapphire-tally: {err}");
                1
            }
        }
    };
    let own = match package::APP_PACKAGE.updater() {
        Some(updater) => package::APP_PACKAGE.update(
            &updater,
            args.update.check,
            &mut std::io::stdout(),
            std::time::Duration::from_secs(5 * 60),
        ),
        None => 1,
    };
    own.max(bridge)
}

fn window() -> eframe::Result<()> {
    sapphire_framework::gui::app::run(app_options(), |setup: AppSetup<'_>| {
        let settings = Settings::new(setup.framework);
        // About's service row and updates, in the package.
        #[cfg(feature = "package")]
        let settings = settings.with_package_about(setup.package_about);
        i18n::register(&setup.cc.egui_ctx, RESOURCES);
        let tally_client = TallyClient::new(APP.app_name, APP.version, setup.endpoints.app);
        let shell = Shell::new(
            vec![Destination::new(
                Screen::Tally,
                egui::include_image!("../assets/icons/calendar-check.svg"),
                "tally-screen",
            )],
            settings,
        );
        Box::new(App {
            shell,
            tally: TallyView::new(setup.runtime, tally_client, setup.repaint),
        })
    })
}

/// What the window opens with: the package's icon and id, and in the package, the package.
fn app_options() -> AppOptions {
    let options = AppOptions::new(APP, "Sapphire Tally")
        .inner_size([1100.0, 640.0])
        .min_inner_size([720.0, 440.0])
        .launcher(LAUNCHER);
    #[cfg(feature = "package")]
    let options = options.package(package::APP_PACKAGE);
    options
}

#[cfg(test)]
mod tests {
    use super::*;

    // The installer's hooks start the binary with arguments clap does not know; the
    // package's start must see them before clap exits on them.
    #[test]
    fn package_start_runs_before_clap_rejects() {
        let parsed = |args: &[&str]| DesktopCli::try_parse_from(args);
        assert!(runs_package_start(&parsed(&[
            "x",
            "--corundum-package",
            "installed"
        ])));
        assert!(runs_package_start(&parsed(&["x", "--version"])));
        assert!(runs_package_start(&parsed(&["x"])));
        assert!(runs_package_start(&parsed(&["x", "status"])));
        assert!(!runs_package_start(&parsed(&["x", "serve"])));
    }

    // The window shows the package's icon even outside a package, and goes by the
    // package's bundle id, so the OS groups it with the installer's entry.
    #[test]
    fn the_window_has_the_packages_icon_and_id() {
        assert_eq!(LAUNCHER.id, "sapphire-tally");
        assert_eq!(LAUNCHER.name, "Sapphire Tally");
        assert_eq!(LAUNCHER.category, "Utility");
        assert_eq!(LAUNCHER.bundle_prefix, "org.sapphire-turtle");
        assert_eq!(
            LAUNCHER.icon_svg,
            include_bytes!("../../docs/icon.svg").as_slice()
        );
        assert!(corundum_framework_platform::icon::render_rgba(LAUNCHER.icon_svg, 64).is_ok());
    }
}
