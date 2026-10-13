//! sapphire-tally-core: sapphire-tally's data model and TOML store.
//!
//! Records are Markdown files with YAML front matter, laid out flat under
//! `<data-dir>/activities/` and `<data-dir>/stamps/` (one file per record,
//! named `<grain-id>.md`). The free text (an activity's description, a stamp's
//! comment) is the body; the rest is front matter. Each file also carries an
//! `id` field, but the filename is authoritative and a mismatch is logged as a
//! warning.
//!
//! The data directory is the workspace the app server has selected
//! (`sapphire-tally workspace select`); syncing it is the framework's job.

pub mod error;
pub mod model;
pub mod store;
pub mod tally;

pub use error::{Error, Result};
pub use model::{Activity, ActivityUpdate, HeatmapBucket, Stamp, StampUpdate, Unit};
pub use tally::Tally;

/// Process-wide application context, naming the cache, data and config
/// directories the framework uses. Mirrors `LEDGER_CTX` in sapphire-ledger.
pub static TALLY_CTX: sapphire_workspace::AppContext =
    sapphire_workspace::AppContext::new("sapphire-tally");

pub use sapphire_workspace::AppKind;

/// Point [`TALLY_CTX`] at this machine's cache, data and config directories.
///
/// `AppContext::cache_dir()` panics if this has not run, so every binary that
/// touches the context calls this first. The framework resolves the
/// directories (honouring `SAPPHIRE_TALLY_{CACHE,DATA,CONFIG}_DIR`) and
/// installs its console log subscriber. Idempotent: first writer wins.
pub fn init_app_context(kind: AppKind) {
    TALLY_CTX.init(kind);
}
