use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde_json::Value;

use crate::hashing::{hash_json_value, hash_path, sanitize_mcp_value};
use crate::model::{Ecosystem, Entry, EntryScope, ExtensionKind, ScanScope};

#[derive(Clone, Debug)]
pub struct ScanOptions {
    pub workspace: PathBuf,
    pub home: PathBuf,
    pub scope: ScanScope,
}

impl ScanOptions {
    /// Creates explicit scan options.
    #[must_use]
    pub fn new(workspace: PathBuf, home: PathBuf, scope: ScanScope) -> Self {
        Self {
            workspace,
            home,
            scope,
        }
    }

    /// Creates scan options using the current user's home directory.
    ///
    /// # Errors
    ///
    /// Returns an error when no supported home-directory environment variable is available.
    pub fn from_environment(workspace: PathBuf, scope: ScanScope) -> Result<Self> {
        Ok(Self::new(workspace, home_directory()?, scope))
    }
}

/// Discovers agent extensions in the configured user and workspace scopes.
///
/// # Errors
///
/// Returns an error when a configured extension location cannot be read, parsed, or hashed
/// completely. Discovery fails closed rather than producing a partial inventory.
pub fn scan(options: &ScanOptions) -> Result<Vec<Entry>> {
    let workspace = options
        .workspace
        .canonicalize()
        .with_context(|| format!("cannot resolve workspace {}", options.workspace.display()))?;
    let home = options
        .home
        .canonicalize()
        .with_context(|| format!("cannot resolve home {}", options.home.display()))?;
    let mut entries = BTreeMap::new();

    if options.scope.includes_user() {
        discover_user_skills(&home, &workspace, &mut entries)?;
        discover_user_plugins(&home, &workspace, &mut entries)?;
        discover_user_mcp(&home, &workspace, &mut entries)?;
    }
    if options.scope.includes_workspace() {
        discover_workspace_skills(&workspace, &home, &mut entries)?;
        discover_workspace_plugins(&workspace, &home, &mut entries)?;
        discover_workspace_mcp(&workspace, &home, &mut entries)?;
        discover_claude_project_mcp(&home, &workspace, &mut entries)?;
    }

    Ok(entries.into_values().collect())
}

fn discover_user_skills(
    home: &Path,
    workspace: &Path,
    entries: &mut BTreeMap<String, Entry>,
) -> Result<()> {
    let roots = [
        (home.join(".agents/skills"), Ecosystem::Agents),
        (home.join(".claude/skills"), Ecosystem::Claude),
        (home.join(".codex/skills"), Ecosystem::Codex),
        (home.join(".cursor/skills"), Ecosystem::Cursor),
        (home.join(".cursor/skills-cursor"), Ecosystem::Cursor),
        (home.join(".gemini/skills"), Ecosystem::Gemini),
    ];
    for (root, ecosystem) in roots {
        discover_skill_root(&root, ecosystem, EntryScope::User, workspace, home, entries)?;
    }
    Ok(())
}

fn discover_workspace_skills(
    workspace: &Path,
    home: &Path,
    entries: &mut BTreeMap<String, Entry>,
) -> Result<()> {
    let roots = [
        (workspace.join(".agents/skills"), Ecosystem::Agents),
        (workspace.join(".claude/skills"), Ecosystem::Claude),
        (workspace.join(".codex/skills"), Ecosystem::Codex),
        (workspace.join(".cursor/skills"), Ecosystem::Cursor),
        (workspace.join(".gemini/skills"), Ecosystem::Gemini),
    ];
    for (root, ecosystem) in roots {
        discover_skill_root(
            &root,
            ecosystem,
            EntryScope::Workspace,
            workspace,
            home,
            entries,
        )?;
    }
    Ok(())
}

fn discover_skill_root(
    root: &Path,
    ecosystem: Ecosystem,
    scope: EntryScope,
    workspace: &Path,
    home: &Path,
    entries: &mut BTreeMap<String, Entry>,
) -> Result<()> {
    if !path_exists(root)? {
        return Ok(());
    }
    let mut children = fs::read_dir(root)
        .with_context(|| format!("cannot list skill root {}", root.display()))?
        .collect::<std::io::Result<Vec<_>>>()
        .with_context(|| format!("cannot list skill root {}", root.display()))?;
    children.sort_by_key(std::fs::DirEntry::file_name);

    for child in children {
        let path = child.path();
        if skill_marker_exists(&path.join("SKILL.md"))? {
            let name = child.file_name().to_string_lossy().into_owned();
            add_path_entry(
                entries,
                ecosystem,
                ExtensionKind::Skill,
                scope,
                name,
                &path,
                None,
                None,
                workspace,
                home,
            )?;
        }
    }
    Ok(())
}

fn discover_user_plugins(
    home: &Path,
    workspace: &Path,
    entries: &mut BTreeMap<String, Entry>,
) -> Result<()> {
    discover_claude_installed_plugins(home, workspace, entries)?;
    discover_manifest_plugins(
        &home.join(".codex/plugins/cache"),
        Ecosystem::Codex,
        EntryScope::User,
        7,
        workspace,
        home,
        entries,
    )?;
    discover_manifest_plugins(
        &home.join(".gemini/extensions"),
        Ecosystem::Gemini,
        EntryScope::User,
        3,
        workspace,
        home,
        entries,
    )?;
    Ok(())
}

fn discover_workspace_plugins(
    workspace: &Path,
    home: &Path,
    entries: &mut BTreeMap<String, Entry>,
) -> Result<()> {
    for (root, ecosystem) in [
        (workspace.join(".claude/plugins"), Ecosystem::Claude),
        (workspace.join(".codex/plugins"), Ecosystem::Codex),
        (workspace.join(".gemini/extensions"), Ecosystem::Gemini),
    ] {
        discover_manifest_plugins(
            &root,
            ecosystem,
            EntryScope::Workspace,
            5,
            workspace,
            home,
            entries,
        )?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn discover_manifest_plugins(
    root: &Path,
    ecosystem: Ecosystem,
    scope: EntryScope,
    max_depth: usize,
    workspace: &Path,
    home: &Path,
    entries: &mut BTreeMap<String, Entry>,
) -> Result<()> {
    if !path_exists(root)? {
        return Ok(());
    }
    let mut manifests = Vec::new();
    find_plugin_manifests(root, 0, max_depth, &mut manifests)?;
    manifests.sort();

    for manifest in manifests {
        let plugin_root = if manifest.file_name().and_then(|name| name.to_str())
            == Some("gemini-extension.json")
        {
            manifest
                .parent()
                .context("Gemini extension manifest has no parent")?
        } else {
            manifest
                .parent()
                .and_then(Path::parent)
                .context("plugin manifest has no plugin root")?
        };
        let (name, version) = manifest_identity(&manifest, plugin_root)?;
        add_path_entry(
            entries,
            ecosystem,
            ExtensionKind::Plugin,
            scope,
            name,
            plugin_root,
            version,
            None,
            workspace,
            home,
        )?;
    }
    Ok(())
}

fn find_plugin_manifests(
    directory: &Path,
    depth: usize,
    max_depth: usize,
    manifests: &mut Vec<PathBuf>,
) -> Result<()> {
    if depth > max_depth {
        return Ok(());
    }
    let mut children = fs::read_dir(directory)
        .with_context(|| format!("cannot list plugin directory {}", directory.display()))?
        .collect::<std::io::Result<Vec<_>>>()
        .with_context(|| format!("cannot list plugin directory {}", directory.display()))?;
    children.sort_by_key(std::fs::DirEntry::file_name);

    for child in children {
        let path = child.path();
        let metadata = fs::symlink_metadata(&path)
            .with_context(|| format!("cannot inspect {}", path.display()))?;
        if metadata.file_type().is_symlink() {
            continue;
        }
        if metadata.is_file() && is_plugin_manifest(&path) {
            manifests.push(path);
        } else if metadata.is_dir() {
            find_plugin_manifests(&path, depth + 1, max_depth, manifests)?;
        }
    }
    Ok(())
}

fn is_plugin_manifest(path: &Path) -> bool {
    if path.file_name().and_then(|name| name.to_str()) == Some("gemini-extension.json") {
        return true;
    }
    if path.file_name().and_then(|name| name.to_str()) != Some("plugin.json") {
        return false;
    }
    matches!(
        path.parent()
            .and_then(Path::file_name)
            .and_then(|name| name.to_str()),
        Some(".codex-plugin" | ".claude-plugin")
    )
}

fn manifest_identity(manifest: &Path, plugin_root: &Path) -> Result<(String, Option<String>)> {
    let fallback = plugin_root
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("plugin")
        .to_owned();
    let bytes = fs::read(manifest)
        .with_context(|| format!("cannot read plugin manifest {}", manifest.display()))?;
    let value = serde_json::from_slice::<Value>(&bytes)
        .with_context(|| format!("invalid JSON in plugin manifest {}", manifest.display()))?;
    let name = value
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or(&fallback)
        .to_owned();
    let version = value
        .get("version")
        .and_then(Value::as_str)
        .map(str::to_owned);
    Ok((name, version))
}

fn discover_claude_installed_plugins(
    home: &Path,
    workspace: &Path,
    entries: &mut BTreeMap<String, Entry>,
) -> Result<()> {
    let registry = home.join(".claude/plugins/installed_plugins.json");
    if !path_exists(&registry)? {
        return Ok(());
    }
    let bytes = fs::read(&registry)
        .with_context(|| format!("cannot read Claude plugin registry {}", registry.display()))?;
    let value: Value = serde_json::from_slice(&bytes)
        .with_context(|| format!("invalid JSON in {}", registry.display()))?;
    let Some(plugins) = value.get("plugins").and_then(Value::as_object) else {
        bail!("{} has no plugins object", registry.display());
    };

    let mut plugin_ids = plugins.keys().collect::<Vec<_>>();
    plugin_ids.sort();
    for plugin_id in plugin_ids {
        let records = match &plugins[plugin_id] {
            Value::Array(records) => records.iter().collect::<Vec<_>>(),
            record @ Value::Object(_) => vec![record],
            _ => bail!("invalid install record for Claude plugin {plugin_id}"),
        };
        for record in records {
            let Some(install_path) = record.get("installPath").and_then(Value::as_str) else {
                bail!("Claude plugin {plugin_id} has no installPath");
            };
            let install_path = PathBuf::from(install_path);
            if !install_path.is_absolute() {
                bail!(
                    "Claude plugin path is not absolute: {}",
                    install_path.display()
                );
            }
            let canonical_install_path = install_path.canonicalize().with_context(|| {
                format!(
                    "Claude plugin {plugin_id} is registered but unavailable at {}",
                    install_path.display()
                )
            })?;
            if !canonical_install_path.starts_with(home) {
                bail!(
                    "Claude plugin path is outside the home directory: {}",
                    canonical_install_path.display()
                );
            }
            let version = record
                .get("version")
                .and_then(Value::as_str)
                .map(str::to_owned);
            let revision = record
                .get("gitCommitSha")
                .and_then(Value::as_str)
                .map(str::to_owned);
            add_path_entry(
                entries,
                Ecosystem::Claude,
                ExtensionKind::Plugin,
                EntryScope::User,
                plugin_id.clone(),
                &install_path,
                version,
                revision,
                workspace,
                home,
            )?;
        }
    }
    Ok(())
}

fn discover_user_mcp(
    home: &Path,
    workspace: &Path,
    entries: &mut BTreeMap<String, Entry>,
) -> Result<()> {
    for (path, ecosystem, keys) in [
        (
            home.join(".claude.json"),
            Ecosystem::Claude,
            &["mcpServers"][..],
        ),
        (
            home.join(".cursor/mcp.json"),
            Ecosystem::Cursor,
            &["mcpServers"][..],
        ),
        (
            home.join(".gemini/settings.json"),
            Ecosystem::Gemini,
            &["mcpServers"][..],
        ),
        (
            home.join("Library/Application Support/Claude/claude_desktop_config.json"),
            Ecosystem::Claude,
            &["mcpServers"][..],
        ),
        (
            home.join("Library/Application Support/Code/User/mcp.json"),
            Ecosystem::Vscode,
            &["servers", "mcpServers"][..],
        ),
        (
            home.join("AppData/Roaming/Claude/claude_desktop_config.json"),
            Ecosystem::Claude,
            &["mcpServers"][..],
        ),
        (
            home.join("AppData/Roaming/Code/User/mcp.json"),
            Ecosystem::Vscode,
            &["servers", "mcpServers"][..],
        ),
    ] {
        discover_json_mcp(
            &path,
            ecosystem,
            EntryScope::User,
            keys,
            workspace,
            home,
            entries,
        )?;
    }
    discover_toml_mcp(
        &home.join(".codex/config.toml"),
        Ecosystem::Codex,
        EntryScope::User,
        workspace,
        home,
        entries,
    )
}

fn discover_workspace_mcp(
    workspace: &Path,
    home: &Path,
    entries: &mut BTreeMap<String, Entry>,
) -> Result<()> {
    for (path, ecosystem, keys) in [
        (
            workspace.join(".mcp.json"),
            Ecosystem::Mcp,
            &["mcpServers", "servers"][..],
        ),
        (
            workspace.join(".cursor/mcp.json"),
            Ecosystem::Cursor,
            &["mcpServers"][..],
        ),
        (
            workspace.join(".gemini/settings.json"),
            Ecosystem::Gemini,
            &["mcpServers"][..],
        ),
        (
            workspace.join(".vscode/mcp.json"),
            Ecosystem::Vscode,
            &["servers", "mcpServers"][..],
        ),
    ] {
        discover_json_mcp(
            &path,
            ecosystem,
            EntryScope::Workspace,
            keys,
            workspace,
            home,
            entries,
        )?;
    }
    discover_toml_mcp(
        &workspace.join(".codex/config.toml"),
        Ecosystem::Codex,
        EntryScope::Workspace,
        workspace,
        home,
        entries,
    )
}

fn discover_claude_project_mcp(
    home: &Path,
    workspace: &Path,
    entries: &mut BTreeMap<String, Entry>,
) -> Result<()> {
    let path = home.join(".claude.json");
    if !path_exists(&path)? {
        return Ok(());
    }
    let bytes = fs::read(&path).with_context(|| format!("cannot read {}", path.display()))?;
    let value: Value = serde_json::from_slice(&bytes)
        .with_context(|| format!("invalid JSON in {}", path.display()))?;
    let Some(projects) = value.get("projects").and_then(Value::as_object) else {
        return Ok(());
    };
    let workspace_key = workspace.to_string_lossy();
    let Some(project) = projects.get(workspace_key.as_ref()) else {
        return Ok(());
    };
    if let Some(servers) = project.get("mcpServers").and_then(Value::as_object) {
        add_mcp_servers(
            entries,
            servers,
            Ecosystem::Claude,
            EntryScope::Workspace,
            &path,
            "projects/$WORKSPACE/mcpServers",
            workspace,
            home,
        )?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn discover_json_mcp(
    path: &Path,
    ecosystem: Ecosystem,
    scope: EntryScope,
    keys: &[&str],
    workspace: &Path,
    home: &Path,
    entries: &mut BTreeMap<String, Entry>,
) -> Result<()> {
    if !path_exists(path)? {
        return Ok(());
    }
    let bytes = fs::read(path).with_context(|| format!("cannot read {}", path.display()))?;
    let value: Value = serde_json::from_slice(&bytes)
        .with_context(|| format!("invalid JSON in {}", path.display()))?;
    for key in keys {
        if let Some(servers) = value.get(*key).and_then(Value::as_object) {
            add_mcp_servers(
                entries, servers, ecosystem, scope, path, key, workspace, home,
            )?;
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn discover_toml_mcp(
    path: &Path,
    ecosystem: Ecosystem,
    scope: EntryScope,
    workspace: &Path,
    home: &Path,
    entries: &mut BTreeMap<String, Entry>,
) -> Result<()> {
    if !path_exists(path)? {
        return Ok(());
    }
    let text =
        fs::read_to_string(path).with_context(|| format!("cannot read {}", path.display()))?;
    let value: toml::Value =
        toml::from_str(&text).with_context(|| format!("invalid TOML in {}", path.display()))?;
    let Some(servers) = value.get("mcp_servers").and_then(toml::Value::as_table) else {
        return Ok(());
    };
    let mut names = servers.keys().collect::<Vec<_>>();
    names.sort();
    for name in names {
        let json = serde_json::to_value(&servers[name])
            .with_context(|| format!("cannot normalize MCP server {name}"))?;
        add_mcp_entry(
            entries,
            ecosystem,
            scope,
            name,
            path,
            &format!("mcp_servers/{}", pointer_escape(name)),
            &json,
            workspace,
            home,
        )?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn add_mcp_servers(
    entries: &mut BTreeMap<String, Entry>,
    servers: &serde_json::Map<String, Value>,
    ecosystem: Ecosystem,
    scope: EntryScope,
    path: &Path,
    pointer: &str,
    workspace: &Path,
    home: &Path,
) -> Result<()> {
    let mut names = servers.keys().collect::<Vec<_>>();
    names.sort();
    for name in names {
        add_mcp_entry(
            entries,
            ecosystem,
            scope,
            name,
            path,
            &format!("{pointer}/{}", pointer_escape(name)),
            &servers[name],
            workspace,
            home,
        )?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn add_mcp_entry(
    entries: &mut BTreeMap<String, Entry>,
    ecosystem: Ecosystem,
    scope: EntryScope,
    name: &str,
    config_path: &Path,
    pointer: &str,
    value: &Value,
    workspace: &Path,
    home: &Path,
) -> Result<()> {
    let sanitized = sanitize_mcp_value(value, workspace, home, scope);
    let hashed = hash_json_value(&sanitized)?;
    let source = format!(
        "{}#/{pointer}",
        anchored_path(config_path, workspace, home, scope)?
    );
    let id = entry_id(ecosystem, ExtensionKind::McpServer, scope, &source);
    insert_entry(
        entries,
        Entry {
            id,
            ecosystem,
            kind: ExtensionKind::McpServer,
            scope,
            name: name.to_owned(),
            source,
            version: None,
            revision: None,
            digest: hashed.digest,
            files: hashed.files,
            bytes: hashed.bytes,
            symlinks: hashed.symlinks,
        },
    )
}

#[allow(clippy::too_many_arguments)]
fn add_path_entry(
    entries: &mut BTreeMap<String, Entry>,
    ecosystem: Ecosystem,
    kind: ExtensionKind,
    scope: EntryScope,
    name: String,
    path: &Path,
    version: Option<String>,
    revision: Option<String>,
    workspace: &Path,
    home: &Path,
) -> Result<()> {
    let boundary = match scope {
        EntryScope::User => home,
        EntryScope::Workspace => workspace,
    };
    let canonical = path
        .canonicalize()
        .with_context(|| format!("cannot resolve extension source {}", path.display()))?;
    if !canonical.starts_with(boundary) {
        bail!(
            "{} extension source escapes its {} boundary: {}",
            match scope {
                EntryScope::User => "user",
                EntryScope::Workspace => "workspace",
            },
            match scope {
                EntryScope::User => "home",
                EntryScope::Workspace => "workspace",
            },
            canonical.display()
        );
    }
    let hashed = hash_path(path)?;
    let source = anchored_path(&canonical, workspace, home, scope)?;
    let id = entry_id(ecosystem, kind, scope, &source);
    insert_entry(
        entries,
        Entry {
            id,
            ecosystem,
            kind,
            scope,
            name,
            source,
            version,
            revision,
            digest: hashed.digest,
            files: hashed.files,
            bytes: hashed.bytes,
            symlinks: hashed.symlinks,
        },
    )
}

fn insert_entry(entries: &mut BTreeMap<String, Entry>, entry: Entry) -> Result<()> {
    if let Some(existing) = entries.get(&entry.id) {
        if existing == &entry {
            return Ok(());
        }
        bail!(
            "two different extensions produced the same id: {}",
            entry.id
        );
    }
    entries.insert(entry.id.clone(), entry);
    Ok(())
}

fn entry_id(ecosystem: Ecosystem, kind: ExtensionKind, scope: EntryScope, source: &str) -> String {
    format!(
        "{ecosystem}:{kind}:{}:{source}",
        match scope {
            EntryScope::User => "user",
            EntryScope::Workspace => "workspace",
        }
    )
}

fn anchored_path(path: &Path, workspace: &Path, home: &Path, scope: EntryScope) -> Result<String> {
    let anchors = match scope {
        EntryScope::User => [("$HOME", home), ("$WORKSPACE", workspace)],
        EntryScope::Workspace => [("$WORKSPACE", workspace), ("$HOME", home)],
    };
    for (anchor, prefix) in anchors {
        if let Ok(relative) = path.strip_prefix(prefix) {
            return anchored(anchor, relative);
        }
    }
    let value = path
        .to_str()
        .with_context(|| format!("non-UTF-8 source path: {}", path.display()))?;
    Ok(value.replace('\\', "/"))
}

fn anchored(anchor: &str, relative: &Path) -> Result<String> {
    if relative.as_os_str().is_empty() {
        return Ok(anchor.to_owned());
    }
    let relative = relative
        .to_str()
        .with_context(|| format!("non-UTF-8 source path: {}", relative.display()))?;
    Ok(format!("{anchor}/{}", relative.replace('\\', "/")))
}

fn pointer_escape(value: &str) -> String {
    value.replace('~', "~0").replace('/', "~1")
}

fn path_exists(path: &Path) -> Result<bool> {
    match fs::metadata(path) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            match fs::symlink_metadata(path) {
                Ok(metadata) if metadata.file_type().is_symlink() => {
                    bail!("broken discovery symlink {}", path.display())
                }
                Ok(_) => Ok(true),
                Err(link_error) if link_error.kind() == std::io::ErrorKind::NotFound => Ok(false),
                Err(link_error) => Err(link_error)
                    .with_context(|| format!("cannot inspect discovery path {}", path.display())),
            }
        }
        Err(error) => {
            Err(error).with_context(|| format!("cannot inspect discovery path {}", path.display()))
        }
    }
}

fn skill_marker_exists(path: &Path) -> Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => Ok(metadata.is_file() || metadata.file_type().is_symlink()),
        Err(error)
            if matches!(
                error.kind(),
                std::io::ErrorKind::NotFound | std::io::ErrorKind::NotADirectory
            ) =>
        {
            Ok(false)
        }
        Err(error) => {
            Err(error).with_context(|| format!("cannot inspect skill marker {}", path.display()))
        }
    }
}

fn home_directory() -> Result<PathBuf> {
    if let Some(home) = env::var_os("HOME") {
        return Ok(PathBuf::from(home));
    }
    if let Some(profile) = env::var_os("USERPROFILE") {
        return Ok(PathBuf::from(profile));
    }
    match (env::var_os("HOMEDRIVE"), env::var_os("HOMEPATH")) {
        (Some(drive), Some(path)) => {
            let mut joined = PathBuf::from(drive);
            joined.push(path);
            Ok(joined)
        }
        _ => bail!("cannot determine the home directory; pass --home"),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::{ScanOptions, scan};
    use crate::model::{ExtensionKind, ScanScope};

    #[test]
    fn scans_user_and_workspace_extensions_without_exposing_secrets() {
        let fixture = tempdir().expect("tempdir");
        let home = fixture.path().join("home");
        let workspace = fixture.path().join("workspace");
        fs::create_dir_all(home.join(".claude/skills/review")).expect("user skill directory");
        fs::create_dir_all(workspace.join(".agents/skills/test"))
            .expect("workspace skill directory");
        fs::write(home.join(".claude/skills/review/SKILL.md"), "review").expect("user skill");
        fs::write(workspace.join(".agents/skills/test/SKILL.md"), "workspace")
            .expect("workspace skill");
        fs::write(
            workspace.join(".mcp.json"),
            r#"{
                "mcpServers": {
                    "example": {
                        "command": "node",
                        "args": ["server.js", "--token", "do-not-record"],
                        "env": {"API_KEY": "do-not-record", "MODE": "safe"}
                    }
                }
            }"#,
        )
        .expect("MCP config");

        let entries = scan(&ScanOptions::new(workspace, home, ScanScope::All)).expect("scan");
        assert_eq!(entries.len(), 3);
        assert_eq!(
            entries
                .iter()
                .filter(|entry| entry.kind == ExtensionKind::Skill)
                .count(),
            2
        );
        let serialized = serde_json::to_string(&entries).expect("serialize entries");
        assert!(!serialized.contains("do-not-record"));
        assert!(!serialized.contains(fixture.path().to_string_lossy().as_ref()));
    }

    #[test]
    fn user_sources_prefer_home_when_workspace_is_the_home_directory() {
        let fixture = tempdir().expect("tempdir");
        let home = fixture.path().join("home");
        fs::create_dir_all(home.join(".claude/skills/review")).expect("user skill directory");
        fs::write(home.join(".claude/skills/review/SKILL.md"), "review").expect("user skill");

        let entries =
            scan(&ScanOptions::new(home.clone(), home, ScanScope::User)).expect("user scan");

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].source, "$HOME/.claude/skills/review");
        assert!(entries[0].id.ends_with(":$HOME/.claude/skills/review"));
    }

    #[cfg(unix)]
    #[test]
    fn user_extension_root_cannot_escape_the_home_boundary() {
        use std::os::unix::fs::symlink;

        let fixture = tempdir().expect("tempdir");
        let home = fixture.path().join("home");
        let workspace = fixture.path().join("workspace");
        let outside = fixture.path().join("outside-skill");
        fs::create_dir_all(home.join(".claude/skills")).expect("user skill root");
        fs::create_dir_all(&workspace).expect("workspace");
        fs::create_dir_all(&outside).expect("outside skill");
        fs::write(outside.join("SKILL.md"), "outside").expect("outside marker");
        symlink(&outside, home.join(".claude/skills/escaped")).expect("escaped skill link");

        let error = scan(&ScanOptions::new(workspace, home, ScanScope::User))
            .expect_err("escaped user extension must fail");
        assert!(error.to_string().contains("escapes its home boundary"));
    }
}
