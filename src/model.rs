use clap::ValueEnum;
use serde::{Deserialize, Serialize};

pub const LOCKFILE_VERSION: u32 = 1;
pub const LOCKFILE_SCHEMA: &str = "https://raw.githubusercontent.com/skikkh/agentfreeze/main/schema/agentfreeze-lock-v1.schema.json";

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum ScanScope {
    All,
    User,
    Workspace,
}

impl ScanScope {
    #[must_use]
    pub const fn includes_user(self) -> bool {
        matches!(self, Self::All | Self::User)
    }

    #[must_use]
    pub const fn includes_workspace(self) -> bool {
        matches!(self, Self::All | Self::Workspace)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum EntryScope {
    User,
    Workspace,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Ecosystem {
    Agents,
    Claude,
    Codex,
    Cursor,
    Gemini,
    Mcp,
    Vscode,
}

impl std::fmt::Display for Ecosystem {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            Self::Agents => "agents",
            Self::Claude => "claude",
            Self::Codex => "codex",
            Self::Cursor => "cursor",
            Self::Gemini => "gemini",
            Self::Mcp => "mcp",
            Self::Vscode => "vscode",
        };
        formatter.write_str(value)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ExtensionKind {
    McpServer,
    Plugin,
    Skill,
}

impl std::fmt::Display for ExtensionKind {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            Self::McpServer => "mcp-server",
            Self::Plugin => "plugin",
            Self::Skill => "skill",
        };
        formatter.write_str(value)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Entry {
    pub id: String,
    pub ecosystem: Ecosystem,
    pub kind: ExtensionKind,
    pub scope: EntryScope,
    pub name: String,
    pub source: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revision: Option<String>,
    pub digest: String,
    pub files: u64,
    pub bytes: u64,
    pub symlinks: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Lockfile {
    #[serde(rename = "$schema")]
    pub schema: String,
    pub lockfile_version: u32,
    pub generated_by: String,
    pub scan_scopes: Vec<EntryScope>,
    pub entries: Vec<Entry>,
}
