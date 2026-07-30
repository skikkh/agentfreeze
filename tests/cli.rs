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
