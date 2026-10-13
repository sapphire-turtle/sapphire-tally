//! The desktop's link to the tally's methods on the running app server.
//!
//! The framework's `FrameworkClient` covers the framework's half (status,
//! workspaces, sync); the tally's own `sapphire-tally.*` methods go over a
//! connection of this app's own. It is dialled on the first call and again
//! after it drops — the server may start after the window opens, or restart
//! while it is open — and a call is never retried, so a write is never
//! applied twice.

use std::sync::Arc;

use sapphire_framework::backend::protocol::API_VERSION;
use sapphire_framework::ipc::{self, Client, ClientInfo, Endpoint};
use serde::Serialize;
use serde::de::DeserializeOwned;

/// What this client reports in the IPC handshake.
const CLIENT_KIND: &str = "desktop";

pub struct TallyClient {
    app: &'static str,
    version: &'static str,
    endpoint: Endpoint,
    held: tokio::sync::Mutex<Option<Arc<Client>>>,
}

impl TallyClient {
    pub fn new(app: &'static str, version: &'static str, endpoint: Endpoint) -> TallyClient {
        TallyClient {
            app,
            version,
            endpoint,
            held: tokio::sync::Mutex::new(None),
        }
    }

    /// Call `method` and return its text: a confirmation for a write, JSON
    /// for a read. The error is a message to show as it is.
    pub async fn call(&self, method: &str, params: impl Serialize) -> Result<String, String> {
        let client = self.client().await?;
        match client.call::<_, String>(method, params).await {
            Ok(text) => Ok(text),
            Err(ipc::Error::Rpc(rpc)) => Err(rpc.message),
            Err(ipc::Error::Closed | ipc::Error::Io(_)) => {
                self.forget(&client).await;
                Err(format!(
                    "the connection to the {} server was lost; try again",
                    self.app
                ))
            }
            Err(err) => Err(err.to_string()),
        }
    }

    /// Call a read method and parse the JSON it answers with.
    pub async fn read<R: DeserializeOwned>(
        &self,
        method: &str,
        params: impl Serialize,
    ) -> Result<R, String> {
        let text = self.call(method, params).await?;
        serde_json::from_str(&text).map_err(|e| format!("unexpected answer to {method}: {e}"))
    }

    async fn client(&self) -> Result<Arc<Client>, String> {
        let mut held = self.held.lock().await;
        if let Some(client) = held.as_ref().filter(|c| !c.is_closed()) {
            return Ok(Arc::clone(client));
        }
        let info = ClientInfo {
            kind: CLIENT_KIND.to_owned(),
            version: self.version.to_owned(),
            api: API_VERSION,
            app_api: Some(sapphire_tally_mcp::APP_API),
            pid: std::process::id(),
        };
        let (client, _) = ipc::connect_or_absent(&self.endpoint, self.app, info)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| {
                format!(
                    "no {} server is running; start it from the Sync tab, or with `{} serve`",
                    self.app, self.app
                )
            })?;
        let client = Arc::new(client);
        *held = Some(Arc::clone(&client));
        Ok(client)
    }

    async fn forget(&self, lost: &Arc<Client>) {
        let mut held = self.held.lock().await;
        if held.as_ref().is_some_and(|h| Arc::ptr_eq(h, lost)) {
            *held = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use sapphire_framework::backend::protocol as proto;
    use sapphire_tally_core::{Activity, AppKind, Stamp, TALLY_CTX};
    use sapphire_tally_mcp::methods::{
        ActivityDeleteParams, ActivityDeleted, EmptyParams, IdParams, QueryParams, names,
    };

    use super::*;
    use crate::form::{ActivityForm, StampForm};

    /// The desktop's reads and writes against a running tally server: that
    /// what the methods answer parses into the types the view shows, and
    /// that a form's parameters are what the server accepts.
    ///
    /// One test: `TALLY_CTX`'s directories are first-writer-wins per process.
    #[tokio::test(flavor = "multi_thread")]
    async fn the_desktop_reads_and_writes_the_servers_tally() {
        let tmp = tempfile::tempdir().unwrap();
        for cat in ["CACHE", "DATA", "CONFIG"] {
            // SAFETY: the only test in this binary that touches the
            // environment, before the server starts.
            unsafe {
                std::env::set_var(
                    format!("SAPPHIRE_TALLY_{cat}_DIR"),
                    tmp.path().join(cat.to_lowercase()),
                )
            };
        }
        sapphire_tally_core::init_app_context(AppKind::Server);
        std::fs::create_dir_all(tmp.path().join("run")).unwrap();
        let endpoint = Endpoint::in_dir(TALLY_CTX.app_name, tmp.path().join("run"));

        let client = TallyClient::new(TALLY_CTX.app_name, "0.0.0", endpoint.clone());
        let err = client
            .call(names::ACTIVITY_LIST, EmptyParams {})
            .await
            .unwrap_err();
        assert!(
            err.contains("no sapphire-tally server is running"),
            "got: {err}"
        );

        let server = sapphire_tally::serve::build().endpoint(endpoint.clone());
        let task = tokio::spawn(async move { server.run().await.unwrap() });
        let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
        while !ipc::probe(&endpoint).await.unwrap() {
            assert!(
                tokio::time::Instant::now() < deadline,
                "the server never listened"
            );
            tokio::time::sleep(Duration::from_millis(20)).await;
        }

        let data = tmp.path().join("tally");
        std::fs::create_dir_all(&data).unwrap();
        let _: proto::WorkspaceInitResult = client
            .client()
            .await
            .unwrap()
            .call(
                proto::WORKSPACE_INIT,
                proto::WorkspaceInitParams { dir: data.clone() },
            )
            .await
            .unwrap();

        let form = ActivityForm {
            title: "筋トレ".into(),
            description: "毎朝".into(),
            ..ActivityForm::default()
        };
        let activity: Activity = client
            .read(names::ACTIVITY_ADD, form.to_add_params().unwrap())
            .await
            .expect("the activity form's parameters are what the server takes");

        let now = chrono::DateTime::parse_from_rfc3339("2026-09-05T09:12:34+09:00").unwrap();
        let mut form = StampForm::new(Some(activity.id), now);
        form.comment = "30回".into();
        let stamp: Stamp = client
            .read(names::STAMP_ADD, form.to_add_params().unwrap())
            .await
            .expect("the stamp form's parameters are what the server takes");
        assert_eq!(stamp.timestamp.to_rfc3339(), "2026-09-05T09:12:00+09:00");

        let mut form = StampForm::edit(&stamp);
        form.comment.clear();
        let changed: Stamp = client
            .read(
                names::STAMP_UPDATE,
                form.to_update_params(stamp.id).unwrap(),
            )
            .await
            .expect("stamp_update");
        assert_eq!(changed.comment, None, "an emptied comment is removed");
        assert_eq!(changed.timestamp, stamp.timestamp);

        let mut form = ActivityForm::edit(&activity);
        form.title = "筋トレ（朝）".into();
        let changed: Activity = client
            .read(
                names::ACTIVITY_UPDATE,
                form.to_update_params(activity.id).unwrap(),
            )
            .await
            .expect("activity_update");
        assert_eq!(changed.description.as_deref(), Some("毎朝"));

        let activities: Vec<Activity> = client
            .read(names::ACTIVITY_LIST, EmptyParams {})
            .await
            .expect("activities parse");
        assert_eq!(activities[0].title, "筋トレ（朝）");
        let stamps: Vec<Stamp> = client
            .read(
                names::STAMP_LIST,
                QueryParams {
                    activity: None,
                    from: Some("2026-09-01".into()),
                    to: Some("2026-09-30".into()),
                },
            )
            .await
            .expect("stamps parse");
        assert_eq!(stamps.len(), 1);

        // What the delete confirmations send.
        let deleted: Stamp = client
            .read(
                names::STAMP_DELETE,
                IdParams {
                    id: stamp.id.to_string(),
                },
            )
            .await
            .expect("stamp_delete");
        assert_eq!(deleted.id, stamp.id);
        let other: Stamp = client
            .read(
                names::STAMP_ADD,
                StampForm::new(Some(activity.id), now)
                    .to_add_params()
                    .unwrap(),
            )
            .await
            .unwrap();
        let deleted: ActivityDeleted = client
            .read(
                names::ACTIVITY_DELETE,
                ActivityDeleteParams {
                    id: activity.id.to_string(),
                    with_stamps: true,
                },
            )
            .await
            .expect("activity_delete");
        assert_eq!(deleted.stamps_deleted, 1);
        let err = client
            .call(
                names::STAMP_DELETE,
                IdParams {
                    id: other.id.to_string(),
                },
            )
            .await
            .unwrap_err();
        assert!(err.contains("not found"), "went with the activity: {err}");

        let _: serde_json::Value = client
            .client()
            .await
            .unwrap()
            .call(ipc::SHUTDOWN_METHOD, serde_json::json!({}))
            .await
            .unwrap();
        tokio::time::timeout(Duration::from_secs(10), task)
            .await
            .expect("the server stopped")
            .unwrap();
    }
}
