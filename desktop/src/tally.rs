//! The Tally tab: every activity with a year of its stamps as a heatmap, one
//! under the other, the stamps of a day picked in one of them, and the windows
//! that show, add, change and delete a stamp or an activity.
//!
//! Everything comes from the running server's `sapphire-tally.*` methods
//! ([`TallyClient`]), so the tab shows the workspace the server has selected.
//! Calls run on the tokio runtime and come back through a channel; the view
//! itself never blocks.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use chrono::{Datelike as _, Days, Local, NaiveDate};
use grain_id::GrainId;
use sapphire_framework::gui::egui;
use sapphire_tally_core::{Activity, Stamp};
use sapphire_tally_mcp::methods::{
    ActivityDeleteParams, ActivityDeleted, EmptyParams, IdParams, QueryParams, StampAddParams,
    names,
};

use crate::client::TallyClient;
use crate::form::{self, ActivityForm, StampForm};
use crate::heatmap::{self, Grid};

/// How often the tab re-reads the data on its own, to pick up what the CLI,
/// an agent or sync wrote.
const REFRESH_EVERY: Duration = Duration::from_secs(10);

/// The heatmap's cell and the gap between cells, in points.
const CELL: f32 = 12.0;
const GAP: f32 = 3.0;
/// Room for the weekday labels left of the grid, and the month labels above.
const LABEL_W: f32 = 30.0;
const LABEL_H: f32 = 16.0;

/// What came back from the server.
enum Reply {
    /// One refresh: every activity, and the stamps in the grid's range.
    Data(Grid, Result<Data, String>),
    /// A write went through; the text is what the server answered with.
    Written(Sent, Result<String, String>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Sent {
    Stamp,
    Activity,
    /// The one-click "Stamp now", which has no window to close.
    QuickStamp,
    DeleteStamp,
    DeleteActivity,
}

struct Data {
    activities: Vec<Activity>,
    stamps: Vec<Stamp>,
}

/// A window showing one record, or a form for a new one.
struct Window<F> {
    /// `None` for a new record.
    id: Option<GrainId>,
    /// Showing the record (`false`) or changing it (`true`). A new record is
    /// always a form.
    editing: bool,
    form: F,
}

/// A delete waiting for the person to confirm it.
enum Confirm {
    Stamp(GrainId),
    Activity { id: GrainId, title: String },
}

pub struct TallyView {
    runtime: tokio::runtime::Handle,
    client: Arc<TallyClient>,
    repaint: Arc<dyn Fn() + Send + Sync>,
    replies: (mpsc::Sender<Reply>, mpsc::Receiver<Reply>),

    data: Option<Data>,
    /// Why the data could not be read, if the last refresh failed.
    load_error: Option<String>,
    loading: bool,
    last_refresh: Option<Instant>,
    /// The days the heatmaps cover, and the range the stamps were read for.
    grid: Grid,
    /// The day picked in one activity's heatmap: its stamps are listed under
    /// that heatmap.
    picked: Option<(GrainId, NaiveDate)>,
    /// The last write's confirmation, shown under the toolbar.
    notice: Option<String>,

    stamp_window: Option<Window<StampForm>>,
    activity_window: Option<Window<ActivityForm>>,
    confirm: Option<Confirm>,
    /// A write is on its way to the server.
    sending: bool,
    /// Why the open window's form was not sent (or was refused).
    form_error: Option<String>,
}

impl TallyView {
    pub fn new(
        runtime: tokio::runtime::Handle,
        client: TallyClient,
        repaint: Arc<dyn Fn() + Send + Sync>,
    ) -> TallyView {
        TallyView {
            runtime,
            client: Arc::new(client),
            repaint,
            replies: mpsc::channel(),
            data: None,
            load_error: None,
            loading: false,
            last_refresh: None,
            grid: Grid::ending(today()),
            picked: None,
            notice: None,
            stamp_window: None,
            activity_window: None,
            confirm: None,
            sending: false,
            form_error: None,
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        self.drain_replies();
        if !self.loading
            && self
                .last_refresh
                .is_none_or(|t| t.elapsed() >= REFRESH_EVERY)
        {
            self.refresh();
        }
        // Wake up for the next automatic refresh even if nothing else happens.
        ui.ctx().request_repaint_after(REFRESH_EVERY);

        egui::Panel::top("tally_toolbar").show(ui, |ui| self.toolbar(ui));
        egui::CentralPanel::default().show(ui, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| self.activities(ui));
        });

        self.stamp_window(ui.ctx());
        self.activity_window(ui.ctx());
        self.confirm_window(ui.ctx());
    }

    // ── server calls ────────────────────────────────────────────────────────

    fn refresh(&mut self) {
        self.loading = true;
        self.last_refresh = Some(Instant::now());
        let client = Arc::clone(&self.client);
        let grid = self.grid;
        self.spawn(async move {
            let data = async {
                let activities = client.read(names::ACTIVITY_LIST, EmptyParams {}).await?;
                let stamps = client
                    .read(
                        names::STAMP_LIST,
                        QueryParams {
                            activity: None,
                            from: Some(grid.start.to_string()),
                            to: Some(grid.end.to_string()),
                        },
                    )
                    .await?;
                Ok(Data { activities, stamps })
            };
            Reply::Data(grid, data.await)
        });
    }

    fn send_stamp(&mut self) {
        let Some(window) = &self.stamp_window else {
            return;
        };
        let client = Arc::clone(&self.client);
        match window.id {
            None => match window.form.to_add_params() {
                Ok(params) => {
                    self.start_sending();
                    self.spawn(async move {
                        Reply::Written(Sent::Stamp, client.call(names::STAMP_ADD, params).await)
                    });
                }
                Err(e) => self.form_error = Some(e),
            },
            Some(id) => match window.form.to_update_params(id) {
                Ok(params) => {
                    self.start_sending();
                    self.spawn(async move {
                        Reply::Written(Sent::Stamp, client.call(names::STAMP_UPDATE, params).await)
                    });
                }
                Err(e) => self.form_error = Some(e),
            },
        }
    }

    fn send_activity(&mut self) {
        let Some(window) = &self.activity_window else {
            return;
        };
        let client = Arc::clone(&self.client);
        match window.id {
            None => match window.form.to_add_params() {
                Ok(params) => {
                    self.start_sending();
                    self.spawn(async move {
                        let result = client.call(names::ACTIVITY_ADD, params).await;
                        Reply::Written(Sent::Activity, result)
                    });
                }
                Err(e) => self.form_error = Some(e),
            },
            Some(id) => match window.form.to_update_params(id) {
                Ok(params) => {
                    self.start_sending();
                    self.spawn(async move {
                        let result = client.call(names::ACTIVITY_UPDATE, params).await;
                        Reply::Written(Sent::Activity, result)
                    });
                }
                Err(e) => self.form_error = Some(e),
            },
        }
    }

    /// Delete what the confirmation names. An activity goes with its stamps:
    /// the confirmation said so.
    fn send_delete(&mut self, confirm: &Confirm) {
        let client = Arc::clone(&self.client);
        self.start_sending();
        match *confirm {
            Confirm::Stamp(id) => {
                let params = IdParams { id: id.to_string() };
                self.spawn(async move {
                    let result = client.call(names::STAMP_DELETE, params).await;
                    Reply::Written(Sent::DeleteStamp, result)
                });
            }
            Confirm::Activity { id, .. } => {
                let params = ActivityDeleteParams {
                    id: id.to_string(),
                    with_stamps: true,
                };
                self.spawn(async move {
                    let result = client.call(names::ACTIVITY_DELETE, params).await;
                    Reply::Written(Sent::DeleteActivity, result)
                });
            }
        }
    }

    /// Stamp `activity` now, without a window.
    fn stamp_now(&mut self, activity: GrainId) {
        let client = Arc::clone(&self.client);
        let params = StampAddParams {
            activity: activity.to_string(),
            timestamp: None,
            comment: None,
        };
        self.sending = true;
        self.spawn(async move {
            Reply::Written(
                Sent::QuickStamp,
                client.call(names::STAMP_ADD, params).await,
            )
        });
    }

    fn start_sending(&mut self) {
        self.sending = true;
        self.form_error = None;
    }

    fn spawn(&self, call: impl Future<Output = Reply> + Send + 'static) {
        let tx = self.replies.0.clone();
        let repaint = Arc::clone(&self.repaint);
        self.runtime.spawn(async move {
            // The receiver lives as long as the view; a send after the window
            // closed has no one to tell.
            let _ = tx.send(call.await);
            repaint();
        });
    }

    fn drain_replies(&mut self) {
        while let Ok(reply) = self.replies.1.try_recv() {
            match reply {
                // A refresh for a range the view has moved away from.
                Reply::Data(grid, _) if grid != self.grid => {}
                Reply::Data(_, Ok(data)) => {
                    if let Some((id, _)) = self.picked
                        && !data.activities.iter().any(|a| a.id == id)
                    {
                        self.picked = None;
                    }
                    self.data = Some(data);
                    self.load_error = None;
                    self.loading = false;
                }
                Reply::Data(_, Err(e)) => {
                    // Keep showing the last data: a server restart should not
                    // blank the window.
                    self.load_error = Some(e);
                    self.loading = false;
                }
                Reply::Written(sent, Ok(text)) => {
                    self.sending = false;
                    self.notice = Some(match sent {
                        Sent::Stamp | Sent::QuickStamp => {
                            format!("Saved {}.", describe_stamp(&text, self.data.as_ref()))
                        }
                        Sent::Activity => describe_activity(&text),
                        Sent::DeleteStamp => {
                            format!("Deleted {}.", describe_stamp(&text, self.data.as_ref()))
                        }
                        Sent::DeleteActivity => describe_deleted_activity(&text),
                    });
                    match sent {
                        Sent::Stamp => self.stamp_window = None,
                        Sent::Activity => self.activity_window = None,
                        Sent::QuickStamp => {}
                        Sent::DeleteStamp => {
                            self.confirm = None;
                            self.stamp_window = None;
                        }
                        Sent::DeleteActivity => {
                            self.confirm = None;
                            self.activity_window = None;
                        }
                    }
                    self.refresh();
                }
                Reply::Written(sent, Err(e)) => {
                    self.sending = false;
                    if sent == Sent::QuickStamp {
                        self.notice = Some(e);
                    } else {
                        // The window stays open with what was typed.
                        self.form_error = Some(e);
                    }
                }
            }
        }
    }

    // ── panels ──────────────────────────────────────────────────────────────

    fn toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let can_write = self.load_error.is_none() && self.data.is_some();
            if ui
                .add_enabled(can_write, egui::Button::new("New activity"))
                .clicked()
            {
                self.activity_window = Some(Window {
                    id: None,
                    editing: true,
                    form: ActivityForm::default(),
                });
                self.form_error = None;
            }
            if ui
                .add_enabled(!self.loading, egui::Button::new("Refresh"))
                .clicked()
            {
                self.refresh();
            }
            if self.loading || self.sending {
                ui.spinner();
            }
            ui.separator();
            self.range_bar(ui);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                legend(ui);
            });
        });
        if let Some(e) = &self.load_error {
            ui.colored_label(ui.visuals().error_fg_color, e);
        }
        if let Some(text) = &self.notice {
            ui.label(text);
        }
    }

    fn range_bar(&mut self, ui: &mut egui::Ui) {
        let mut moved = None;
        if ui.button("◀").on_hover_text("A year earlier").clicked() {
            moved = Some(self.grid.end - Days::new(heatmap::WEEKS * 7));
        }
        let latest = self.grid.end >= today();
        if ui
            .add_enabled(!latest, egui::Button::new("▶"))
            .on_hover_text("A year later")
            .clicked()
        {
            moved = Some((self.grid.end + Days::new(heatmap::WEEKS * 7)).min(today()));
        }
        if ui
            .add_enabled(!latest, egui::Button::new("Today"))
            .clicked()
        {
            moved = Some(today());
        }
        ui.label(format!("{} – {}", self.grid.start, self.grid.end));
        if let Some(end) = moved {
            self.grid = Grid::ending(end);
            self.picked = None;
            self.refresh();
        }
    }

    /// Every activity: its name and buttons, its heatmap, and the stamps of
    /// the day picked in it.
    fn activities(&mut self, ui: &mut egui::Ui) {
        let Some(data) = &self.data else {
            ui.label("Loading…");
            return;
        };
        if data.activities.is_empty() {
            ui.add_space(8.0);
            ui.label("No activities yet. Every stamp is for one: start with New activity.");
            return;
        }
        let activities = data.activities.clone();
        for (i, activity) in activities.iter().enumerate() {
            if i > 0 {
                ui.separator();
            }
            ui.push_id(activity.id.to_string(), |ui| self.activity(ui, activity));
        }
    }

    fn activity(&mut self, ui: &mut egui::Ui, activity: &Activity) {
        let Some(data) = &self.data else {
            return;
        };
        let counts = heatmap::day_counts(&data.stamps, Some(activity.id));
        let total: u32 = counts.values().sum();

        ui.horizontal(|ui| {
            if ui
                .add(
                    egui::Label::new(egui::RichText::new(&activity.title).heading())
                        .sense(egui::Sense::click()),
                )
                .on_hover_text("Show its details")
                .clicked()
            {
                self.open_activity(activity);
            }
            let noun = if total == 1 { "stamp" } else { "stamps" };
            ui.weak(format!("{total} {noun}"));
            if ui
                .add_enabled(!self.sending, egui::Button::new("Stamp now"))
                .on_hover_text("Add a stamp at the current time")
                .clicked()
            {
                self.stamp_now(activity.id);
            }
            if ui
                .button("Stamp…")
                .on_hover_text("Add a stamp with a time and a comment")
                .clicked()
            {
                self.open_new_stamp(activity.id);
            }
            if ui.button("Details").clicked() {
                self.open_activity(activity);
            }
        });

        let picked_day = self
            .picked
            .filter(|(id, _)| *id == activity.id)
            .map(|(_, day)| day);
        if let Some(day) = heatmap_ui(ui, self.grid, &counts, picked_day) {
            self.picked = (picked_day != Some(day)).then_some((activity.id, day));
        }
        if let Some(day) = picked_day {
            self.day_stamps(ui, activity.id, day);
        }
    }

    fn day_stamps(&mut self, ui: &mut egui::Ui, activity: GrainId, day: NaiveDate) {
        let Some(data) = &self.data else {
            return;
        };
        let mut stamps: Vec<&Stamp> = data
            .stamps
            .iter()
            .filter(|s| s.activity == activity && s.timestamp.date_naive() == day)
            .collect();
        stamps.reverse();

        let mut open = None;
        let mut close = false;
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.strong(format!("Stamps on {day}"));
            if ui.small_button("Close").clicked() {
                close = true;
            }
        });
        if stamps.is_empty() {
            ui.weak("No stamps.");
        } else {
            egui::Grid::new("day_stamps")
                .num_columns(2)
                .striped(true)
                .spacing([16.0, 4.0])
                .show(ui, |ui| {
                    for stamp in stamps {
                        let when = stamp.timestamp.format("%H:%M").to_string();
                        if ui.link(when).on_hover_text("Show this stamp").clicked() {
                            open = Some(stamp.clone());
                        }
                        ui.label(first_line(stamp.comment.as_deref().unwrap_or_default()));
                        ui.end_row();
                    }
                });
        }
        if close {
            self.picked = None;
        }
        if let Some(stamp) = open {
            self.stamp_window = Some(Window {
                id: Some(stamp.id),
                editing: false,
                form: StampForm::edit(&stamp),
            });
            self.form_error = None;
        }
    }

    // ── windows ─────────────────────────────────────────────────────────────

    fn open_new_stamp(&mut self, activity: GrainId) {
        let mut form = StampForm::new(Some(activity), Local::now().fixed_offset());
        // A stamp added while a past day is picked is most likely for that day.
        if let Some((id, day)) = self.picked
            && id == activity
            && day != today()
        {
            form.when = format!("{day} 12:00");
        }
        self.stamp_window = Some(Window {
            id: None,
            editing: true,
            form,
        });
        self.form_error = None;
    }

    fn open_activity(&mut self, activity: &Activity) {
        self.activity_window = Some(Window {
            id: Some(activity.id),
            editing: false,
            form: ActivityForm::edit(activity),
        });
        self.form_error = None;
    }

    fn stamp_window(&mut self, ctx: &egui::Context) {
        let Some(window) = &mut self.stamp_window else {
            return;
        };
        let activities: Vec<(GrainId, String)> = self
            .data
            .as_ref()
            .map(|d| {
                d.activities
                    .iter()
                    .map(|a| (a.id, a.title.clone()))
                    .collect()
            })
            .unwrap_or_default();
        let title_of = |id: Option<GrainId>| {
            activities
                .iter()
                .find(|(a, _)| Some(*a) == id)
                .map_or("(choose)".to_owned(), |(_, t)| t.clone())
        };
        let title = match window.id {
            None => "New stamp".to_owned(),
            Some(id) => format!("Stamp {id}"),
        };
        let mut open = true;
        let mut action = Action::None;
        egui::Window::new(title)
            .id(egui::Id::new("tally_stamp_window"))
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(ctx.content_rect().center())
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_width(420.0)
            .show(ctx, |ui| {
                let form = &mut window.form;
                egui::Grid::new("stamp_fields")
                    .num_columns(2)
                    .show(ui, |ui| {
                        ui.label("Activity");
                        if window.editing {
                            egui::ComboBox::from_id_salt("stamp_activity")
                                .selected_text(title_of(form.activity))
                                .show_ui(ui, |ui| {
                                    for (id, title) in &activities {
                                        ui.selectable_value(&mut form.activity, Some(*id), title);
                                    }
                                });
                        } else {
                            ui.label(title_of(form.activity));
                        }
                        ui.end_row();
                        ui.label("Time");
                        if window.editing {
                            ui.add(
                                egui::TextEdit::singleline(&mut form.when)
                                    .hint_text("YYYY-MM-DD HH:MM"),
                            );
                        } else {
                            ui.label(&form.when);
                        }
                        ui.end_row();
                    });
                ui.label("Comment");
                if window.editing {
                    ui.add(
                        egui::TextEdit::multiline(&mut form.comment)
                            .desired_rows(6)
                            .desired_width(f32::INFINITY)
                            .hint_text("Markdown"),
                    );
                } else if form.comment.trim().is_empty() {
                    ui.weak("No comment.");
                } else {
                    ui.label(&form.comment);
                }
                action = window_buttons(
                    ui,
                    window.editing,
                    window.id.is_some(),
                    self.sending,
                    &self.form_error,
                );
            });
        match action {
            Action::None => {}
            Action::Edit => window.editing = true,
            Action::Submit => {}
            Action::CancelEdit => {
                window.editing = false;
                // Back to the record as stored.
                if let Some(stamp) = window
                    .id
                    .and_then(|id| self.data.as_ref()?.stamps.iter().find(|s| s.id == id))
                {
                    window.form = StampForm::edit(stamp);
                }
                self.form_error = None;
            }
            Action::Delete => {
                if let Some(id) = window.id {
                    self.confirm = Some(Confirm::Stamp(id));
                    self.form_error = None;
                }
            }
        }
        if !open {
            self.stamp_window = None;
            self.form_error = None;
        } else if action == Action::Submit {
            self.send_stamp();
        }
    }

    fn activity_window(&mut self, ctx: &egui::Context) {
        let Some(window) = &mut self.activity_window else {
            return;
        };
        let title = match window.id {
            None => "New activity".to_owned(),
            Some(_) => window.form.title.clone(),
        };
        let mut open = true;
        let mut action = Action::None;
        egui::Window::new(title)
            .id(egui::Id::new("tally_activity_window"))
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(ctx.content_rect().center())
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_width(420.0)
            .show(ctx, |ui| {
                let form = &mut window.form;
                egui::Grid::new("activity_fields")
                    .num_columns(2)
                    .show(ui, |ui| {
                        if let Some(id) = window.id {
                            ui.label("Id");
                            ui.monospace(id.to_string());
                            ui.end_row();
                        }
                        ui.label("Title");
                        if window.editing {
                            ui.add(
                                egui::TextEdit::singleline(&mut form.title).desired_width(280.0),
                            );
                        } else {
                            ui.label(&form.title);
                        }
                        ui.end_row();
                        ui.label("Unit");
                        if window.editing {
                            egui::ComboBox::from_id_salt("activity_unit")
                                .selected_text(form::unit_name(form.unit))
                                .show_ui(ui, |ui| {
                                    for unit in form::UNITS {
                                        ui.selectable_value(
                                            &mut form.unit,
                                            unit,
                                            form::unit_name(unit),
                                        );
                                    }
                                });
                        } else {
                            ui.label(form::unit_name(form.unit));
                        }
                        ui.end_row();
                    });
                ui.label("Description");
                if window.editing {
                    ui.add(
                        egui::TextEdit::multiline(&mut form.description)
                            .desired_rows(6)
                            .desired_width(f32::INFINITY)
                            .hint_text("Markdown"),
                    );
                } else if form.description.trim().is_empty() {
                    ui.weak("No description.");
                } else {
                    ui.label(&form.description);
                }
                action = window_buttons(
                    ui,
                    window.editing,
                    window.id.is_some(),
                    self.sending,
                    &self.form_error,
                );
            });
        match action {
            Action::None | Action::Submit => {}
            Action::Edit => window.editing = true,
            Action::CancelEdit => {
                window.editing = false;
                if let Some(activity) = window
                    .id
                    .and_then(|id| self.data.as_ref()?.activities.iter().find(|a| a.id == id))
                {
                    window.form = ActivityForm::edit(activity);
                }
                self.form_error = None;
            }
            Action::Delete => {
                if let Some(id) = window.id {
                    self.confirm = Some(Confirm::Activity {
                        id,
                        title: window.form.title.clone(),
                    });
                    self.form_error = None;
                }
            }
        }
        if !open {
            self.activity_window = None;
            self.form_error = None;
        } else if action == Action::Submit {
            self.send_activity();
        }
    }

    /// Ask before deleting. Nothing else can be clicked meanwhile.
    fn confirm_window(&mut self, ctx: &egui::Context) {
        let Some(confirm) = self.confirm.take() else {
            return;
        };
        let mut keep = true;
        let mut delete = false;
        egui::Modal::new(egui::Id::new("tally_confirm_delete")).show(ctx, |ui| {
            ui.set_max_width(380.0);
            match &confirm {
                Confirm::Stamp(_) => {
                    ui.heading("Delete this stamp?");
                }
                Confirm::Activity { title, .. } => {
                    ui.heading(format!("Delete {title}?"));
                    ui.label("All of its stamps are deleted with it, in every year.");
                }
            }
            ui.label("This cannot be undone.");
            if let Some(e) = &self.form_error {
                ui.colored_label(ui.visuals().error_fg_color, e);
            }
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                let button = egui::Button::new(
                    egui::RichText::new("Delete").color(ui.visuals().error_fg_color),
                );
                if ui.add_enabled(!self.sending, button).clicked() {
                    delete = true;
                }
                if ui.button("Cancel").clicked() {
                    keep = false;
                }
                if self.sending {
                    ui.spinner();
                }
            });
        });
        if delete {
            self.send_delete(&confirm);
        }
        if keep {
            self.confirm = Some(confirm);
        } else {
            self.form_error = None;
        }
    }
}

/// What the buttons under a record's window asked for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Action {
    None,
    Edit,
    Submit,
    CancelEdit,
    Delete,
}

/// The buttons under a record's window, and the form's error.
///
/// Showing a record offers Edit and Delete; a form offers Save (Add for a new
/// record) and, for an existing record, Cancel back to showing it.
fn window_buttons(
    ui: &mut egui::Ui,
    editing: bool,
    exists: bool,
    sending: bool,
    error: &Option<String>,
) -> Action {
    let mut action = Action::None;
    ui.separator();
    if let Some(e) = error {
        ui.colored_label(ui.visuals().error_fg_color, e);
    }
    ui.horizontal(|ui| {
        if !editing {
            if ui.button("Edit").clicked() {
                action = Action::Edit;
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let delete = egui::Button::new(
                    egui::RichText::new("Delete").color(ui.visuals().error_fg_color),
                );
                if ui.add_enabled(!sending, delete).clicked() {
                    action = Action::Delete;
                }
            });
            return;
        }
        let label = if exists { "Save" } else { "Add" };
        if ui.add_enabled(!sending, egui::Button::new(label)).clicked() {
            action = Action::Submit;
        }
        if exists && ui.button("Cancel").clicked() {
            action = Action::CancelEdit;
        }
        if sending {
            ui.spinner();
        }
    });
    action
}

/// One activity's heatmap: a column per week, shaded by each day's count
/// against the activity's busiest day in view. Returns the day clicked.
fn heatmap_ui(
    ui: &mut egui::Ui,
    grid: Grid,
    counts: &BTreeMap<NaiveDate, u32>,
    picked: Option<NaiveDate>,
) -> Option<NaiveDate> {
    let max = counts.values().copied().max().unwrap_or(0);
    let palette = palette(ui.visuals().dark_mode);
    let step = CELL + GAP;
    let size = egui::vec2(LABEL_W + heatmap::WEEKS as f32 * step, LABEL_H + 7.0 * step);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    let painter = ui.painter_at(rect);
    let origin = rect.min + egui::vec2(LABEL_W, LABEL_H);
    let font = egui::FontId::proportional(10.0);
    let text_color = ui.visuals().weak_text_color();

    for (row, name) in [(0, "Mon"), (2, "Wed"), (4, "Fri")] {
        painter.text(
            egui::pos2(rect.min.x, origin.y + row as f32 * step + CELL / 2.0),
            egui::Align2::LEFT_CENTER,
            name,
            font.clone(),
            text_color,
        );
    }

    let hovered = response.hover_pos();
    let mut hovered_day = None;
    for week in 0..heatmap::WEEKS {
        let x = origin.x + week as f32 * step;
        // A month's label goes over the column its first day is in.
        if let Some(first) = (0..7)
            .filter_map(|d| grid.day(week, d))
            .find(|d| d.day() == 1)
        {
            painter.text(
                egui::pos2(x, rect.min.y),
                egui::Align2::LEFT_TOP,
                first.format("%b").to_string(),
                font.clone(),
                text_color,
            );
        }
        for weekday in 0..7 {
            let Some(day) = grid.day(week, weekday) else {
                continue;
            };
            let cell = egui::Rect::from_min_size(
                egui::pos2(x, origin.y + weekday as f32 * step),
                egui::vec2(CELL, CELL),
            );
            let count = counts.get(&day).copied().unwrap_or(0);
            let fill = palette[usize::from(heatmap::level(count, max))];
            painter.rect_filled(cell, 2.0, fill);
            let is_hovered = hovered.is_some_and(|p| cell.expand(GAP / 2.0).contains(p));
            if picked == Some(day) || is_hovered {
                painter.rect_stroke(
                    cell,
                    2.0,
                    egui::Stroke::new(1.5, ui.visuals().strong_text_color()),
                    egui::StrokeKind::Outside,
                );
            }
            if is_hovered {
                hovered_day = Some((day, count));
            }
        }
    }

    let clicked = response.clicked();
    if let Some((day, count)) = hovered_day {
        let noun = if count == 1 { "stamp" } else { "stamps" };
        response.on_hover_text_at_pointer(format!("{count} {noun} on {day}"));
    }
    clicked.then_some(hovered_day.map(|(day, _)| day)).flatten()
}

/// "Less ■■■■■ More".
fn legend(ui: &mut egui::Ui) {
    // Laid out right to left: the last swatch first.
    ui.weak("More");
    for fill in palette(ui.visuals().dark_mode).into_iter().rev() {
        let (cell, _) = ui.allocate_exact_size(egui::vec2(CELL, CELL), egui::Sense::hover());
        ui.painter().rect_filled(cell, 2.0, fill);
    }
    ui.weak("Less");
}

/// GitHub's contribution greens, level 0 (no stamps) to [`heatmap::LEVELS`].
fn palette(dark: bool) -> [egui::Color32; 5] {
    let rgb = egui::Color32::from_rgb;
    if dark {
        [
            rgb(0x2d, 0x33, 0x3b),
            rgb(0x0e, 0x44, 0x29),
            rgb(0x00, 0x6d, 0x32),
            rgb(0x26, 0xa6, 0x41),
            rgb(0x39, 0xd3, 0x53),
        ]
    } else {
        [
            rgb(0xeb, 0xed, 0xf0),
            rgb(0x9b, 0xe9, 0xa8),
            rgb(0x40, 0xc4, 0x63),
            rgb(0x30, 0xa1, 0x4e),
            rgb(0x21, 0x6e, 0x39),
        ]
    }
}

/// "a stamp for <activity> at <time>" from the stamp JSON the server
/// answered with.
fn describe_stamp(text: &str, data: Option<&Data>) -> String {
    let Ok(stamp) = serde_json::from_str::<Stamp>(text) else {
        return "the stamp".to_owned();
    };
    let title = data
        .and_then(|d| d.activities.iter().find(|a| a.id == stamp.activity))
        .map_or_else(|| stamp.activity.to_string(), |a| a.title.clone());
    format!(
        "a stamp for {title} at {}",
        stamp.timestamp.format(form::TIME_FORMAT)
    )
}

fn describe_activity(text: &str) -> String {
    match serde_json::from_str::<Activity>(text) {
        Ok(activity) => format!("Saved {}.", activity.title),
        Err(_) => "Saved the activity.".to_owned(),
    }
}

fn describe_deleted_activity(text: &str) -> String {
    match serde_json::from_str::<ActivityDeleted>(text) {
        Ok(deleted) => {
            let noun = if deleted.stamps_deleted == 1 {
                "stamp"
            } else {
                "stamps"
            };
            format!(
                "Deleted {} and its {} {noun}.",
                deleted.activity.title, deleted.stamps_deleted
            )
        }
        Err(_) => "Deleted the activity.".to_owned(),
    }
}

fn first_line(text: &str) -> &str {
    text.lines().next().unwrap_or_default()
}

fn today() -> NaiveDate {
    Local::now().date_naive()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stamp_is_described_by_its_activity() {
        let activity = Activity {
            id: "012atvw".parse().unwrap(),
            title: "筋トレ".into(),
            unit: None,
            description: None,
        };
        let data = Data {
            activities: vec![activity],
            stamps: vec![],
        };
        let text =
            r#"{"id":"012atvx","activity":"012atvw","timestamp":"2026-09-05T09:12:00+09:00"}"#;
        assert_eq!(
            describe_stamp(text, Some(&data)),
            "a stamp for 筋トレ at 2026-09-05 09:12"
        );
        assert_eq!(describe_stamp("not json", None), "the stamp");
    }

    #[test]
    fn a_deleted_activity_says_how_many_stamps_went() {
        let text = r#"{"activity":{"id":"012atvw","title":"筋トレ"},"stamps_deleted":3}"#;
        assert_eq!(
            describe_deleted_activity(text),
            "Deleted 筋トレ and its 3 stamps."
        );
    }
}
