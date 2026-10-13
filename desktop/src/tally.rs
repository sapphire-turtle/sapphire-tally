//! The Tally tab: the activities, a year of stamps as a heatmap, the stamps
//! of the chosen day, and the windows that show, add and change a stamp or an
//! activity.
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
use eframe::egui;
use grain_id::GrainId;
use sapphire_tally_core::{Activity, Stamp};
use sapphire_tally_mcp::methods::{EmptyParams, QueryParams, StampAddParams, names};

use crate::client::TallyClient;
use crate::form::{self, ActivityForm, StampForm};
use crate::heatmap::{self, Grid};

/// How often the tab re-reads the data on its own, to pick up what the CLI,
/// an agent or sync wrote.
const REFRESH_EVERY: Duration = Duration::from_secs(10);

/// How many stamps the list shows when no day is chosen.
const RECENT: usize = 50;

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
    /// A write went through; the text is the record the server wrote.
    Written(Sent, Result<String, String>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Sent {
    Stamp,
    Activity,
    /// The one-click "Stamp now", which has no window to close.
    QuickStamp,
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
    /// The days the heatmap covers, and the range the stamps were read for.
    grid: Grid,
    /// The activity the heatmap and list are for; `None` is all of them.
    selected: Option<GrainId>,
    /// The day whose stamps are listed; `None` lists the latest ones.
    day: Option<NaiveDate>,
    /// The last write's confirmation, shown under the toolbar.
    notice: Option<String>,

    stamp_window: Option<Window<StampForm>>,
    activity_window: Option<Window<ActivityForm>>,
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
            selected: None,
            day: None,
            notice: None,
            stamp_window: None,
            activity_window: None,
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
        egui::Panel::left("tally_activities")
            .resizable(true)
            .default_size(240.0)
            .min_size(160.0)
            .show(ui, |ui| self.activities(ui));
        egui::CentralPanel::default().show(ui, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| self.main(ui));
        });

        self.stamp_window(ui.ctx());
        self.activity_window(ui.ctx());
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
                    if let Some(id) = self.selected
                        && !data.activities.iter().any(|a| a.id == id)
                    {
                        self.selected = None;
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
                        Sent::Stamp | Sent::QuickStamp => describe_stamp(&text, self.data.as_ref()),
                        Sent::Activity => describe_activity(&text),
                    });
                    match sent {
                        Sent::Stamp => self.stamp_window = None,
                        Sent::Activity => self.activity_window = None,
                        Sent::QuickStamp => {}
                    }
                    self.refresh();
                }
                Reply::Written(sent, Err(e)) => {
                    self.sending = false;
                    if sent == Sent::QuickStamp {
                        self.notice = Some(e);
                    } else {
                        // The form stays open with what was typed.
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
            let has_activities = self.data.as_ref().is_some_and(|d| !d.activities.is_empty());
            if ui
                .add_enabled(can_write && has_activities, egui::Button::new("New stamp"))
                .on_disabled_hover_text("Every stamp is for an activity: add one first")
                .clicked()
            {
                self.open_new_stamp();
            }
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
        });
        if let Some(e) = &self.load_error {
            ui.colored_label(ui.visuals().error_fg_color, e);
        }
        if let Some(text) = &self.notice {
            ui.label(text);
        }
    }

    fn activities(&mut self, ui: &mut egui::Ui) {
        let Some(data) = &self.data else {
            ui.label("Loading…");
            return;
        };
        let counts = per_activity(&data.stamps);
        let mut open = None;
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.selectable_value(&mut self.selected, None, "All activities");
            if data.activities.is_empty() {
                ui.add_space(8.0);
                ui.label("No activities yet. Every stamp is for one: start with New activity.");
            }
            for activity in &data.activities {
                let selected = self.selected == Some(activity.id);
                let count = counts.get(&activity.id).copied().unwrap_or(0);
                // Full width, the count at the right edge; a long title is
                // cut short rather than drawn over the count.
                let response = ui
                    .add(
                        egui::Button::selectable(selected, &activity.title)
                            .shortcut_text(count.to_string())
                            .truncate()
                            .min_size(egui::vec2(ui.available_width(), 0.0)),
                    )
                    .on_hover_text(format!(
                        "{count} stamps in the year shown. Double-click for details."
                    ));
                if response.clicked() {
                    self.selected = Some(activity.id);
                }
                if response.double_clicked() {
                    open = Some(activity.clone());
                }
            }
        });
        if let Some(activity) = open {
            self.open_activity(&activity);
        }
    }

    fn main(&mut self, ui: &mut egui::Ui) {
        let Some(data) = &self.data else {
            return;
        };
        let activity = self
            .selected
            .and_then(|id| data.activities.iter().find(|a| a.id == id))
            .cloned();

        ui.horizontal(|ui| {
            ui.heading(
                activity
                    .as_ref()
                    .map_or("All activities", |a| a.title.as_str()),
            );
            if let Some(activity) = &activity {
                if ui
                    .add_enabled(!self.sending, egui::Button::new("Stamp now"))
                    .on_hover_text("Add a stamp for this activity at the current time")
                    .clicked()
                {
                    self.stamp_now(activity.id);
                }
                if ui.button("Details").clicked() {
                    self.open_activity(activity);
                }
            }
        });
        ui.add_space(4.0);
        self.range_bar(ui);
        ui.add_space(4.0);
        self.heatmap(ui);
        ui.add_space(12.0);
        self.stamp_list(ui);
    }

    fn range_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
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
                self.day = None;
                self.refresh();
            }
        });
    }

    fn heatmap(&mut self, ui: &mut egui::Ui) {
        let Some(data) = &self.data else {
            return;
        };
        let counts = heatmap::day_counts(&data.stamps, self.selected);
        let max = counts.values().copied().max().unwrap_or(0);
        let palette = palette(ui.visuals().dark_mode);
        let grid = self.grid;
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
                if self.day == Some(day) || is_hovered {
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

        if response.clicked()
            && let Some((day, _)) = hovered_day
        {
            self.day = (self.day != Some(day)).then_some(day);
        }
        if let Some((day, count)) = hovered_day {
            let noun = if count == 1 { "stamp" } else { "stamps" };
            response.on_hover_text_at_pointer(format!("{count} {noun} on {day}"));
        }

        ui.horizontal(|ui| {
            ui.weak("Less");
            for fill in palette {
                let (cell, _) =
                    ui.allocate_exact_size(egui::vec2(CELL, CELL), egui::Sense::hover());
                ui.painter().rect_filled(cell, 2.0, fill);
            }
            ui.weak("More");
        });
    }

    fn stamp_list(&mut self, ui: &mut egui::Ui) {
        let Some(data) = &self.data else {
            return;
        };
        let titles: BTreeMap<GrainId, &str> = data
            .activities
            .iter()
            .map(|a| (a.id, a.title.as_str()))
            .collect();
        let mut stamps: Vec<&Stamp> = data
            .stamps
            .iter()
            .filter(|s| self.selected.is_none_or(|a| a == s.activity))
            .filter(|s| self.day.is_none_or(|d| d == s.timestamp.date_naive()))
            .collect();
        stamps.reverse();
        let heading = match self.day {
            Some(day) => format!("Stamps on {day}"),
            None => "Latest stamps".to_owned(),
        };

        let mut open = None;
        ui.horizontal(|ui| {
            ui.strong(heading);
            if self.day.is_some() && ui.small_button("Show latest").clicked() {
                self.day = None;
            }
        });
        if stamps.is_empty() {
            ui.weak("No stamps.");
            return;
        }
        egui::Grid::new("tally_stamps")
            .num_columns(3)
            .striped(true)
            .spacing([16.0, 4.0])
            .show(ui, |ui| {
                for stamp in stamps.into_iter().take(if self.day.is_some() {
                    usize::MAX
                } else {
                    RECENT
                }) {
                    let when = stamp.timestamp.format(form::TIME_FORMAT).to_string();
                    if ui.link(when).on_hover_text("Show this stamp").clicked() {
                        open = Some(stamp.clone());
                    }
                    ui.label(titles.get(&stamp.activity).copied().unwrap_or("(unknown)"));
                    ui.label(first_line(stamp.comment.as_deref().unwrap_or_default()));
                    ui.end_row();
                }
            });
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

    fn open_new_stamp(&mut self) {
        let activity = self.selected.or_else(|| {
            self.data
                .as_ref()
                .and_then(|d| d.activities.first())
                .map(|a| a.id)
        });
        let mut form = StampForm::new(activity, Local::now().fixed_offset());
        // A stamp added while a past day is chosen is most likely for that day.
        if let Some(day) = self.day.filter(|d| *d != today()) {
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
        let mut submit = false;
        let mut cancel_edit = false;
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
                submit = window_buttons(
                    ui,
                    &mut window.editing,
                    window.id.is_some(),
                    self.sending,
                    &self.form_error,
                    &mut cancel_edit,
                );
            });
        if cancel_edit {
            // Back to the record as stored.
            if let Some(stamp) = window
                .id
                .and_then(|id| self.data.as_ref()?.stamps.iter().find(|s| s.id == id))
            {
                window.form = StampForm::edit(stamp);
            }
            self.form_error = None;
        }
        if !open {
            self.stamp_window = None;
            self.form_error = None;
        } else if submit {
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
        let mut submit = false;
        let mut cancel_edit = false;
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
                submit = window_buttons(
                    ui,
                    &mut window.editing,
                    window.id.is_some(),
                    self.sending,
                    &self.form_error,
                    &mut cancel_edit,
                );
            });
        if cancel_edit {
            if let Some(activity) = window
                .id
                .and_then(|id| self.data.as_ref()?.activities.iter().find(|a| a.id == id))
            {
                window.form = ActivityForm::edit(activity);
            }
            self.form_error = None;
        }
        if !open {
            self.activity_window = None;
            self.form_error = None;
        } else if submit {
            self.send_activity();
        }
    }
}

/// The buttons under a record's window, and the form's error. Returns whether
/// the form was submitted.
///
/// Showing a record offers Edit; a form offers Save (Add for a new record)
/// and, for an existing record, Cancel back to showing it.
fn window_buttons(
    ui: &mut egui::Ui,
    editing: &mut bool,
    exists: bool,
    sending: bool,
    error: &Option<String>,
    cancel_edit: &mut bool,
) -> bool {
    let mut submit = false;
    ui.separator();
    if let Some(e) = error {
        ui.colored_label(ui.visuals().error_fg_color, e);
    }
    ui.horizontal(|ui| {
        if !*editing {
            if ui.button("Edit").clicked() {
                *editing = true;
            }
            return;
        }
        let label = if exists { "Save" } else { "Add" };
        if ui.add_enabled(!sending, egui::Button::new(label)).clicked() {
            submit = true;
        }
        if exists && ui.button("Cancel").clicked() {
            *editing = false;
            *cancel_edit = true;
        }
        if sending {
            ui.spinner();
        }
    });
    submit
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

fn per_activity(stamps: &[Stamp]) -> BTreeMap<GrainId, usize> {
    let mut counts = BTreeMap::new();
    for stamp in stamps {
        *counts.entry(stamp.activity).or_insert(0) += 1;
    }
    counts
}

/// "Stamped <activity> at <time>" from the stamp JSON the server wrote.
fn describe_stamp(text: &str, data: Option<&Data>) -> String {
    let Ok(stamp) = serde_json::from_str::<Stamp>(text) else {
        return "Saved the stamp.".to_owned();
    };
    let title = data
        .and_then(|d| d.activities.iter().find(|a| a.id == stamp.activity))
        .map_or_else(|| stamp.activity.to_string(), |a| a.title.clone());
    format!(
        "Saved a stamp for {title} at {}.",
        stamp.timestamp.format(form::TIME_FORMAT)
    )
}

fn describe_activity(text: &str) -> String {
    match serde_json::from_str::<Activity>(text) {
        Ok(activity) => format!("Saved {}.", activity.title),
        Err(_) => "Saved the activity.".to_owned(),
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
    fn a_saved_stamp_is_described_by_its_activity() {
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
            "Saved a stamp for 筋トレ at 2026-09-05 09:12."
        );
        assert_eq!(describe_stamp("not json", None), "Saved the stamp.");
    }
}
