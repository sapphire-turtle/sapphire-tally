//! sapphire-tally's desktop app: the heatmap and its stamps on the
//! framework's shell, with its Settings.
//!
//! A pure client, like sapphire-ledger's: it talks to the service-managed
//! `sapphire-tally serve` and `sapphire-bridge` over IPC and never starts
//! either itself. The Tally screen calls the server's `sapphire-tally.*`
//! methods, so it shows the workspace the server has selected; Settings, at
//! the rail's foot, picks that workspace and manages the workgroup. Closing
//! the window stops nothing.

mod client;
mod form;
mod heatmap;
mod tally;

use sapphire_framework::gui::app::{AppOptions, AppSetup};
use sapphire_framework::gui::client::AppIdentity;
use sapphire_framework::gui::{Destination, Settings, Shell, eframe, egui};

use crate::client::TallyClient;
use crate::tally::TallyView;

const APP: AppIdentity = AppIdentity {
    app_name: "sapphire-tally",
    version: env!("CARGO_PKG_VERSION"),
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
        let tally = &mut self.tally;
        self.shell.ui(ui, |ui, screen| match screen {
            Screen::Tally => {
                egui::CentralPanel::default().show(ui, |ui| tally.ui(ui));
            }
        });
    }
}

fn main() -> eframe::Result<()> {
    sapphire_framework::gui::app::run(
        AppOptions::new(APP, "Sapphire Tally")
            .inner_size([1100.0, 640.0])
            .min_inner_size([720.0, 440.0]),
        |setup: AppSetup<'_>| {
            let tally_client = TallyClient::new(APP.app_name, APP.version, setup.endpoints.app);
            let shell = Shell::new(
                vec![Destination::new(
                    Screen::Tally,
                    egui::include_image!("../assets/icons/calendar-check.svg"),
                    "Tally",
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
