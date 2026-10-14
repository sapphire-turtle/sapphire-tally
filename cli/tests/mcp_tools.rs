//! The tally's MCP tools against a running tally server, over IPC: the path
//! `sapphire-tally mcp` takes, and the one `/mcp` takes in-process.
//!
//! What these pin is the tally's half of the wiring — that `serve::build`
//! registers the tally's methods, that the tools reach them, and that they
//! work on the workspace the server has selected. The transports, the token
//! check on `/mcp` and the framework's own tools are the framework's to test.
//!
//! One test: `TALLY_CTX`'s directories are first-writer-wins per process.

use std::time::Duration;

use sapphire_framework::backend::protocol as proto;
use sapphire_framework::ipc::{self, ClientInfo, Endpoint};
use sapphire_framework::mcp::ToolContext;
use sapphire_framework::mcp::rmcp::handler::server::wrapper::Parameters;
use sapphire_tally::serve;
use sapphire_tally_core::{AppKind, TALLY_CTX};
use sapphire_tally_mcp::TallyTools;
use sapphire_tally_mcp::methods::{ActivityAddParams, EmptyParams, QueryParams, StampAddParams};

async fn wait_until_listening(endpoint: &Endpoint) {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    while !ipc::probe(endpoint).await.unwrap() {
        assert!(
            tokio::time::Instant::now() < deadline,
            "the server never started listening"
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
}

async fn client(endpoint: &Endpoint) -> ipc::Client {
    let info = ClientInfo {
        kind: "test".into(),
        version: "0.0.0".into(),
        api: proto::API_VERSION,
        app_api: None,
        pid: std::process::id(),
    };
    ipc::connect_or_absent(endpoint, TALLY_CTX.app_name, info)
        .await
        .unwrap()
        .expect("the server is listening")
        .0
}

#[tokio::test(flavor = "multi_thread")]
async fn the_tools_work_on_the_servers_workspace_over_ipc() {
    let tmp = tempfile::tempdir().unwrap();
    for cat in ["CACHE", "DATA", "CONFIG"] {
        // SAFETY: the only test in this binary, before any thread reads them.
        unsafe {
            std::env::set_var(
                format!("SAPPHIRE_TALLY_{cat}_DIR"),
                tmp.path().join(cat.to_lowercase()),
            )
        };
    }
    sapphire_tally_core::init_app_context(AppKind::Server);

    let endpoint = Endpoint::in_dir(TALLY_CTX.app_name, tmp.path().join("run"));
    std::fs::create_dir_all(tmp.path().join("run")).unwrap();
    let server = serve::build().endpoint(endpoint.clone());
    let task = tokio::spawn(async move { server.run().await.unwrap() });
    wait_until_listening(&endpoint).await;

    let tools = TallyTools::new(ToolContext::dial(
        TALLY_CTX.app_name,
        serve::VERSION,
        Some(sapphire_tally_mcp::APP_API),
        endpoint.clone(),
    ));

    // No workspace yet: the tally's own message, not the IPC layer's.
    let err = tools
        .activity_list(Parameters(EmptyParams {}))
        .await
        .expect_err("nothing is selected");
    assert!(err.contains("workspace select"), "got: {err}");
    assert!(
        !err.contains("remote error"),
        "the IPC framing leaked: {err}"
    );

    let data = tmp.path().join("tally");
    std::fs::create_dir_all(&data).unwrap();
    let c = client(&endpoint).await;
    let _: proto::WorkspaceInitResult = c
        .call(
            proto::WORKSPACE_INIT,
            proto::WorkspaceInitParams { dir: data.clone() },
        )
        .await
        .unwrap();

    let created = tools
        .activity_add(Parameters(ActivityAddParams {
            title: "practice".into(),
            unit: None,
        }))
        .await
        .expect("activity_add");
    let created: serde_json::Value = serde_json::from_str(&created).unwrap();
    let id = created["id"].as_str().unwrap().to_string();

    tools
        .stamp_add(Parameters(StampAddParams {
            activity: id.clone(),
            timestamp: Some("2026-09-05T09:12:00+09:00".into()),
            comment: None,
        }))
        .await
        .expect("stamp_add");
    assert!(
        std::fs::read_dir(data.join("stamps"))
            .unwrap()
            .next()
            .is_some(),
        "the stamp is in the selected workspace"
    );

    let heat = tools
        .heatmap(Parameters(QueryParams {
            activity: Some(id),
            from: None,
            to: None,
        }))
        .await
        .expect("heatmap");
    let heat: serde_json::Value = serde_json::from_str(&heat).unwrap();
    assert_eq!(heat[0]["period"], "2026-09-05");
    assert_eq!(heat[0]["count"], 1);

    // A refused write comes back as the tally's reason.
    let err = tools
        .stamp_add(Parameters(StampAddParams {
            activity: "012atvw".into(),
            timestamp: None,
            comment: None,
        }))
        .await
        .expect_err("unknown activity");
    assert!(err.contains("not found"), "got: {err}");
    assert!(
        !err.contains("remote error"),
        "the IPC framing leaked: {err}"
    );

    let _: serde_json::Value = c
        .call(ipc::SHUTDOWN_METHOD, serde_json::json!({}))
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(10), task)
        .await
        .expect("the server stopped")
        .unwrap();
}
