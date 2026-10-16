//! sapphire-tally's desktop app: the heatmap and its stamps, and the
//! framework's Settings.
//!
//! A pure client, like sapphire-ledger's: it talks to the service-managed
//! `sapphire-tally serve` and `sapphire-bridge` over IPC and never starts
//! either itself. The Tally tab calls the server's `sapphire-tally.*`
//! methods, so it shows the workspace the server has selected; the Sync tab
//! picks that workspace and manages the workgroup. Closing the window stops
//! nothing.

mod client;
mod form;
mod heatmap;
mod tally;

use sapphire_framework::gui::app::{AppOptions, AppSetup};
use sapphire_framework::gui::client::AppIdentity;
use sapphire_framework::gui::{Settings, eframe, egui};

use crate::client::TallyClient;
use crate::tally::TallyView;

const APP: AppIdentity = AppIdentity {
    app_name: "sapphire-tally",
    version: env!("CARGO_PKG_VERSION"),
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Tally,
    Sync,
}

struct App {
    tab: Tab,
    tally: TallyView,
    sync: Settings,
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::Panel::top("tabs").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.tab, Tab::Tally, "Tally");
                ui.selectable_value(&mut self.tab, Tab::Sync, "Sync");
            });
        });
        egui::CentralPanel::default().show(ui, |ui| match self.tab {
            Tab::Tally => self.tally.ui(ui),
            Tab::Sync => self.sync.ui(ui),
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
            Box::new(App {
                tab: Tab::Tally,
                tally: TallyView::new(setup.runtime, tally_client, setup.repaint),
                sync: Settings::new(setup.framework),
            })
        },
    )
}
