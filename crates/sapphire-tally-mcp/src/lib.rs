//! MCP for sapphire-tally, on the framework's MCP (`sapphire_framework::mcp`).
//!
//! Two halves, joined by the app server's IPC:
//!
//! - [`methods`]: the tally's `sapphire-tally.*` methods, registered on the
//!   app server, working on the workspace it has selected.
//! - [`tools`]: the MCP tools an agent sees, which call those methods. The
//!   framework serves them on stdio (`sapphire-tally mcp`) and at `/mcp` on
//!   the server's HTTP listener.

pub mod methods;
pub mod tools;

pub use methods::{APP_API, StateSource, TallyMethods, register};
pub use tools::{TallyTools, service};
