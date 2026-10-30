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
//! the rail's foot, picks that workspace and manages the workgroup. Closing
//! the window stops nothing.

mod client;
mod form;
mod heatmap;
mod tally;

use clap::Parser;
use sapphire_framework::gui::app::{AppOptions, AppSetup};
use sapphire_framework::gui::client::AppIdentity;
use sapphire_framework::gui::{Destination, Settings, Shell, ShellApp, Side, eframe, egui, i18n};
use sapphire_tally_cli::cli::Command;

use crate::client::TallyClient;
use crate::tally::TallyView;

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

fn main() -> anyhow::Result<()> {
    match DesktopCli::parse().command {
        Some(command) => sapphire_tally_cli::run(command, sapphire_tally_cli::Edition::Desktop),
        None => Ok(window()?),
    }
}

fn window() -> eframe::Result<()> {
    sapphire_framework::gui::app::run(
        AppOptions::new(APP, "Sapphire Tally")
            .inner_size([1100.0, 640.0])
            .min_inner_size([720.0, 440.0]),
        |setup: AppSetup<'_>| {
            i18n::register(&setup.cc.egui_ctx, RESOURCES);
            let tally_client = TallyClient::new(APP.app_name, APP.version, setup.endpoints.app);
            let shell = Shell::new(
                vec![Destination::new(
                    Screen::Tally,
                    egui::include_image!("../assets/icons/calendar-check.svg"),
                    "tally-screen",
                )],
                Settings::new(setup.framework),
            );
            Box::new(App {
                shell,
                tally: TallyView::new(setup.runtime, tally_client, setup.repaint),
            })
        },
    )
}
