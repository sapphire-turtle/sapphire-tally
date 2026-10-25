use std::net::SocketAddr;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use sapphire_tally::serve;

fn addr(s: &str) -> SocketAddr {
    s.parse().expect("addr")
}

#[test]
fn loopback_needs_no_allowed_host() {
    serve::check_exposure(addr("127.0.0.1:3174"), &[])
        .expect("a loopback bind is complete on its own");
}

#[test]
fn a_wide_bind_without_an_allowed_host_is_refused() {
    let err = serve::check_exposure(addr("0.0.0.0:3174"), &[])
        .expect_err("a wide bind with no allowlist serves 403 to everyone");
    let msg = err.to_string();
    assert!(
        msg.contains("allowed_hosts"),
        "the error must name the setting that fixes it, got: {msg}"
    );
}

#[test]
fn a_wide_bind_with_an_allowed_host_is_accepted() {
    serve::check_exposure(addr("0.0.0.0:3174"), &["tally.example.net".to_string()])
        .expect("a named host makes a wide bind usable");
}

#[test]
fn a_blank_allowed_host_does_not_count() {
    serve::check_exposure(addr("0.0.0.0:3174"), &["  ".to_string()])
        .expect_err("whitespace is not a hostname");
}

/// The check runs where the settings are read: a `config.toml` that turns
/// `/mcp` on over a wide bind with no allowlist stops `serve` before it binds.
#[test]
fn serve_refuses_a_wide_bind_without_an_allowed_host() {
    let tmp = tempfile::tempdir().unwrap();
    // `SAPPHIRE_TALLY_CONFIG_DIR` is the root the app's own directory goes in.
    let config = tmp.path().join("config");
    std::fs::create_dir_all(config.join("sapphire-tally")).unwrap();
    std::fs::write(
        config.join("sapphire-tally").join("config.toml"),
        "[http]\nlisten = \"0.0.0.0:3174\"\n\n[http.mcp]\nenabled = true\n",
    )
    .unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_sapphire-tally"))
        .arg("serve")
        .env("SAPPHIRE_TALLY_CONFIG_DIR", &config)
        .env("SAPPHIRE_TALLY_DATA_DIR", tmp.path().join("data"))
        .env("SAPPHIRE_TALLY_CACHE_DIR", tmp.path().join("cache"))
        .env("SAPPHIRE_RUNTIME_DIR", tmp.path().join("run"))
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("run");

    let deadline = Instant::now() + Duration::from_secs(30);
    while child.try_wait().unwrap().is_none() {
        if Instant::now() > deadline {
            child.kill().unwrap();
            panic!("serve started with a wide bind and no allowed_hosts");
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    let out = child.wait_with_output().unwrap();
    assert!(!out.status.success());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("allowed_hosts"), "got: {err}");
}
