//! The tally's methods on its app server: what the MCP tools call.
//!
//! Every tool reaches the tally the way the framework's own tools reach the
//! workspace — over the app server's IPC, as `sapphire-tally.<tool>` — so one
//! implementation serves the `mcp` subcommand (stdio, dialling the running
//! server) and `/mcp` on the server's HTTP listener alike. [`register`] adds
//! the methods to the server's router; [`crate::tools`] is the other end.

use std::sync::Arc;

use sapphire_framework::ipc::{RequestCtx, Router, RpcError};
use sapphire_tally_core::{Tally, Unit};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

/// Where a call finds its tally: asked once per call.
///
/// The app server's tally is whatever workspace it has selected, which
/// `workspace select` can change underneath it; an error refuses the call.
pub type StateSource = Arc<dyn Fn() -> std::io::Result<Tally> + Send + Sync>;

/// The tally's methods, `sapphire-tally.<tool>`.
pub mod names {
    pub const ACTIVITY_ADD: &str = "sapphire-tally.activity_add";
    pub const ACTIVITY_LIST: &str = "sapphire-tally.activity_list";
    pub const STAMP_ADD: &str = "sapphire-tally.stamp_add";
    pub const STAMP_LIST: &str = "sapphire-tally.stamp_list";
    pub const HEATMAP: &str = "sapphire-tally.heatmap";
}

/// The version of the methods in [`names`], reported in the IPC handshake.
///
/// Bump it when a method's parameters or meaning change incompatibly: a
/// client built against another version is refused at connect time rather
/// than mid-call.
pub const APP_API: u32 = 1;

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
        names::ACTIVITY_LIST,
        &source,
        TallyMethods::activity_list,
    );
    let router = route(router, names::STAMP_ADD, &source, TallyMethods::stamp_add);
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
    /// Display granularity: day/week/month (day when omitted).
    #[serde(default)]
    pub unit: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
pub struct StampAddParams {
    pub activity: String,
    /// ISO 8601 timestamp with a timezone. The server's current time when
    /// omitted.
    #[serde(default)]
    pub timestamp: Option<String>,
    #[serde(default)]
    pub comment: Option<String>,
}

/// The filters `stamp_list` and `heatmap` share.
#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
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
            let unit = match p.unit.as_deref() {
                Some(s) => Some(Unit::parse_unit(s).map_err(anyhow::Error::msg)?),
                None => None,
            };
            let a = self.tally.create_activity(&p.title, unit)?;
            Ok(serde_json::to_string_pretty(&a)?)
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
            let activity = p
                .activity
                .parse()
                .map_err(|_| anyhow::anyhow!("invalid grain-id `{}`", p.activity))?;
            let ts = match p.timestamp.as_deref() {
                Some(s) => Some(
                    chrono::DateTime::parse_from_rfc3339(s)
                        .map_err(|e| anyhow::anyhow!("invalid timestamp: {e}"))?,
                ),
                None => None,
            };
            let s = self.tally.add_stamp(activity, ts, p.comment)?;
            Ok(serde_json::to_string_pretty(&s)?)
        })()
        .map_err(|e| e.to_string())
    }

    pub fn stamp_list(&self, p: QueryParams) -> Result<String, String> {
        (|| -> anyhow::Result<String> {
            let (activity, from, to) = p.to_filters()?;
            let list = self.tally.list_stamps(activity, from, to)?;
            Ok(serde_json::to_string_pretty(&list)?)
        })()
        .map_err(|e| e.to_string())
    }

    pub fn heatmap(&self, p: QueryParams) -> Result<String, String> {
        (|| -> anyhow::Result<String> {
            let (activity, from, to) = p.to_filters()?;
            let buckets = self.tally.heatmap(activity, from, to)?;
            Ok(serde_json::to_string_pretty(&buckets)?)
        })()
        .map_err(|e| e.to_string())
    }
}

impl QueryParams {
    /// Parse the shared filters. A bad value becomes the tool's error text.
    fn to_filters(
        &self,
    ) -> anyhow::Result<(
        Option<grain_id::GrainId>,
        Option<chrono::NaiveDate>,
        Option<chrono::NaiveDate>,
    )> {
        let activity = self
            .activity
            .as_deref()
            .map(|s| {
                s.parse::<grain_id::GrainId>()
                    .map_err(|_| anyhow::anyhow!("invalid grain-id `{s}`"))
            })
            .transpose()?;
        let from = parse_date(self.from.as_deref(), "from")?;
        let to = parse_date(self.to.as_deref(), "to")?;
        Ok((activity, from, to))
    }
}

fn parse_date(s: Option<&str>, field: &str) -> anyhow::Result<Option<chrono::NaiveDate>> {
    s.map(|s| {
        chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
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
            })
            .unwrap_err();
        assert!(err.contains("fortnight"), "{err}");
    }
}
