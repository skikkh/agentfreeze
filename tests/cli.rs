use std::fs;
use std::process::Command;

use tempfile::tempdir;

#[test]
fn freeze_verify_and_detect_drift_end_to_end() {
    let fixture = tempdir().expect("tempdir");
    let home = fixture.path().join("home");
    let workspace = fixture.path().join("workspace");
    let skill = home.join(".claude/skills/demo/SKILL.md");
    let lockfile = fixture.path().join("agentfreeze.lock");
    fs::create_dir_all(skill.parent().expect("skill parent")).expect("create skill");
    fs::create_dir_all(&workspace).expect("create workspace");
    fs::write(&skill, "# Demo\n").expect("write skill");

    let binary = env!("CARGO_BIN_EXE_agentfreeze");
    let freeze = Command::new(binary)
        .args([
            "freeze",
            "--home",
            home.to_str().expect("home path"),
            "--workspace",
            workspace.to_str().expect("workspace path"),
            "--output",
            lockfile.to_str().expect("lockfile path"),
        ])
        .output()
        .expect("run freeze");
    assert!(
        freeze.status.success(),
        "{}",
        String::from_utf8_lossy(&freeze.stderr)
    );

    let repeat_freeze = Command::new(binary)
        .args([
            "freeze",
            "--home",
            home.to_str().expect("home path"),
            "--workspace",
            workspace.to_str().expect("workspace path"),
            "--output",
            lockfile.to_str().expect("lockfile path"),
        ])
        .output()
        .expect("repeat freeze");
    assert!(
        repeat_freeze.status.success(),
        "{}",
        String::from_utf8_lossy(&repeat_freeze.stderr)
    );

    let clean = verify(binary, &home, &workspace, &lockfile);
    assert!(clean.status.success());
    assert!(String::from_utf8_lossy(&clean.stdout).contains("no drift"));

    fs::write(&skill, "# Modified\n").expect("modify skill");
    let drift = verify(binary, &home, &workspace, &lockfile);
    assert_eq!(drift.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&drift.stdout).contains("modified"));

    let watch = Command::new(binary)
        .args([
            "watch",
            "--home",
            home.to_str().expect("home path"),
            "--workspace",
            workspace.to_str().expect("workspace path"),
            "--lockfile",
            lockfile.to_str().expect("lockfile path"),
            "--max-scans",
            "1",
            "--json",
        ])
        .output()
        .expect("run watch");
    assert_eq!(watch.status.code(), Some(1));
    let event: serde_json::Value = serde_json::from_slice(&watch.stdout).expect("watch JSON event");
    assert_eq!(event["scan"], 1);
    assert_eq!(event["verification"]["status"], "drift");
}

#[test]
fn malformed_configuration_and_lockfile_fail_closed() {
    let fixture = tempdir().expect("tempdir");
    let home = fixture.path().join("home");
    let workspace = fixture.path().join("workspace");
    let lockfile = fixture.path().join("agentfreeze.lock");
    fs::create_dir_all(&home).expect("create home");
    fs::create_dir_all(&workspace).expect("create workspace");
    fs::write(home.join(".claude.json"), "{not-json").expect("write malformed config");

    let binary = env!("CARGO_BIN_EXE_agentfreeze");
    let freeze = Command::new(binary)
        .args([
            "freeze",
            "--home",
            home.to_str().expect("home path"),
            "--workspace",
            workspace.to_str().expect("workspace path"),
            "--output",
            lockfile.to_str().expect("lockfile path"),
        ])
        .output()
        .expect("run freeze");
    assert_eq!(freeze.status.code(), Some(2));
    assert!(!lockfile.exists());
    assert!(String::from_utf8_lossy(&freeze.stderr).contains("invalid JSON"));

    fs::write(home.join(".claude.json"), "{}").expect("repair config");
    fs::write(&lockfile, "{not-json").expect("write malformed lockfile");
    let verify = verify(binary, &home, &workspace, &lockfile);
    assert_eq!(verify.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&verify.stderr).contains("invalid JSON"));
}

#[cfg(unix)]
#[test]
fn unreadable_extension_fails_instead_of_producing_a_partial_lock() {
    use std::os::unix::fs::PermissionsExt;

    let fixture = tempdir().expect("tempdir");
    let home = fixture.path().join("home");
    let workspace = fixture.path().join("workspace");
    let skill = home.join(".claude/skills/demo/SKILL.md");
    let lockfile = fixture.path().join("agentfreeze.lock");
    fs::create_dir_all(skill.parent().expect("skill parent")).expect("create skill");
    fs::create_dir_all(&workspace).expect("create workspace");
    fs::write(&skill, "# Demo\n").expect("write skill");
    fs::set_permissions(&skill, fs::Permissions::from_mode(0o000)).expect("remove read permission");

    let freeze = Command::new(env!("CARGO_BIN_EXE_agentfreeze"))
        .args([
            "freeze",
            "--home",
            home.to_str().expect("home path"),
            "--workspace",
            workspace.to_str().expect("workspace path"),
            "--output",
            lockfile.to_str().expect("lockfile path"),
        ])
        .output()
        .expect("run freeze");

    fs::set_permissions(&skill, fs::Permissions::from_mode(0o600))
        .expect("restore read permission");
    assert_eq!(freeze.status.code(), Some(2));
    assert!(!lockfile.exists());
    assert!(String::from_utf8_lossy(&freeze.stderr).contains("cannot read"));
}

#[cfg(unix)]
#[test]
fn unreadable_discovery_parent_fails_instead_of_appearing_absent() {
    use std::os::unix::fs::PermissionsExt;

    let fixture = tempdir().expect("tempdir");
    let home = fixture.path().join("home");
    let workspace = fixture.path().join("workspace");
    let claude_root = home.join(".claude");
    let skill = claude_root.join("skills/demo/SKILL.md");
    let lockfile = fixture.path().join("agentfreeze.lock");
    fs::create_dir_all(skill.parent().expect("skill parent")).expect("create skill");
    fs::create_dir_all(&workspace).expect("create workspace");
    fs::write(&skill, "# Demo\n").expect("write skill");
    fs::set_permissions(&claude_root, fs::Permissions::from_mode(0o000))
        .expect("remove parent permission");

    let freeze = Command::new(env!("CARGO_BIN_EXE_agentfreeze"))
        .args([
            "freeze",
            "--home",
            home.to_str().expect("home path"),
            "--workspace",
            workspace.to_str().expect("workspace path"),
            "--output",
            lockfile.to_str().expect("lockfile path"),
        ])
        .output()
        .expect("run freeze");

    fs::set_permissions(&claude_root, fs::Permissions::from_mode(0o700))
        .expect("restore parent permission");
    assert_eq!(freeze.status.code(), Some(2));
    assert!(!lockfile.exists());
    assert!(String::from_utf8_lossy(&freeze.stderr).contains("cannot inspect discovery path"));
}

#[test]
fn malformed_plugin_manifest_fails_instead_of_using_fallback_metadata() {
    let fixture = tempdir().expect("tempdir");
    let home = fixture.path().join("home");
    let workspace = fixture.path().join("workspace");
    let manifest = home.join(".gemini/extensions/broken/gemini-extension.json");
    let lockfile = fixture.path().join("agentfreeze.lock");
    fs::create_dir_all(manifest.parent().expect("manifest parent")).expect("create extension");
    fs::create_dir_all(&workspace).expect("create workspace");
    fs::write(&manifest, "{not-json").expect("write malformed manifest");

    let freeze = Command::new(env!("CARGO_BIN_EXE_agentfreeze"))
        .args([
            "freeze",
            "--home",
            home.to_str().expect("home path"),
            "--workspace",
            workspace.to_str().expect("workspace path"),
            "--output",
            lockfile.to_str().expect("lockfile path"),
        ])
        .output()
        .expect("run freeze");

    assert_eq!(freeze.status.code(), Some(2));
    assert!(!lockfile.exists());
    assert!(String::from_utf8_lossy(&freeze.stderr).contains("invalid JSON in plugin manifest"));
}

#[test]
fn semantically_invalid_plugin_metadata_fails_closed() {
    let cases = [
        ("root-array", "[]"),
        ("name-number", r#"{"name":1}"#),
        ("version-number", r#"{"name":"broken","version":1}"#),
    ];

    for (case_name, content) in cases {
        let fixture = tempdir().expect("tempdir");
        let home = fixture.path().join("home");
        let workspace = fixture.path().join("workspace");
        let manifest = home.join(".gemini/extensions/broken/gemini-extension.json");
        fs::create_dir_all(manifest.parent().expect("manifest parent")).expect("create extension");
        fs::create_dir_all(&workspace).expect("create workspace");
        fs::write(&manifest, content).expect("write manifest");

        let scan = scan_scope(&home, &workspace, "user");
        assert_eq!(
            scan.status.code(),
            Some(2),
            "case {case_name}: {}",
            String::from_utf8_lossy(&scan.stderr)
        );
        assert!(
            String::from_utf8_lossy(&scan.stderr).contains("must be"),
            "case {case_name}: {}",
            String::from_utf8_lossy(&scan.stderr)
        );
    }
}

#[test]
fn semantically_invalid_claude_registry_metadata_fails_closed() {
    let cases = [
        ("empty-records", r#"{"plugins":{"broken":[]}}"#),
        (
            "version-number",
            r#"{"plugins":{"broken":{"installPath":"INSTALL_PATH","version":1}}}"#,
        ),
        (
            "revision-number",
            r#"{"plugins":{"broken":{"installPath":"INSTALL_PATH","gitCommitSha":1}}}"#,
        ),
    ];

    for (case_name, template) in cases {
        let fixture = tempdir().expect("tempdir");
        let home = fixture.path().join("home");
        let workspace = fixture.path().join("workspace");
        let registry = home.join(".claude/plugins/installed_plugins.json");
        let install = home.join(".claude/plugins/cache/broken");
        fs::create_dir_all(registry.parent().expect("registry parent")).expect("registry parent");
        fs::create_dir_all(&install).expect("plugin install");
        fs::create_dir_all(&workspace).expect("workspace");
        let content = template.replace("INSTALL_PATH", &install.to_string_lossy());
        fs::write(&registry, content).expect("write registry");

        let scan = scan_scope(&home, &workspace, "user");
        assert_eq!(
            scan.status.code(),
            Some(2),
            "case {case_name}: {}",
            String::from_utf8_lossy(&scan.stderr)
        );
    }
}

#[cfg(unix)]
#[test]
fn special_discovery_files_fail_before_they_can_block_reads() {
    let cases = [
        (".claude.json", "MCP JSON config"),
        (
            ".claude/plugins/installed_plugins.json",
            "Claude plugin registry",
        ),
        (
            ".gemini/extensions/broken/gemini-extension.json",
            "unsupported plugin discovery entry",
        ),
    ];

    for (relative, expected_error) in cases {
        let fixture = tempdir().expect("tempdir");
        let home = fixture.path().join("home");
        let workspace = fixture.path().join("workspace");
        let special = home.join(relative);
        fs::create_dir_all(special.parent().expect("special parent")).expect("special parent");
        fs::create_dir_all(&workspace).expect("workspace");
        let status = Command::new("mkfifo")
            .arg(&special)
            .status()
            .expect("create FIFO");
        assert!(status.success(), "mkfifo {relative}");

        let scan = scan_scope(&home, &workspace, "user");
        assert_eq!(
            scan.status.code(),
            Some(2),
            "case {relative}: {}",
            String::from_utf8_lossy(&scan.stderr)
        );
        assert!(
            String::from_utf8_lossy(&scan.stderr).contains(expected_error),
            "case {relative}: {}",
            String::from_utf8_lossy(&scan.stderr)
        );
    }
}

#[cfg(unix)]
#[test]
fn plugin_symlink_escapes_fail_closed_across_supported_roots() {
    use std::os::unix::fs::symlink;

    let cases = [
        (
            ".codex/plugins/cache/escaped",
            ".codex-plugin/plugin.json",
            "user",
        ),
        (
            ".gemini/extensions/escaped",
            "gemini-extension.json",
            "user",
        ),
        (
            ".claude/plugins/escaped",
            ".claude-plugin/plugin.json",
            "workspace",
        ),
        (
            ".codex/plugins/escaped",
            ".codex-plugin/plugin.json",
            "workspace",
        ),
        (
            ".gemini/extensions/escaped",
            "gemini-extension.json",
            "workspace",
        ),
    ];

    for (index, (link_relative, manifest_relative, scope)) in cases.iter().enumerate() {
        let fixture = tempdir().expect("tempdir");
        let home = fixture.path().join("home");
        let workspace = fixture.path().join("workspace");
        let boundary = if *scope == "user" { &home } else { &workspace };
        let link = boundary.join(link_relative);
        let outside = fixture.path().join(format!("outside-plugin-{index}"));
        let manifest = outside.join(manifest_relative);
        fs::create_dir_all(link.parent().expect("link parent")).expect("plugin root");
        fs::create_dir_all(manifest.parent().expect("manifest parent")).expect("outside plugin");
        fs::create_dir_all(&home).expect("home");
        fs::create_dir_all(&workspace).expect("workspace");
        fs::write(&manifest, r#"{"name":"escaped","version":"1.0.0"}"#).expect("manifest");
        symlink(&outside, &link).expect("escaped plugin link");

        let scan = Command::new(env!("CARGO_BIN_EXE_agentfreeze"))
            .args([
                "scan",
                "--home",
                home.to_str().expect("home path"),
                "--workspace",
                workspace.to_str().expect("workspace path"),
                "--scope",
                scope,
            ])
            .output()
            .expect("run scan");
        assert_eq!(scan.status.code(), Some(2), "case {link_relative}");
        assert!(
            String::from_utf8_lossy(&scan.stderr)
                .contains("plugin discovery symlink escapes its root"),
            "case {link_relative}: {}",
            String::from_utf8_lossy(&scan.stderr)
        );

        if *scope == "user" {
            let lockfile = fixture.path().join("agentfreeze.lock");
            fs::write(&lockfile, "existing-baseline\n").expect("existing lock");
            let freeze = Command::new(env!("CARGO_BIN_EXE_agentfreeze"))
                .args([
                    "freeze",
                    "--home",
                    home.to_str().expect("home path"),
                    "--workspace",
                    workspace.to_str().expect("workspace path"),
                    "--output",
                    lockfile.to_str().expect("lock path"),
                ])
                .output()
                .expect("run freeze");
            assert_eq!(freeze.status.code(), Some(2), "case {link_relative}");
            assert_eq!(
                fs::read_to_string(&lockfile).expect("preserved lock"),
                "existing-baseline\n"
            );
        }
    }
}

#[cfg(unix)]
#[test]
fn internal_plugin_aliases_reach_targets_beyond_physical_depth_across_supported_roots() {
    use std::os::unix::fs::symlink;

    let cases = [
        (
            ".codex/plugins/cache",
            "a/b/c/d/e/f/p",
            ".codex-plugin/plugin.json",
            "user",
        ),
        (
            ".gemini/extensions",
            "a/b/c/p",
            "gemini-extension.json",
            "user",
        ),
        (
            ".claude/plugins",
            "a/b/c/d/p",
            ".claude-plugin/plugin.json",
            "workspace",
        ),
        (
            ".codex/plugins",
            "a/b/c/d/p",
            ".codex-plugin/plugin.json",
            "workspace",
        ),
        (
            ".gemini/extensions",
            "a/b/c/d/e/p",
            "gemini-extension.json",
            "workspace",
        ),
    ];

    for (root_relative, target_relative, manifest_relative, scope) in cases {
        let fixture = tempdir().expect("tempdir");
        let home = fixture.path().join("home");
        let workspace = fixture.path().join("workspace");
        let boundary = if scope == "user" { &home } else { &workspace };
        let root = boundary.join(root_relative);
        let target = root.join(target_relative);
        let manifest = target.join(manifest_relative);
        fs::create_dir_all(manifest.parent().expect("manifest parent"))
            .expect("create deep plugin");
        fs::create_dir_all(&home).expect("home");
        fs::create_dir_all(&workspace).expect("workspace");
        fs::write(&manifest, r#"{"name":"deep-plugin","version":"1.0.0"}"#).expect("manifest");
        symlink(&target, root.join("active")).expect("internal plugin alias");

        let scan = Command::new(env!("CARGO_BIN_EXE_agentfreeze"))
            .args([
                "scan",
                "--home",
                home.to_str().expect("home path"),
                "--workspace",
                workspace.to_str().expect("workspace path"),
                "--scope",
                scope,
                "--json",
            ])
            .output()
            .expect("run scan");
        assert!(
            scan.status.success(),
            "case {root_relative}: {}",
            String::from_utf8_lossy(&scan.stderr)
        );
        let inventory: serde_json::Value = serde_json::from_slice(&scan.stdout).expect("scan JSON");
        assert_eq!(inventory["count"], 1, "case {root_relative}");
        assert_eq!(
            inventory["entries"][0]["name"], "deep-plugin",
            "case {root_relative}"
        );

        if scope == "user" {
            assert_user_alias_freeze_verifies(&home, &workspace, fixture.path(), root_relative);
        }
    }
}

#[cfg(unix)]
fn assert_user_alias_freeze_verifies(
    home: &std::path::Path,
    workspace: &std::path::Path,
    fixture: &std::path::Path,
    case_name: &str,
) {
    let lockfile = fixture.join("agentfreeze.lock");
    let freeze = Command::new(env!("CARGO_BIN_EXE_agentfreeze"))
        .args([
            "freeze",
            "--home",
            home.to_str().expect("home path"),
            "--workspace",
            workspace.to_str().expect("workspace path"),
            "--output",
            lockfile.to_str().expect("lock path"),
            "--json",
        ])
        .output()
        .expect("run freeze");
    assert!(
        freeze.status.success(),
        "case {case_name}: {}",
        String::from_utf8_lossy(&freeze.stderr)
    );
    let frozen: serde_json::Value = serde_json::from_slice(&freeze.stdout).expect("freeze JSON");
    assert_eq!(frozen["entries"], 1, "case {case_name}");

    let verify = Command::new(env!("CARGO_BIN_EXE_agentfreeze"))
        .args([
            "verify",
            "--home",
            home.to_str().expect("home path"),
            "--workspace",
            workspace.to_str().expect("workspace path"),
            "--lockfile",
            lockfile.to_str().expect("lock path"),
            "--json",
        ])
        .output()
        .expect("run verify");
    assert!(
        verify.status.success(),
        "case {case_name}: {}",
        String::from_utf8_lossy(&verify.stderr)
    );
    let verification: serde_json::Value =
        serde_json::from_slice(&verify.stdout).expect("verify JSON");
    assert_eq!(verification["status"], "clean", "case {case_name}");
    assert_eq!(verification["locked"], 1, "case {case_name}");
    assert_eq!(verification["observed"], 1, "case {case_name}");
}

#[cfg(unix)]
#[test]
fn plugin_discovery_symlink_failures_are_closed_across_supported_roots() {
    use std::os::unix::fs::symlink;

    let cases = [
        (
            ".codex/plugins/cache",
            "user",
            7_usize,
            ".codex-plugin/plugin.json",
        ),
        (".gemini/extensions", "user", 3, "gemini-extension.json"),
        (
            ".claude/plugins",
            "workspace",
            5,
            ".claude-plugin/plugin.json",
        ),
        (
            ".codex/plugins",
            "workspace",
            5,
            ".codex-plugin/plugin.json",
        ),
        (
            ".gemini/extensions",
            "workspace",
            5,
            "gemini-extension.json",
        ),
    ];

    for (root_relative, scope, max_depth, manifest_relative) in cases {
        for failure in ["broken", "cycle", "depth", "manifest"] {
            let fixture = tempdir().expect("tempdir");
            let home = fixture.path().join("home");
            let workspace = fixture.path().join("workspace");
            let boundary = if scope == "user" { &home } else { &workspace };
            let root = boundary.join(root_relative);
            fs::create_dir_all(&root).expect("plugin root");
            fs::create_dir_all(&home).expect("home");
            fs::create_dir_all(&workspace).expect("workspace");

            let expected_error = match failure {
                "broken" => {
                    symlink(root.join("missing"), root.join("active")).expect("broken alias");
                    "cannot resolve plugin discovery symlink"
                }
                "cycle" => {
                    symlink(&root, root.join("active")).expect("cyclic alias");
                    "plugin discovery symlink cycle"
                }
                "depth" => {
                    let mut parent = root.clone();
                    for index in 0..max_depth {
                        parent = parent.join(format!("d{index}"));
                    }
                    let target = root.join("target");
                    fs::create_dir_all(&parent).expect("depth boundary");
                    fs::create_dir_all(&target).expect("internal target");
                    symlink(&target, parent.join("active")).expect("depth alias");
                    "plugin discovery symlink exceeds maximum depth"
                }
                "manifest" => {
                    let payload = root.join("manifest-payload.json");
                    let manifest = root.join("active").join(manifest_relative);
                    fs::create_dir_all(manifest.parent().expect("manifest parent"))
                        .expect("manifest parent");
                    fs::write(&payload, r#"{"name":"linked","version":"1.0.0"}"#)
                        .expect("manifest payload");
                    symlink(&payload, &manifest).expect("manifest symlink");
                    "symlinked plugin manifests are not supported"
                }
                _ => unreachable!(),
            };

            let scan = Command::new(env!("CARGO_BIN_EXE_agentfreeze"))
                .args([
                    "scan",
                    "--home",
                    home.to_str().expect("home path"),
                    "--workspace",
                    workspace.to_str().expect("workspace path"),
                    "--scope",
                    scope,
                ])
                .output()
                .expect("run scan");
            assert_eq!(
                scan.status.code(),
                Some(2),
                "case {root_relative} {failure}"
            );
            assert!(
                String::from_utf8_lossy(&scan.stderr).contains(expected_error),
                "case {root_relative} {failure}: {}",
                String::from_utf8_lossy(&scan.stderr)
            );
        }
    }
}

#[derive(Clone, Copy)]
enum McpMalformedFixture {
    Json(&'static [&'static str]),
    Toml,
    ClaudeProject,
}

#[test]
fn semantic_mcp_malformed_data_fails_closed_across_supported_paths() {
    let cases = [
        (
            true,
            ".claude.json",
            "user",
            McpMalformedFixture::Json(&["mcpServers"]),
        ),
        (
            true,
            ".cursor/mcp.json",
            "user",
            McpMalformedFixture::Json(&["mcpServers"]),
        ),
        (
            true,
            ".gemini/settings.json",
            "user",
            McpMalformedFixture::Json(&["mcpServers"]),
        ),
        (
            true,
            "Library/Application Support/Claude/claude_desktop_config.json",
            "user",
            McpMalformedFixture::Json(&["mcpServers"]),
        ),
        (
            true,
            "Library/Application Support/Code/User/mcp.json",
            "user",
            McpMalformedFixture::Json(&["servers", "mcpServers"]),
        ),
        (
            true,
            "AppData/Roaming/Claude/claude_desktop_config.json",
            "user",
            McpMalformedFixture::Json(&["mcpServers"]),
        ),
        (
            true,
            "AppData/Roaming/Code/User/mcp.json",
            "user",
            McpMalformedFixture::Json(&["servers", "mcpServers"]),
        ),
        (
            true,
            ".codex/config.toml",
            "user",
            McpMalformedFixture::Toml,
        ),
        (
            false,
            ".mcp.json",
            "workspace",
            McpMalformedFixture::Json(&["mcpServers", "servers"]),
        ),
        (
            false,
            ".cursor/mcp.json",
            "workspace",
            McpMalformedFixture::Json(&["mcpServers"]),
        ),
        (
            false,
            ".gemini/settings.json",
            "workspace",
            McpMalformedFixture::Json(&["mcpServers"]),
        ),
        (
            false,
            ".vscode/mcp.json",
            "workspace",
            McpMalformedFixture::Json(&["servers", "mcpServers"]),
        ),
        (
            false,
            ".codex/config.toml",
            "workspace",
            McpMalformedFixture::Toml,
        ),
        (
            true,
            ".claude.json",
            "workspace",
            McpMalformedFixture::ClaudeProject,
        ),
    ];

    for (under_home, relative, scope, fixture_kind) in cases {
        assert_semantic_mcp_malformed_case(under_home, relative, scope, fixture_kind);
    }
}

fn assert_semantic_mcp_malformed_case(
    under_home: bool,
    relative: &str,
    scope: &str,
    fixture_kind: McpMalformedFixture,
) {
    let fixture = tempdir().expect("tempdir");
    let home = fixture.path().join("home");
    let workspace = fixture.path().join("workspace");
    let config = if under_home {
        home.join(relative)
    } else {
        workspace.join(relative)
    };
    fs::create_dir_all(config.parent().expect("config parent")).expect("config parent");
    fs::create_dir_all(&home).expect("home");
    fs::create_dir_all(&workspace).expect("workspace");
    fs::write(&config, valid_mcp_content(fixture_kind)).expect("valid config");

    let lockfile = fixture.path().join("agentfreeze.lock");
    let baseline = if scope == "user" {
        let freeze = freeze_to(&home, &workspace, &lockfile);
        assert!(
            freeze.status.success(),
            "baseline {relative}: {}",
            String::from_utf8_lossy(&freeze.stderr)
        );
        Some(fs::read(&lockfile).expect("baseline lock"))
    } else {
        None
    };

    for (variant, content) in malformed_mcp_contents(fixture_kind, &workspace) {
        fs::write(&config, &content).expect("malformed config");
        let scan = scan_scope(&home, &workspace, scope);
        assert_eq!(
            scan.status.code(),
            Some(2),
            "case {relative} {variant} content={content}: {}",
            String::from_utf8_lossy(&scan.stderr)
        );
        assert!(
            String::from_utf8_lossy(&scan.stderr).contains("must be"),
            "case {relative} {variant}: {}",
            String::from_utf8_lossy(&scan.stderr)
        );

        if let Some(expected_lock) = &baseline {
            let refreeze = freeze_to(&home, &workspace, &lockfile);
            assert_eq!(
                refreeze.status.code(),
                Some(2),
                "refreeze {relative} {variant}"
            );
            assert_eq!(
                fs::read(&lockfile).expect("preserved lock"),
                *expected_lock,
                "case {relative} {variant}"
            );
            let verification = verify(
                env!("CARGO_BIN_EXE_agentfreeze"),
                &home,
                &workspace,
                &lockfile,
            );
            assert_eq!(
                verification.status.code(),
                Some(2),
                "verify {relative} {variant}"
            );
        }
    }
}

fn valid_mcp_content(fixture: McpMalformedFixture) -> &'static str {
    match fixture {
        McpMalformedFixture::Json(_) => "{}",
        McpMalformedFixture::Toml => "",
        McpMalformedFixture::ClaudeProject => r#"{"projects":{}}"#,
    }
}

fn malformed_mcp_contents(
    fixture: McpMalformedFixture,
    workspace: &std::path::Path,
) -> Vec<(&'static str, String)> {
    match fixture {
        McpMalformedFixture::Json(keys) => {
            let mut cases = [
                ("root-array", "[]"),
                ("root-null", "null"),
                ("root-string", r#""wrong""#),
                ("root-number", "1"),
            ]
            .into_iter()
            .map(|(name, raw)| (name, raw.to_owned()))
            .collect::<Vec<_>>();
            cases.extend(
                [
                    ("container-array", "[]"),
                    ("container-null", "null"),
                    ("container-string", r#""wrong""#),
                    ("container-number", "1"),
                ]
                .into_iter()
                .map(|(name, raw)| (name, json_mcp_fixture(keys, raw))),
            );
            cases.push(("server-array", json_mcp_fixture(keys, r#"{"broken":[]}"#)));
            cases
        }
        McpMalformedFixture::Toml => vec![
            ("array", "mcp_servers = []\n".to_owned()),
            ("string", "mcp_servers = \"wrong\"\n".to_owned()),
            ("number", "mcp_servers = 1\n".to_owned()),
            ("server-array", "[mcp_servers]\nbroken = []\n".to_owned()),
        ],
        McpMalformedFixture::ClaudeProject => {
            let mut cases = [
                ("root-array", "[]"),
                ("root-null", "null"),
                ("root-string", r#""wrong""#),
                ("root-number", "1"),
            ]
            .into_iter()
            .map(|(name, raw)| (name, raw.to_owned()))
            .collect::<Vec<_>>();
            cases.extend(
                [
                    ("projects-array", "[]"),
                    ("projects-null", "null"),
                    ("projects-string", r#""wrong""#),
                    ("projects-number", "1"),
                ]
                .into_iter()
                .map(|(name, raw)| (name, claude_projects_fixture(raw))),
            );
            cases.extend(
                [
                    ("project-array", "[]"),
                    ("project-null", "null"),
                    ("project-string", r#""wrong""#),
                    ("project-number", "1"),
                ]
                .into_iter()
                .map(|(name, raw)| (name, claude_project_entry_fixture(workspace, raw))),
            );
            cases.extend(
                [
                    ("container-array", "[]"),
                    ("container-null", "null"),
                    ("container-string", r#""wrong""#),
                    ("container-number", "1"),
                ]
                .into_iter()
                .map(|(name, raw)| (name, claude_project_fixture(workspace, raw))),
            );
            cases.push((
                "server-array",
                claude_project_fixture(workspace, r#"{"broken":[]}"#),
            ));
            cases
        }
    }
}

fn json_mcp_fixture(keys: &[&str], raw_value: &str) -> String {
    let value: serde_json::Value = serde_json::from_str(raw_value).expect("fixture JSON");
    let object = keys
        .iter()
        .map(|key| ((*key).to_owned(), value.clone()))
        .collect::<serde_json::Map<_, _>>();
    serde_json::Value::Object(object).to_string()
}

fn claude_projects_fixture(raw_projects: &str) -> String {
    let projects: serde_json::Value =
        serde_json::from_str(raw_projects).expect("projects fixture JSON");
    serde_json::json!({"projects": projects}).to_string()
}

fn claude_project_entry_fixture(workspace: &std::path::Path, raw_project: &str) -> String {
    let project: serde_json::Value =
        serde_json::from_str(raw_project).expect("project fixture JSON");
    let canonical_workspace = workspace.canonicalize().expect("canonical workspace");
    let mut projects = serde_json::Map::new();
    projects.insert(canonical_workspace.to_string_lossy().into_owned(), project);
    serde_json::json!({"projects": projects}).to_string()
}

fn claude_project_fixture(workspace: &std::path::Path, raw_servers: &str) -> String {
    let servers: serde_json::Value =
        serde_json::from_str(raw_servers).expect("project fixture JSON");
    let canonical_workspace = workspace.canonicalize().expect("canonical workspace");
    let mut projects = serde_json::Map::new();
    projects.insert(
        canonical_workspace.to_string_lossy().into_owned(),
        serde_json::json!({"mcpServers": servers}),
    );
    serde_json::json!({"projects": projects}).to_string()
}

fn scan_scope(
    home: &std::path::Path,
    workspace: &std::path::Path,
    scope: &str,
) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_agentfreeze"))
        .args([
            "scan",
            "--home",
            home.to_str().expect("home path"),
            "--workspace",
            workspace.to_str().expect("workspace path"),
            "--scope",
            scope,
        ])
        .output()
        .expect("run scan")
}

fn freeze_to(
    home: &std::path::Path,
    workspace: &std::path::Path,
    lockfile: &std::path::Path,
) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_agentfreeze"))
        .args([
            "freeze",
            "--home",
            home.to_str().expect("home path"),
            "--workspace",
            workspace.to_str().expect("workspace path"),
            "--output",
            lockfile.to_str().expect("lock path"),
        ])
        .output()
        .expect("run freeze")
}

#[cfg(unix)]
#[test]
fn broken_mcp_config_parents_fail_closed_across_nested_routes() {
    use std::os::unix::fs::symlink;

    let cases = [
        (true, ".cursor/mcp.json", "user"),
        (true, ".gemini/settings.json", "user"),
        (
            true,
            "Library/Application Support/Claude/claude_desktop_config.json",
            "user",
        ),
        (
            true,
            "Library/Application Support/Code/User/mcp.json",
            "user",
        ),
        (
            true,
            "AppData/Roaming/Claude/claude_desktop_config.json",
            "user",
        ),
        (true, "AppData/Roaming/Code/User/mcp.json", "user"),
        (true, ".codex/config.toml", "user"),
        (false, ".cursor/mcp.json", "workspace"),
        (false, ".gemini/settings.json", "workspace"),
        (false, ".vscode/mcp.json", "workspace"),
        (false, ".codex/config.toml", "workspace"),
    ];

    for (index, (under_home, relative, scope)) in cases.into_iter().enumerate() {
        let fixture = tempdir().expect("tempdir");
        let home = fixture.path().join("home");
        let workspace = fixture.path().join("workspace");
        fs::create_dir_all(&home).expect("home");
        fs::create_dir_all(&workspace).expect("workspace");
        let baseline = user_baseline_if_needed(scope, &home, &workspace, fixture.path());
        let boundary = if under_home { &home } else { &workspace };
        let first = std::path::Path::new(relative)
            .components()
            .next()
            .expect("first component")
            .as_os_str();
        symlink(
            boundary.join(format!("missing-parent-{index}")),
            boundary.join(first),
        )
        .expect("broken config parent");

        assert_broken_discovery_fails(scope, &home, &workspace, baseline.as_ref(), relative);
    }
}

#[cfg(unix)]
#[test]
fn broken_skill_parents_and_children_fail_closed_across_supported_roots() {
    use std::os::unix::fs::symlink;

    let cases = [
        (".agents/skills", "user"),
        (".claude/skills", "user"),
        (".codex/skills", "user"),
        (".cursor/skills", "user"),
        (".cursor/skills-cursor", "user"),
        (".gemini/skills", "user"),
        (".agents/skills", "workspace"),
        (".claude/skills", "workspace"),
        (".codex/skills", "workspace"),
        (".cursor/skills", "workspace"),
        (".gemini/skills", "workspace"),
    ];

    for (index, (relative, scope)) in cases.into_iter().enumerate() {
        for failure in ["parent", "child"] {
            let fixture = tempdir().expect("tempdir");
            let home = fixture.path().join("home");
            let workspace = fixture.path().join("workspace");
            fs::create_dir_all(&home).expect("home");
            fs::create_dir_all(&workspace).expect("workspace");
            let baseline = user_baseline_if_needed(scope, &home, &workspace, fixture.path());
            let boundary = if scope == "user" { &home } else { &workspace };
            let root = boundary.join(relative);

            if failure == "parent" {
                let first = std::path::Path::new(relative)
                    .components()
                    .next()
                    .expect("first component")
                    .as_os_str();
                symlink(
                    boundary.join(format!("missing-skill-parent-{index}")),
                    boundary.join(first),
                )
                .expect("broken skill parent");
            } else {
                fs::create_dir_all(&root).expect("skill root");
                symlink(
                    root.join(format!("missing-skill-{index}")),
                    root.join("active"),
                )
                .expect("broken skill child");
            }

            assert_broken_discovery_fails(
                scope,
                &home,
                &workspace,
                baseline.as_ref(),
                &format!("{relative}:{failure}"),
            );
        }
    }
}

#[test]
fn regular_management_files_inside_skill_roots_are_not_skill_candidates() {
    let fixture = tempdir().expect("tempdir");
    let home = fixture.path().join("home");
    let workspace = fixture.path().join("workspace");
    let managed_root = home.join(".cursor/skills-cursor");
    fs::create_dir_all(&managed_root).expect("managed skill root");
    fs::create_dir_all(&workspace).expect("workspace");
    fs::write(
        managed_root.join(".cursor-managed-skills-manifest.json"),
        r#"{"version":1}"#,
    )
    .expect("management file");

    let scan = scan_scope(&home, &workspace, "user");
    assert!(
        scan.status.success(),
        "{}",
        String::from_utf8_lossy(&scan.stderr)
    );
    assert!(String::from_utf8_lossy(&scan.stdout).contains("Found 0 extensions"));
}

#[cfg(unix)]
#[test]
fn broken_plugin_and_registry_parents_fail_closed() {
    use std::os::unix::fs::symlink;

    let cases = [
        (".codex/plugins", "user", "user-codex"),
        (".gemini", "user", "user-gemini"),
        (".claude", "workspace", "workspace-claude"),
        (".codex", "workspace", "workspace-codex"),
        (".gemini", "workspace", "workspace-gemini"),
        (".claude/plugins", "user", "claude-registry"),
    ];

    for (index, (broken_parent, scope, case_name)) in cases.into_iter().enumerate() {
        let fixture = tempdir().expect("tempdir");
        let home = fixture.path().join("home");
        let workspace = fixture.path().join("workspace");
        fs::create_dir_all(&home).expect("home");
        fs::create_dir_all(&workspace).expect("workspace");
        let baseline = user_baseline_if_needed(scope, &home, &workspace, fixture.path());
        let boundary = if scope == "user" { &home } else { &workspace };
        let link = boundary.join(broken_parent);
        fs::create_dir_all(link.parent().expect("broken parent owner")).expect("parent owner");
        symlink(
            boundary.join(format!("missing-plugin-parent-{index}")),
            &link,
        )
        .expect("broken plugin parent");

        assert_broken_discovery_fails(scope, &home, &workspace, baseline.as_ref(), case_name);
    }
}

#[cfg(unix)]
#[test]
fn missing_paths_under_internal_aliases_are_absent_but_external_aliases_fail() {
    use std::os::unix::fs::symlink;

    let fixture = tempdir().expect("tempdir");
    let home = fixture.path().join("home");
    let workspace = fixture.path().join("workspace");
    let internal = home.join("internal-cursor");
    fs::create_dir_all(&internal).expect("internal target");
    fs::create_dir_all(&workspace).expect("workspace");
    symlink(&internal, home.join(".cursor")).expect("internal parent alias");

    let internal_scan = scan_scope(&home, &workspace, "user");
    assert!(
        internal_scan.status.success(),
        "{}",
        String::from_utf8_lossy(&internal_scan.stderr)
    );

    let external_home = fixture.path().join("external-home");
    let external_workspace = fixture.path().join("external-workspace");
    let outside = fixture.path().join("outside-cursor");
    fs::create_dir_all(&external_home).expect("external home");
    fs::create_dir_all(&external_workspace).expect("external workspace");
    fs::create_dir_all(&outside).expect("outside target");
    symlink(&outside, external_home.join(".cursor")).expect("external parent alias");

    let external_scan = scan_scope(&external_home, &external_workspace, "user");
    assert_eq!(external_scan.status.code(), Some(2));
    assert!(
        String::from_utf8_lossy(&external_scan.stderr).contains("ancestor escapes its boundary"),
        "{}",
        String::from_utf8_lossy(&external_scan.stderr)
    );
}

fn user_baseline_if_needed(
    scope: &str,
    home: &std::path::Path,
    workspace: &std::path::Path,
    fixture: &std::path::Path,
) -> Option<(std::path::PathBuf, Vec<u8>)> {
    if scope != "user" {
        return None;
    }
    fs::write(
        home.join(".claude.json"),
        r#"{"mcpServers":{"baseline":{"command":"baseline"}}}"#,
    )
    .expect("baseline config");
    let lockfile = fixture.join("agentfreeze.lock");
    let freeze = freeze_to(home, workspace, &lockfile);
    assert!(
        freeze.status.success(),
        "{}",
        String::from_utf8_lossy(&freeze.stderr)
    );
    let bytes = fs::read(&lockfile).expect("baseline lock");
    Some((lockfile, bytes))
}

fn assert_broken_discovery_fails(
    scope: &str,
    home: &std::path::Path,
    workspace: &std::path::Path,
    baseline: Option<&(std::path::PathBuf, Vec<u8>)>,
    case_name: &str,
) {
    let scan = scan_scope(home, workspace, scope);
    assert_eq!(
        scan.status.code(),
        Some(2),
        "scan {case_name}: {}",
        String::from_utf8_lossy(&scan.stderr)
    );
    assert!(
        String::from_utf8_lossy(&scan.stderr).contains("broken discovery symlink"),
        "scan {case_name}: {}",
        String::from_utf8_lossy(&scan.stderr)
    );

    if let Some((lockfile, expected)) = baseline {
        let refreeze = freeze_to(home, workspace, lockfile);
        assert_eq!(
            refreeze.status.code(),
            Some(2),
            "refreeze {case_name}: {}",
            String::from_utf8_lossy(&refreeze.stderr)
        );
        assert_eq!(
            fs::read(lockfile).expect("preserved baseline"),
            *expected,
            "case {case_name}"
        );
        let verification = verify(env!("CARGO_BIN_EXE_agentfreeze"), home, workspace, lockfile);
        assert_eq!(
            verification.status.code(),
            Some(2),
            "verify {case_name}: {}",
            String::from_utf8_lossy(&verification.stderr)
        );
    }
}

#[cfg(unix)]
#[test]
fn mcp_config_symlink_escapes_fail_closed_across_supported_paths() {
    use std::os::unix::fs::symlink;

    let json = r#"{"mcpServers":{"foreign":{"command":"sentinel"}}}"#;
    let vscode_json = r#"{"servers":{"foreign":{"command":"sentinel"}}}"#;
    let toml = "[mcp_servers.foreign]\ncommand = \"sentinel\"\n";
    let cases = [
        (true, ".claude.json", "user", json),
        (true, ".cursor/mcp.json", "user", json),
        (true, ".gemini/settings.json", "user", json),
        (
            true,
            "Library/Application Support/Claude/claude_desktop_config.json",
            "user",
            json,
        ),
        (
            true,
            "Library/Application Support/Code/User/mcp.json",
            "user",
            vscode_json,
        ),
        (
            true,
            "AppData/Roaming/Claude/claude_desktop_config.json",
            "user",
            json,
        ),
        (
            true,
            "AppData/Roaming/Code/User/mcp.json",
            "user",
            vscode_json,
        ),
        (true, ".codex/config.toml", "user", toml),
        (false, ".mcp.json", "workspace", json),
        (false, ".cursor/mcp.json", "workspace", json),
        (false, ".gemini/settings.json", "workspace", json),
        (false, ".vscode/mcp.json", "workspace", vscode_json),
        (false, ".codex/config.toml", "workspace", toml),
        (true, ".claude.json", "workspace", r#"{"projects":{}}"#),
    ];

    for (index, (under_home, relative, scope, content)) in cases.iter().enumerate() {
        let fixture = tempdir().expect("tempdir");
        let home = fixture.path().join("home");
        let workspace = fixture.path().join("workspace");
        let logical = if *under_home {
            home.join(relative)
        } else {
            workspace.join(relative)
        };
        let outside = fixture.path().join(format!("outside-config-{index}"));
        fs::create_dir_all(logical.parent().expect("config parent")).expect("config parent");
        fs::create_dir_all(&home).expect("home");
        fs::create_dir_all(&workspace).expect("workspace");
        fs::write(&outside, content).expect("outside config");
        symlink(&outside, &logical).expect("escaped config link");

        let scan = Command::new(env!("CARGO_BIN_EXE_agentfreeze"))
            .args([
                "scan",
                "--home",
                home.to_str().expect("home path"),
                "--workspace",
                workspace.to_str().expect("workspace path"),
                "--scope",
                scope,
            ])
            .output()
            .expect("run scan");
        assert_eq!(scan.status.code(), Some(2), "case {scope}:{relative}");
        assert!(
            String::from_utf8_lossy(&scan.stderr).contains("escapes its boundary"),
            "case {scope}:{relative}: {}",
            String::from_utf8_lossy(&scan.stderr)
        );
    }
}

#[cfg(unix)]
#[test]
fn broken_and_parent_escaped_mcp_config_symlinks_fail_closed() {
    use std::os::unix::fs::symlink;

    let fixture = tempdir().expect("tempdir");
    let home = fixture.path().join("home");
    let workspace = fixture.path().join("workspace");
    fs::create_dir_all(&home).expect("home");
    fs::create_dir_all(&workspace).expect("workspace");
    symlink(
        fixture.path().join("missing.json"),
        home.join(".claude.json"),
    )
    .expect("broken config link");

    let broken = Command::new(env!("CARGO_BIN_EXE_agentfreeze"))
        .args([
            "scan",
            "--home",
            home.to_str().expect("home path"),
            "--workspace",
            workspace.to_str().expect("workspace path"),
            "--scope",
            "user",
        ])
        .output()
        .expect("run broken scan");
    assert_eq!(broken.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&broken.stderr).contains("broken discovery symlink"));

    fs::remove_file(home.join(".claude.json")).expect("remove broken link");
    let outside_cursor = fixture.path().join("outside-cursor");
    fs::create_dir_all(&outside_cursor).expect("outside cursor");
    fs::write(
        outside_cursor.join("mcp.json"),
        r#"{"mcpServers":{"foreign":{"command":"sentinel"}}}"#,
    )
    .expect("outside cursor config");
    symlink(&outside_cursor, home.join(".cursor")).expect("escaped parent link");

    let parent = Command::new(env!("CARGO_BIN_EXE_agentfreeze"))
        .args([
            "scan",
            "--home",
            home.to_str().expect("home path"),
            "--workspace",
            workspace.to_str().expect("workspace path"),
            "--scope",
            "user",
        ])
        .output()
        .expect("run parent escape scan");
    assert_eq!(parent.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&parent.stderr).contains("escapes its boundary"));
}

fn verify(
    binary: &str,
    home: &std::path::Path,
    workspace: &std::path::Path,
    lockfile: &std::path::Path,
) -> std::process::Output {
    Command::new(binary)
        .args([
            "verify",
            "--home",
            home.to_str().expect("home path"),
            "--workspace",
            workspace.to_str().expect("workspace path"),
            "--lockfile",
            lockfile.to_str().expect("lockfile path"),
        ])
        .output()
        .expect("run verify")
}
