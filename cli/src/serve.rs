//! The app server: what `sapphire-tally serve` runs.
//!
//! The framework's [`AppServer`] owns the IPC socket, the current workspace
//! (`workspace init|select|sync|show|move|remove`), the settings
//! (`config list|get|set|unset`), signals, restarts and the log file. This
//! module adds what is the tally's own:
//!
//! - **sync**, through the host's `sapphire-bridge`. A bridge that is not
//!   running is not fatal: the server serves its workspace without sync and
//!   picks the bridge up once it starts, or again after it restarts.
//! - **the tally's methods** (`sapphire-tally.*`), which its MCP tools call:
//!   the framework serves those tools on stdio (`sapphire-tally mcp`) and, when
//!   `config.toml`'s `[http.mcp] enabled` turns it on, at `/mcp` on the
//!   server's HTTP listener.

use std::io;
use std::net::SocketAddr;
use std::sync::Arc;

use sapphire_framework::server::{
    AppFiles, AppServer, AppSettings, FieldError, HttpConfig, ReleaseSource, SERVER_CONFIG_FILE,
    ServerConfig, SettingDef, Settings, Staged, StatusRow, WorkspaceHost,
};
use sapphire_tally_core::{TALLY_CTX, Tally};

/// This binary's version, reported in every handshake.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Where the HTTP listener binds unless `config.toml`'s `[http] listen` says
/// otherwise: loopback, on the port the MVP's MCP server used.
pub const DEFAULT_HTTP_ADDR: &str = "127.0.0.1:3174";

/// The tally's settings: no keys of its own yet, and the one check of the
/// framework's `[http]` table that the framework does not make.
#[derive(Clone)]
pub struct TallySettings;

impl AppSettings for TallySettings {
    fn defs(&self) -> Vec<SettingDef> {
        Vec::new()
    }

    /// Refuse an `[http]` table that would serve 403 to every `/mcp` client
    /// ([`check_exposure`]). The framework runs this when the server starts,
    /// so such a file stops the start.
    fn validate(&self, files: &AppFiles, staged: &Staged) -> Result<(), Vec<FieldError>> {
        // A file that cannot be read or parsed, or an `[http]` table of the
        // wrong shape, is the framework's to report.
        let Ok(Some(text)) = staged.read(files.device()) else {
            return Ok(());
        };
        let Ok(mut table) = text.parse::<toml::Table>() else {
            return Ok(());
        };
        let Some(Ok(http)) = table.remove("http").map(|t| t.try_into::<HttpConfig>()) else {
            return Ok(());
        };
        if !http.mcp.enabled {
            return Ok(());
        }
        let addr = http.listen.unwrap_or_else(default_http_addr);
        check_exposure(addr, &http.allowed_hosts).map_err(|err| {
            vec![FieldError {
                key: Some("http.allowed_hosts".to_owned()),
                file: Some(files.device().to_owned()),
                message: err.to_string(),
            }]
        })
    }
}

/// The tally's server, without sync: the framework's server with the tally's
/// methods, its MCP tools, and its HTTP port.
///
/// [`build_with`] adds the bridge; a test runs this as it is.
pub fn build() -> AppServer {
    let server = AppServer::new(&TALLY_CTX, VERSION)
        .releases(release_source())
        .app_api(sapphire_tally_mcp::APP_API)
        .mcp(sapphire_tally_mcp::service())
        .http_listen(default_http_addr());
    let host = Arc::clone(server.host());
    server.extend(move |router| {
        sapphire_tally_mcp::register(router, Arc::new(move || current_tally(&host)))
    })
}

/// The server `serve` runs, built again on every restart from `settings`:
/// [`build`], syncing through the host's bridge, with the `mcp` status row.
///
/// The framework connects to the bridge when the server runs, and again
/// whenever the bridge restarts; a bridge that is not running yet stops
/// nothing but sync. Any other failure — a bridge of another version, say —
/// stops the start. `service`, `update` and `mcp` build this too, but neither
/// run it nor reach the bridge through it.
pub fn build_with(settings: &Settings) -> AppServer {
    // Settings the framework cannot turn into its config stop the server
    // before this row is ever shown.
    let http = ServerConfig::from_settings(settings)
        .map(|config| config.http)
        .unwrap_or_default();
    let mcp_row = if http.mcp.enabled {
        let addr = http.listen.unwrap_or_else(default_http_addr);
        format!("http://{addr}/mcp")
    } else {
        "off".to_owned()
    };
    build()
        .sync_via_bridge()
        .expect("the path of the running executable is readable")
        .status_rows(Arc::new(move || {
            vec![StatusRow {
                name: "mcp".to_owned(),
                value: mcp_row.clone(),
            }]
        }))
}

fn default_http_addr() -> SocketAddr {
    DEFAULT_HTTP_ADDR
        .parse()
        .expect("the default address parses")
}

/// The tally the server's current workspace holds, or why there is none.
///
/// Opened per call: a [`Tally`] is only its root, and `workspace select` can
/// switch the root between two calls.
fn current_tally(host: &WorkspaceHost) -> io::Result<Tally> {
    let root = host.root().ok_or_else(|| {
        io::Error::other(
            "this server has no workspace selected; run \
             `sapphire-tally workspace init <dir>` or `sapphire-tally workspace select <dir>`",
        )
    })?;
    Tally::open(root).map_err(io::Error::other)
}

/// Refuse a configuration that would serve 403 to every `/mcp` client.
///
/// rmcp's `Host` allowlist is loopback-only unless widened. Binding beyond
/// loopback without naming a host is not dangerous — it is useless, and it
/// presents as a client bug. Failing here turns that into an obvious
/// configuration error.
pub fn check_exposure(addr: SocketAddr, allowed_hosts: &[String]) -> anyhow::Result<()> {
    if addr.ip().is_loopback() {
        return Ok(());
    }
    if allowed_hosts.iter().any(|h| !h.trim().is_empty()) {
        return Ok(());
    }
    anyhow::bail!(
        "binding to {addr} without allowed_hosts would refuse every request: \
         rmcp's Host allowlist stays loopback-only, so a client reaching this \
         server by any other name gets 403. List each name clients use in \
         [http] allowed_hosts of {SERVER_CONFIG_FILE}, or bind to loopback."
    )
}

/// Where the app's releases are published: what `update` installs from, and where the
/// server looks once a day for a newer one.
fn release_source() -> ReleaseSource {
    ReleaseSource::github("sapphire-turtle", "sapphire-tally")
        .asset("sapphire-tally")
        .cargo_crate("sapphire-tally")
}
