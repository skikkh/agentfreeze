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
