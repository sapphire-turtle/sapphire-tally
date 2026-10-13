//! The tally's MCP tools: an `rmcp` handler whose tools call the app server's
//! `sapphire-tally.*` methods ([`crate::methods`]), beside the framework's
//! own tools.
//!
//! The handler holds the framework's [`ToolContext`], so the same tools serve
//! the `mcp` subcommand (which dials the running server) and `/mcp` on the
//! server's HTTP listener (an in-process connection). Hand it to the server
//! with [`service`].

use rmcp::{
    ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::ServerConfig,
    tool, tool_handler, tool_router,
};
use sapphire_framework::ipc;
use sapphire_framework::mcp::{self, McpService, ToolContext};
use serde::Serialize;

use crate::methods::{
    ActivityAddParams, ActivityUpdateParams, EmptyParams, IdParams, QueryParams, StampAddParams,
    StampUpdateParams, names,
};

/// How the tally answers MCP: [`TallyTools`], one per session.
pub fn service() -> McpService {
    McpService::new(TallyTools::new)
}

#[derive(Clone, Debug)]
pub struct TallyTools {
    ctx: ToolContext,
    tool_router: ToolRouter<Self>,
}

impl AsRef<ToolContext> for TallyTools {
    fn as_ref(&self) -> &ToolContext {
        &self.ctx
    }
}

impl TallyTools {
    /// The tally's tools and the framework's read-only ones. The framework's
    /// raw file writes are left out: a stamp or activity written by hand
    /// skips the tally's checks (an activity that exists, a grain-id
    /// filename), and the tools below already cover every write.
    pub fn new(ctx: ToolContext) -> Self {
        Self {
            ctx,
            tool_router: Self::tool_router() + mcp::framework_tools(),
        }
    }

    /// Call one of the tally's methods and hand its text back as the tool's.
    ///
    /// An error the method answered with is the tally's message to the agent,
    /// so it goes through without the IPC layer's framing.
    async fn call(&self, method: &str, params: impl Serialize) -> Result<String, String> {
        self.ctx.call(method, params).await.map_err(|e| match e {
            mcp::Error::Ipc(ipc::Error::Rpc(rpc)) => rpc.message,
            other => other.to_string(),
        })
    }
}

#[tool_router]
impl TallyTools {
    #[tool(
        description = "Create a new activity (tracker). `unit` is an optional display \
        granularity: day/week/month (default day). `description` is optional Markdown. \
        Returns the created activity JSON including its id."
    )]
    pub async fn activity_add(
        &self,
        Parameters(p): Parameters<ActivityAddParams>,
    ) -> Result<String, String> {
        self.call(names::ACTIVITY_ADD, p).await
    }

    #[tool(
        description = "Show one activity by grain-id: its title, unit and description.",
        annotations(read_only_hint = true)
    )]
    pub async fn activity_get(
        &self,
        Parameters(p): Parameters<IdParams>,
    ) -> Result<String, String> {
        self.call(names::ACTIVITY_GET, p).await
    }

    #[tool(
        description = "Change an activity's title, unit or description. Omitted fields stay \
        as they are; an empty description removes it. Returns the updated activity JSON."
    )]
    pub async fn activity_update(
        &self,
        Parameters(p): Parameters<ActivityUpdateParams>,
    ) -> Result<String, String> {
        self.call(names::ACTIVITY_UPDATE, p).await
    }

    #[tool(
        description = "List all activities as a JSON array, each with id, title, unit and \
        description.",
        annotations(read_only_hint = true)
    )]
    pub async fn activity_list(
        &self,
        Parameters(p): Parameters<EmptyParams>,
    ) -> Result<String, String> {
        self.call(names::ACTIVITY_LIST, p).await
    }

    #[tool(
        description = "Add a stamp. `activity` is the activity's grain-id. `timestamp` is \
        optional ISO 8601 with timezone (server time is used when omitted). `comment` is an \
        optional Markdown note."
    )]
    pub async fn stamp_add(
        &self,
        Parameters(p): Parameters<StampAddParams>,
    ) -> Result<String, String> {
        self.call(names::STAMP_ADD, p).await
    }

    #[tool(
        description = "Show one stamp by grain-id: its activity, timestamp and comment.",
        annotations(read_only_hint = true)
    )]
    pub async fn stamp_get(&self, Parameters(p): Parameters<IdParams>) -> Result<String, String> {
        self.call(names::STAMP_GET, p).await
    }

    #[tool(
        description = "Change a stamp's activity, timestamp or comment. Omitted fields stay \
        as they are; an empty comment removes it. Returns the updated stamp JSON."
    )]
    pub async fn stamp_update(
        &self,
        Parameters(p): Parameters<StampUpdateParams>,
    ) -> Result<String, String> {
        self.call(names::STAMP_UPDATE, p).await
    }

    #[tool(
        description = "List stamps as a JSON array, sorted by timestamp ascending. Optional \
        filters: activity (grain-id), from/to (YYYY-MM-DD, inclusive).",
        annotations(read_only_hint = true)
    )]
    pub async fn stamp_list(
        &self,
        Parameters(p): Parameters<QueryParams>,
    ) -> Result<String, String> {
        self.call(names::STAMP_LIST, p).await
    }

    #[tool(
        description = "Heatmap buckets as a JSON array. Buckets use the activity's unit \
        (day/week/month); period format is 2026-09-05 / 2026-W36 / 2026-09. Same filters as \
        stamp_list. Zero-count periods are omitted.",
        annotations(read_only_hint = true)
    )]
    pub async fn heatmap(&self, Parameters(p): Parameters<QueryParams>) -> Result<String, String> {
        self.call(names::HEATMAP, p).await
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for TallyTools {
    fn get_info(&self) -> ServerConfig {
        let info = mcp::server_info(&self.ctx);
        let framework = info.instructions.clone().unwrap_or_default();
        info.with_instructions(format!(
            "Sapphire Tally records stamps (activity + timestamp + optional Markdown \
             comment) as Markdown files and aggregates them into GitHub-style \
             heatmap buckets. Use activity_list to find an activity's id, stamp_add to \
             record one, and heatmap to see how often it happened.\n\n{framework}"
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tools() -> Vec<rmcp::model::Tool> {
        let endpoint = ipc::Endpoint::in_dir("sapphire-tally", std::env::temp_dir());
        let ctx = ToolContext::dial("sapphire-tally", "0.0.0", None, endpoint);
        TallyTools::new(ctx).tool_router.list_all()
    }

    /// Zero-argument tools must advertise a top-level `type: object`, or
    /// Anthropic rejects the tool with
    /// `tools.<N>.custom.input_schema.type: Field required`.
    #[test]
    fn every_tool_input_schema_declares_object_type() {
        for tool in tools() {
            let schema = serde_json::to_value(&tool.input_schema).expect("schema to json");
            assert_eq!(
                schema.get("type").and_then(|t| t.as_str()),
                Some("object"),
                "tool {} has a non-object input schema: {schema}",
                tool.name
            );
        }
    }

    /// The tally's nine, the framework's read-only seven, and none of the
    /// framework's raw file writes.
    #[test]
    fn the_tally_serves_its_tools_and_the_frameworks_read_only_ones() {
        let names: Vec<String> = tools().iter().map(|t| t.name.to_string()).collect();
        for expected in [
            "activity_add",
            "stamp_add",
            "heatmap",
            "status",
            "workspace_show",
            "workspace_read_file",
        ] {
            assert!(
                names.iter().any(|n| n == expected),
                "missing {expected}: {names:?}"
            );
        }
        assert!(
            !names.iter().any(|n| n.starts_with("workspace_write")
                || n == "workspace_append_file"
                || n == "workspace_delete_file"),
            "raw writes would bypass the tally's checks: {names:?}"
        );
        assert_eq!(names.len(), 9 + 7, "{names:?}");
    }
}
