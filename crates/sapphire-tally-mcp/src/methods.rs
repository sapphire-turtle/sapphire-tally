//! The tally's methods on its app server: what the MCP tools call.
//!
//! Every tool reaches the tally the way the framework's own tools reach the
//! workspace — over the app server's IPC, as `sapphire-tally.<tool>` — so one
//! implementation serves the `mcp` subcommand (stdio, dialling the running
//! server) and `/mcp` on the server's HTTP listener alike. [`register`] adds
//! the methods to the server's router; [`crate::tools`] is the other end.

use std::sync::Arc;

use sapphire_framework::ipc::{RequestCtx, Router, RpcError};
use std::collections::HashMap;

use grain_id::GrainId;
use sapphire_tally_core::{
    Activity, ActivityUpdate, Color, HeatmapBucket, Stamp, StampUpdate, Status, Tally, Target, Unit,
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

/// Where a call finds its tally: asked once per call.
///
/// The app server's tally is whatever workspace it has selected, which
/// `workspace select` can change underneath it; an error refuses the call.
pub type StateSource = Arc<dyn Fn() -> std::io::Result<Tally> + Send + Sync>;

/// The tally's methods, `sapphire-tally.<tool>`.
pub mod names {
    pub const ACTIVITY_ADD: &str = "sapphire-tally.activity_add";
    pub const ACTIVITY_GET: &str = "sapphire-tally.activity_get";
    pub const ACTIVITY_UPDATE: &str = "sapphire-tally.activity_update";
    pub const ACTIVITY_DELETE: &str = "sapphire-tally.activity_delete";
    pub const ACTIVITY_LIST: &str = "sapphire-tally.activity_list";
    pub const STAMP_ADD: &str = "sapphire-tally.stamp_add";
    pub const STAMP_GET: &str = "sapphire-tally.stamp_get";
    pub const STAMP_UPDATE: &str = "sapphire-tally.stamp_update";
    pub const STAMP_DELETE: &str = "sapphire-tally.stamp_delete";
    pub const STAMP_LIST: &str = "sapphire-tally.stamp_list";
    pub const HEATMAP: &str = "sapphire-tally.heatmap";
}

/// The version of the methods in [`names`], reported in the IPC handshake.
///
/// Bump it when a method's parameters or meaning change incompatibly: a
/// client built against another version is refused at connect time rather
/// than mid-call.
pub const APP_API: u32 = 3;

/// Add the tally's methods to the app server's `router`, each resolving the
/// tally through `source` when it is called.
pub fn register(router: Router, source: StateSource) -> Router {
    let router = route(
        router,
        names::ACTIVITY_ADD,
        &source,
        TallyMethods::activity_add,
    );
    let router = route(
        router,
        names::ACTIVITY_GET,
        &source,
        TallyMethods::activity_get,
    );
    let router = route(
        router,
        names::ACTIVITY_UPDATE,
        &source,
        TallyMethods::activity_update,
    );
    let router = route(
        router,
        names::ACTIVITY_DELETE,
        &source,
        TallyMethods::activity_delete,
    );
    let router = route(
        router,
        names::ACTIVITY_LIST,
        &source,
        TallyMethods::activity_list,
    );
    let router = route(router, names::STAMP_ADD, &source, TallyMethods::stamp_add);
    let router = route(router, names::STAMP_GET, &source, TallyMethods::stamp_get);
    let router = route(
        router,
        names::STAMP_UPDATE,
        &source,
        TallyMethods::stamp_update,
    );
    let router = route(
        router,
        names::STAMP_DELETE,
        &source,
        TallyMethods::stamp_delete,
    );
    let router = route(router, names::STAMP_LIST, &source, TallyMethods::stamp_list);
    route(router, names::HEATMAP, &source, TallyMethods::heatmap)
}

/// Register one method: parse its parameters, then run `method` against the
/// current tally on the blocking pool — the tally is plain files.
///
/// The method's text is the result; its error text is the RPC error's message,
/// which the tool hands to the agent as it is.
fn route<P>(
    router: Router,
    name: &str,
    source: &StateSource,
    method: fn(&TallyMethods, P) -> Result<String, String>,
) -> Router
where
    P: DeserializeOwned + Send + 'static,
{
    let source = Arc::clone(source);
    router.method(name, move |ctx: RequestCtx| {
        let source = Arc::clone(&source);
        async move {
            let params: P = serde_json::from_value(ctx.params)
                .map_err(|e| RpcError::invalid_params(e.to_string()))?;
            let text = tokio::task::spawn_blocking(move || {
                let tally = source().map_err(|e| e.to_string())?;
                method(&TallyMethods::new(tally), params)
            })
            .await
            .map_err(|e| RpcError::internal(format!("the tally method panicked: {e}")))?
            .map_err(RpcError::internal)?;
            Ok(serde_json::Value::String(text))
        }
    })
}

/// The tally side of one call: the tally it works on, and a function per
/// method returning the JSON text the agent reads.
#[derive(Clone, Debug)]
pub struct TallyMethods {
    tally: Tally,
}

// ── parameter structs ─────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct EmptyParams {}

#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ActivityAddParams {
    pub title: String,
    /// Period the count is measured and drawn in: day/week/month (day when omitted).
    #[serde(default)]
    pub unit: Option<String>,
    /// The activity's target, as `N-bound`: `3-below` is a habit cut to at most 3 its unit. Omitted means only counted.
    #[serde(default)]
    pub target: Option<String>,
    /// A swatch name (green, blue, …) or oklch(C H). Derived from the title
    /// when omitted or empty.
    #[serde(default)]
    pub color: Option<String>,
    /// Free Markdown text about the activity.
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct IdParams {
    /// The record's grain-id.
    pub id: String,
}

/// Omitted fields stay as they are.
#[derive(Debug, Default, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ActivityUpdateParams {
    /// The activity's grain-id.
    pub id: String,
    #[serde(default)]
    pub title: Option<String>,
    /// Period the count is measured and drawn in: day/week/month.
    #[serde(default)]
    pub unit: Option<String>,
    /// The activity's target, as `N-bound`: `3-below` is a habit cut to at most 3 its unit. Omitted leaves it as it is. An empty string removes it.
    #[serde(default)]
    pub target: Option<String>,
    /// A swatch name (green, blue, …) or oklch(C H). On update an empty
    /// string removes it.
    #[serde(default)]
    pub color: Option<String>,
    /// Free Markdown text. An empty string removes it.
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct StampAddParams {
    pub activity: String,
    /// ISO 8601 timestamp with a timezone. The server's current time when
    /// omitted.
    #[serde(default)]
    pub timestamp: Option<String>,
    /// Free Markdown text about this stamp.
    #[serde(default)]
    pub comment: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ActivityDeleteParams {
    /// The activity's grain-id.
    pub id: String,
    /// Also delete the activity's stamps. Without it, an activity that still
    /// has stamps is not deleted.
    #[serde(default)]
    pub with_stamps: bool,
}

/// What `activity_delete` answers with.
#[derive(Debug, Serialize, Deserialize)]
pub struct ActivityDeleted {
    /// The activity as it was.
    pub activity: Activity,
    /// How many of its stamps went with it.
    pub stamps_deleted: usize,
}

/// Omitted fields stay as they are.
#[derive(Debug, Default, Serialize, Deserialize, schemars::JsonSchema)]
pub struct StampUpdateParams {
    /// The stamp's grain-id.
    pub id: String,
    /// Move the stamp to another activity, by grain-id.
    #[serde(default)]
    pub activity: Option<String>,
    /// ISO 8601 timestamp with a timezone.
    #[serde(default)]
    pub timestamp: Option<String>,
    /// Free Markdown text. An empty string removes it.
    #[serde(default)]
    pub comment: Option<String>,
}

/// One stamp as `stamp_list` answers it: the stamp, and what its activity
/// means, so a count is never read without it.
#[derive(Debug, Serialize, Deserialize)]
pub struct StampEntry {
    #[serde(flatten)]
    pub stamp: Stamp,
    pub activity_title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<Target>,
}

/// One heatmap bucket as `heatmap` answers it, with its activity's title and
/// target and count status.
#[derive(Debug, Serialize, Deserialize)]
pub struct BucketEntry {
    #[serde(flatten)]
    pub bucket: HeatmapBucket,
    pub activity_title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<Target>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<Status>,
}

/// The filters `stamp_list` and `heatmap` share.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct QueryParams {
    #[serde(default)]
    pub activity: Option<String>,
    /// Start of the period (YYYY-MM-DD, inclusive).
    #[serde(default)]
    pub from: Option<String>,
    /// End of the period (YYYY-MM-DD, inclusive).
    #[serde(default)]
    pub to: Option<String>,
}

// ── methods ───────────────────────────────────────────────────────────────────

impl TallyMethods {
    pub fn new(tally: Tally) -> Self {
        Self { tally }
    }

    pub fn activity_add(&self, p: ActivityAddParams) -> Result<String, String> {
        (|| -> anyhow::Result<String> {
            let unit = parse_unit(p.unit.as_deref())?;
            let target = p
                .target
                .as_deref()
                .filter(|t| !t.trim().is_empty())
                .map(parse_target)
                .transpose()?;
            // An empty string is no colour, as on update.
            let color = p
                .color
                .as_deref()
                .filter(|c| !c.trim().is_empty())
                .map(parse_color)
                .transpose()?;
            let a = self
                .tally
                .create_activity(&p.title, unit, target, color, p.description)?;
            Ok(serde_json::to_string_pretty(&a)?)
        })()
        .map_err(|e| e.to_string())
    }

    pub fn activity_get(&self, p: IdParams) -> Result<String, String> {
        (|| -> anyhow::Result<String> {
            let a = self.tally.get_activity(parse_id(&p.id)?)?;
            Ok(serde_json::to_string_pretty(&a)?)
        })()
        .map_err(|e| e.to_string())
    }

    pub fn activity_update(&self, p: ActivityUpdateParams) -> Result<String, String> {
        (|| -> anyhow::Result<String> {
            let update = ActivityUpdate {
                title: p.title,
                unit: parse_unit(p.unit.as_deref())?,
                // An empty string removes the target.
                target: p
                    .target
                    .as_deref()
                    .map(|t| (!t.trim().is_empty()).then(|| parse_target(t)).transpose())
                    .transpose()?,
                // An empty string removes the colour.
                color: p
                    .color
                    .as_deref()
                    .map(|c| (!c.trim().is_empty()).then(|| parse_color(c)).transpose())
                    .transpose()?,
                description: p.description,
            };
            let a = self.tally.update_activity(parse_id(&p.id)?, update)?;
            Ok(serde_json::to_string_pretty(&a)?)
        })()
        .map_err(|e| e.to_string())
    }

    pub fn activity_delete(&self, p: ActivityDeleteParams) -> Result<String, String> {
        (|| -> anyhow::Result<String> {
            let id = parse_id(&p.id)?;
            let activity = self.tally.get_activity(id)?;
            let stamps = self.tally.delete_activity(id, p.with_stamps)?;
            Ok(serde_json::to_string_pretty(&ActivityDeleted {
                activity,
                stamps_deleted: stamps.len(),
            })?)
        })()
        .map_err(|e| e.to_string())
    }

    pub fn activity_list(&self, _: EmptyParams) -> Result<String, String> {
        (|| -> anyhow::Result<String> {
            let list = self.tally.list_activities()?;
            Ok(serde_json::to_string_pretty(&list)?)
        })()
        .map_err(|e| e.to_string())
    }

    pub fn stamp_add(&self, p: StampAddParams) -> Result<String, String> {
        (|| -> anyhow::Result<String> {
            let activity = parse_id(&p.activity)?;
            let ts = parse_timestamp(p.timestamp.as_deref())?;
            let s = self.tally.add_stamp(activity, ts, p.comment)?;
            Ok(serde_json::to_string_pretty(&s)?)
        })()
        .map_err(|e| e.to_string())
    }

    pub fn stamp_get(&self, p: IdParams) -> Result<String, String> {
        (|| -> anyhow::Result<String> {
            let s = self.tally.get_stamp(parse_id(&p.id)?)?;
            Ok(serde_json::to_string_pretty(&s)?)
        })()
        .map_err(|e| e.to_string())
    }

    pub fn stamp_update(&self, p: StampUpdateParams) -> Result<String, String> {
        (|| -> anyhow::Result<String> {
            let update = StampUpdate {
                activity: p.activity.as_deref().map(parse_id).transpose()?,
                timestamp: parse_timestamp(p.timestamp.as_deref())?,
                comment: p.comment,
            };
            let s = self.tally.update_stamp(parse_id(&p.id)?, update)?;
            Ok(serde_json::to_string_pretty(&s)?)
        })()
        .map_err(|e| e.to_string())
    }

    pub fn stamp_delete(&self, p: IdParams) -> Result<String, String> {
        (|| -> anyhow::Result<String> {
            let s = self.tally.delete_stamp(parse_id(&p.id)?)?;
            Ok(serde_json::to_string_pretty(&s)?)
        })()
        .map_err(|e| e.to_string())
    }

    pub fn stamp_list(&self, p: QueryParams) -> Result<String, String> {
        (|| -> anyhow::Result<String> {
            let (activity, from, to) = p.to_filters()?;
            let meanings = self.meanings()?;
            let list: Vec<StampEntry> = self
                .tally
                .list_stamps(activity, from, to)?
                .into_iter()
                .map(|stamp| {
                    let (activity_title, target) = meaning(&meanings, stamp.activity);
                    StampEntry {
                        stamp,
                        activity_title,
                        target,
                    }
                })
                .collect();
            Ok(serde_json::to_string_pretty(&list)?)
        })()
        .map_err(|e| e.to_string())
    }

    pub fn heatmap(&self, p: QueryParams) -> Result<String, String> {
        (|| -> anyhow::Result<String> {
            let (activity, from, to) = p.to_filters()?;
            let meanings = self.meanings()?;
            let buckets: Vec<BucketEntry> = self
                .tally
                .heatmap(activity, from, to)?
                .into_iter()
                .map(|bucket| {
                    let (activity_title, target) = meaning(&meanings, bucket.activity);
                    let status = target.map(|t| t.status(bucket.count));
                    BucketEntry {
                        bucket,
                        activity_title,
                        target,
                        status,
                    }
                })
                .collect();
            Ok(serde_json::to_string_pretty(&buckets)?)
        })()
        .map_err(|e| e.to_string())
    }
}

impl TallyMethods {
    /// Each readable activity's title and target, by id: what a log
    /// entry carries. See [`meaning`] for the ones that cannot be read.
    fn meanings(&self) -> anyhow::Result<HashMap<GrainId, (String, Option<Target>)>> {
        Ok(self
            .tally
            .list_activities()?
            .into_iter()
            .map(|a| (a.id, (a.title, a.target)))
            .collect())
    }
}

/// The title and target a log entry for `activity` carries.
///
/// An activity whose file exists but cannot be read (a hand edit gone wrong)
/// is missing from `meanings`; its stamps still count, under the activity's id
/// as title and no target, rather than vanishing from the agent's counts. Stamps
/// of a deleted activity never get here: the core leaves those out.
fn meaning(
    meanings: &HashMap<GrainId, (String, Option<Target>)>,
    activity: GrainId,
) -> (String, Option<Target>) {
    meanings
        .get(&activity)
        .cloned()
        .unwrap_or_else(|| (activity.to_string(), None))
}

impl QueryParams {
    /// Parse the shared filters. A bad value becomes the tool's error text.
    fn to_filters(
        &self,
    ) -> anyhow::Result<(
        Option<grain_id::GrainId>,
        Option<jiff::civil::Date>,
        Option<jiff::civil::Date>,
    )> {
        let activity = self.activity.as_deref().map(parse_id).transpose()?;
        let from = parse_date(self.from.as_deref(), "from")?;
        let to = parse_date(self.to.as_deref(), "to")?;
        Ok((activity, from, to))
    }
}

fn parse_id(s: &str) -> anyhow::Result<grain_id::GrainId> {
    s.parse()
        .map_err(|_| anyhow::anyhow!("invalid grain-id `{s}`"))
}

fn parse_unit(s: Option<&str>) -> anyhow::Result<Option<Unit>> {
    s.map(|s| Unit::parse_unit(s).map_err(anyhow::Error::msg))
        .transpose()
}

fn parse_color(s: &str) -> anyhow::Result<Color> {
    s.parse::<Color>().map_err(anyhow::Error::msg)
}

fn parse_target(s: &str) -> anyhow::Result<Target> {
    s.parse::<Target>().map_err(anyhow::Error::msg)
}

fn parse_timestamp(s: Option<&str>) -> anyhow::Result<Option<jiff::Timestamp>> {
    s.map(|s| {
        sapphire_tally_core::time::parse(s).map_err(|e| anyhow::anyhow!("invalid timestamp: {e}"))
    })
    .transpose()
}

fn parse_date(s: Option<&str>, field: &str) -> anyhow::Result<Option<jiff::civil::Date>> {
    s.map(|s| {
        s.parse::<jiff::civil::Date>()
            .map_err(|e| anyhow::anyhow!("invalid {field} date: {e}"))
    })
    .transpose()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn methods() -> (tempfile::TempDir, TallyMethods) {
        let dir = tempfile::tempdir().unwrap();
        let tally = Tally::open(dir.path().to_path_buf()).unwrap();
        (dir, TallyMethods::new(tally))
    }

    #[test]
    fn add_activity_then_stamp_then_heatmap_roundtrip() {
        let (_dir, m) = methods();
        let created = m
            .activity_add(ActivityAddParams {
                title: "タピオカミルクティー".into(),
                unit: Some("week".into()),
                target: None,
                color: None,
                description: None,
            })
            .unwrap();
        let created: serde_json::Value = serde_json::from_str(&created).unwrap();
        let id = created["id"].as_str().unwrap().to_string();
        assert_eq!(created["unit"], "week");

        m.stamp_add(StampAddParams {
            activity: id.clone(),
            timestamp: Some("2026-09-05T09:12:00+09:00".into()),
            comment: Some("おいしい".into()),
        })
        .unwrap();

        let list = m
            .stamp_list(QueryParams {
                activity: Some(id.clone()),
                from: None,
                to: None,
            })
            .unwrap();
        let list: serde_json::Value = serde_json::from_str(&list).unwrap();
        assert_eq!(list.as_array().unwrap().len(), 1);
        assert_eq!(list[0]["comment"], "おいしい");

        let heat = m
            .heatmap(QueryParams {
                activity: Some(id),
                from: None,
                to: None,
            })
            .unwrap();
        let heat: serde_json::Value = serde_json::from_str(&heat).unwrap();
        assert_eq!(heat.as_array().unwrap()[0]["count"], 1);
        assert_eq!(heat.as_array().unwrap()[0]["period"], "2026-W36");
    }

    #[test]
    fn get_and_update_roundtrip() {
        let (_dir, m) = methods();
        let created = m
            .activity_add(ActivityAddParams {
                title: "筋トレ".into(),
                unit: None,
                target: None,
                color: None,
                description: Some("毎朝".into()),
            })
            .unwrap();
        let id = serde_json::from_str::<serde_json::Value>(&created).unwrap()["id"]
            .as_str()
            .unwrap()
            .to_string();
        m.activity_update(ActivityUpdateParams {
            id: id.clone(),
            target: None,
            color: None,
            unit: Some("month".into()),
            ..Default::default()
        })
        .unwrap();
        let got: serde_json::Value =
            serde_json::from_str(&m.activity_get(IdParams { id: id.clone() }).unwrap()).unwrap();
        assert_eq!(got["unit"], "month");
        assert_eq!(got["description"], "毎朝");

        let stamp = m
            .stamp_add(StampAddParams {
                activity: id,
                timestamp: Some("2026-09-05T09:12:00+09:00".into()),
                comment: Some("30回".into()),
            })
            .unwrap();
        let stamp_id = serde_json::from_str::<serde_json::Value>(&stamp).unwrap()["id"]
            .as_str()
            .unwrap()
            .to_string();
        m.stamp_update(StampUpdateParams {
            id: stamp_id.clone(),
            comment: Some(String::new()),
            ..Default::default()
        })
        .unwrap();
        let got: serde_json::Value =
            serde_json::from_str(&m.stamp_get(IdParams { id: stamp_id }).unwrap()).unwrap();
        assert!(got.get("comment").is_none(), "{got}");
        assert_eq!(got["timestamp"], "2026-09-05T09:12:00+09:00");
    }

    #[test]
    fn delete_refuses_an_activity_with_stamps_unless_told() {
        let (_dir, m) = methods();
        let created = m
            .activity_add(ActivityAddParams {
                title: "筋トレ".into(),
                unit: None,
                target: None,
                color: None,
                description: None,
            })
            .unwrap();
        let id = serde_json::from_str::<serde_json::Value>(&created).unwrap()["id"]
            .as_str()
            .unwrap()
            .to_string();
        let stamp = m
            .stamp_add(StampAddParams {
                activity: id.clone(),
                timestamp: None,
                comment: None,
            })
            .unwrap();
        let stamp_id = serde_json::from_str::<serde_json::Value>(&stamp).unwrap()["id"]
            .as_str()
            .unwrap()
            .to_string();

        let err = m
            .activity_delete(ActivityDeleteParams {
                id: id.clone(),
                with_stamps: false,
            })
            .unwrap_err();
        assert!(err.contains("with_stamps"), "{err}");

        let deleted: ActivityDeleted = serde_json::from_str(
            &m.activity_delete(ActivityDeleteParams {
                id: id.clone(),
                with_stamps: true,
            })
            .unwrap(),
        )
        .unwrap();
        assert_eq!(deleted.stamps_deleted, 1);
        assert_eq!(deleted.activity.title, "筋トレ");
        let err = m.stamp_delete(IdParams { id: stamp_id }).unwrap_err();
        assert!(err.contains("not found"), "{err}");
    }

    #[test]
    fn stamp_on_unknown_activity_returns_tool_error() {
        let (_dir, m) = methods();
        let err = m
            .stamp_add(StampAddParams {
                activity: "012atvw".into(),
                timestamp: Some("2026-09-05T09:12:00+09:00".into()),
                comment: None,
            })
            .unwrap_err();
        assert!(err.contains("not found"), "{err}");
    }

    #[test]
    fn invalid_unit_returns_tool_error() {
        let (_dir, m) = methods();
        let err = m
            .activity_add(ActivityAddParams {
                title: "x".into(),
                unit: Some("fortnight".into()),
                target: None,
                color: None,
                description: None,
            })
            .unwrap_err();
        assert!(err.contains("fortnight"), "{err}");
    }

    fn add(m: &TallyMethods, title: &str, target: Option<&str>) -> String {
        let v: serde_json::Value = serde_json::from_str(
            &m.activity_add(ActivityAddParams {
                title: title.into(),
                unit: None,
                target: target.map(Into::into),
                color: None,
                description: None,
            })
            .unwrap(),
        )
        .unwrap();
        v["id"].as_str().unwrap().to_string()
    }

    fn stamp(m: &TallyMethods, activity: &str) {
        m.stamp_add(StampAddParams {
            activity: activity.into(),
            timestamp: Some("2026-09-05T09:00:00+09:00".into()),
            comment: None,
        })
        .unwrap();
    }

    fn all() -> QueryParams {
        QueryParams {
            activity: None,
            from: None,
            to: None,
        }
    }

    #[test]
    fn log_entries_carry_each_activitys_title_and_target() {
        let (_dir, m) = methods();
        let tea = add(&m, "タピオカミルクティー", Some("3-below"));
        let gym = add(&m, "筋トレ", None);
        stamp(&m, &tea);
        stamp(&m, &gym);
        let stamps: Vec<serde_json::Value> =
            serde_json::from_str(&m.stamp_list(all()).unwrap()).unwrap();
        let heat: Vec<serde_json::Value> =
            serde_json::from_str(&m.heatmap(all()).unwrap()).unwrap();
        for entries in [&stamps, &heat] {
            let of = |id: &str| {
                entries
                    .iter()
                    .find(|e| e["activity"] == id)
                    .unwrap()
                    .clone()
            };
            assert_eq!(of(&tea)["activity_title"], "タピオカミルクティー");
            assert_eq!(of(&tea)["target"]["bound"], "below");
            assert_eq!(of(&gym)["activity_title"], "筋トレ");
            assert!(of(&gym).get("target").is_none());
        }
    }

    #[test]
    fn decorated_stamp_list_still_reads_as_stamps() {
        let (_dir, m) = methods();
        let tea = add(&m, "タピオカミルクティー", Some("3-below"));
        stamp(&m, &tea);
        let stamps: Vec<sapphire_tally_core::Stamp> =
            serde_json::from_str(&m.stamp_list(all()).unwrap()).unwrap();
        assert_eq!(stamps.len(), 1);
    }

    #[test]
    fn invalid_target_returns_tool_error() {
        let (_dir, m) = methods();
        let err = m
            .activity_add(ActivityAddParams {
                title: "x".into(),
                unit: None,
                target: Some("less".into()),
                color: None,
                description: None,
            })
            .unwrap_err();
        assert!(err.contains("less"), "{err}");
    }

    #[test]
    fn stamps_of_an_unreadable_activity_are_still_counted() {
        let (dir, m) = methods();
        let tea = add(&m, "タピオカミルクティー", Some("3-below"));
        stamp(&m, &tea);
        let path = sapphire_tally_core::store::activity_path(dir.path(), tea.parse().unwrap());
        let text = std::fs::read_to_string(&path).unwrap();
        std::fs::write(&path, text.replace("bound: below", "bound: less")).unwrap();

        let stamps: Vec<serde_json::Value> =
            serde_json::from_str(&m.stamp_list(all()).unwrap()).unwrap();
        let heat: Vec<serde_json::Value> =
            serde_json::from_str(&m.heatmap(all()).unwrap()).unwrap();
        for entries in [&stamps, &heat] {
            assert_eq!(entries.len(), 1, "{entries:?}");
            assert_eq!(entries[0]["activity_title"], tea.as_str());
            assert!(entries[0].get("target").is_none());
            assert!(entries[0].get("status").is_none());
        }
    }

    fn get(m: &TallyMethods, id: &str) -> serde_json::Value {
        serde_json::from_str(&m.activity_get(IdParams { id: id.into() }).unwrap()).unwrap()
    }

    #[test]
    fn color_is_set_and_cleared_over_mcp() {
        let (_dir, m) = methods();
        let created: serde_json::Value = serde_json::from_str(
            &m.activity_add(ActivityAddParams {
                title: "筋トレ".into(),
                unit: None,
                target: None,
                color: Some("green".into()),
                description: None,
            })
            .unwrap(),
        )
        .unwrap();
        assert_eq!(created["color"], "green");
        let id = created["id"].as_str().unwrap().to_string();
        let update = |color: Option<&str>| {
            m.activity_update(ActivityUpdateParams {
                id: id.clone(),
                color: color.map(Into::into),
                ..Default::default()
            })
            .unwrap();
        };
        update(Some("oklch(0.15 255)"));
        assert_eq!(get(&m, &id)["color"], "oklch(0.15 255)");
        update(None);
        assert_eq!(get(&m, &id)["color"], "oklch(0.15 255)");
        update(Some(""));
        assert!(get(&m, &id).get("color").is_none());
    }

    #[test]
    fn invalid_color_returns_tool_error() {
        let (_dir, m) = methods();
        let err = m
            .activity_add(ActivityAddParams {
                title: "x".into(),
                unit: None,
                target: None,
                color: Some("pink".into()),
                description: None,
            })
            .unwrap_err();
        assert!(err.contains("pink"), "{err}");
    }

    #[test]
    fn an_empty_color_on_add_means_none() {
        let (_dir, m) = methods();
        let created: serde_json::Value = serde_json::from_str(
            &m.activity_add(ActivityAddParams {
                title: "x".into(),
                unit: None,
                target: None,
                color: Some(" ".into()),
                description: None,
            })
            .unwrap(),
        )
        .unwrap();
        assert!(created.get("color").is_none(), "{created}");
    }

    #[test]
    fn a_heatmap_bucket_carries_a_target_and_how_its_count_reads_against_it() {
        let (_dir, m) = methods();
        let tea = add(&m, "タピオカミルクティー", Some("3-below"));
        let gym = add(&m, "筋トレ", Some("2-above"));
        let only = add(&m, "ヨーヨー", None);
        for _ in 0..4 {
            stamp(&m, &tea);
        }
        stamp(&m, &gym);
        stamp(&m, &only);
        let met = add(&m, "散歩", Some("1-above"));
        stamp(&m, &met);
        let heat: Vec<serde_json::Value> =
            serde_json::from_str(&m.heatmap(all()).unwrap()).unwrap();
        let of = |id: &str| heat.iter().find(|e| e["activity"] == id).unwrap().clone();
        assert_eq!(of(&tea)["activity_title"], "タピオカミルクティー");
        assert_eq!(of(&tea)["target"]["bound"], "below");
        assert_eq!(of(&tea)["status"], "exceeded");
        assert_eq!(of(&gym)["target"]["times"], 2);
        assert_eq!(of(&gym)["status"], "under");
        assert_eq!(of(&met)["status"], "met");
        assert!(of(&only).get("target").is_none(), "{:?}", of(&only));
        assert!(of(&only).get("status").is_none(), "{:?}", of(&only));
    }

    #[test]
    fn a_stamp_carries_its_activitys_target_and_no_status() {
        let (_dir, m) = methods();
        let tea = add(&m, "タピオカミルクティー", Some("3-below"));
        stamp(&m, &tea);
        let stamps: Vec<serde_json::Value> =
            serde_json::from_str(&m.stamp_list(all()).unwrap()).unwrap();
        assert_eq!(stamps[0]["target"]["times"], 3);
        assert!(stamps[0].get("status").is_none(), "{stamps:?}");
    }

    #[test]
    fn target_is_set_and_cleared_over_mcp() {
        let (_dir, m) = methods();
        let id = add(&m, "筋トレ", Some("1-above"));
        let update = |target: Option<&str>| {
            m.activity_update(ActivityUpdateParams {
                id: id.clone(),
                target: target.map(Into::into),
                ..Default::default()
            })
        };
        update(None).unwrap();
        assert_eq!(get(&m, &id)["target"]["times"], 1);
        update(Some(" 2-BELOW ")).unwrap();
        assert_eq!(get(&m, &id)["target"]["bound"], "below");
        assert_eq!(get(&m, &id)["target"]["times"], 2);
        let err = update(Some("less")).unwrap_err();
        assert!(err.contains("less"), "{err}");
        assert_eq!(get(&m, &id)["target"]["times"], 2);
        update(Some(" ")).unwrap();
        assert!(get(&m, &id).get("target").is_none());
        let only = add(&m, "ヨーヨー", Some(""));
        assert!(get(&m, &only).get("target").is_none());
    }
}
