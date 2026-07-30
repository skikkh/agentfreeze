use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::model::{Entry, EntryScope, LOCKFILE_SCHEMA, LOCKFILE_VERSION, Lockfile};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ChangeKind {
    Missing,
    Modified,
    Untracked,
}

impl std::fmt::Display for ChangeKind {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Missing => "missing",
            Self::Modified => "modified",
            Self::Untracked => "untracked",
        })
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Change {
    pub kind: ChangeKind,
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_digest: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actual_digest: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Verification {
    pub status: VerificationStatus,
    pub locked: usize,
    pub observed: usize,
    pub matched: usize,
    pub changes: Vec<Change>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum VerificationStatus {
    Clean,
    Drift,
}

/// Builds a canonical in-memory lockfile from a completed inventory.
#[must_use]
pub fn build_lockfile(entries: Vec<Entry>) -> Lockfile {
    let mut entries = entries;
    entries.sort_by(|left, right| left.id.cmp(&right.id));
    Lockfile {
        schema: LOCKFILE_SCHEMA.to_owned(),
        lockfile_version: LOCKFILE_VERSION,
        generated_by: format!("agentfreeze {}", env!("CARGO_PKG_VERSION")),
        scan_scopes: vec![EntryScope::User],
        entries,
    }
}

/// Serializes a validated lockfile using deterministic JSON formatting.
///
/// # Errors
///
/// Returns an error when validation or JSON serialization fails.
pub fn serialize_lockfile(lockfile: &Lockfile) -> Result<Vec<u8>> {
    validate_lockfile(lockfile)?;
    let mut bytes = serde_json::to_vec_pretty(lockfile).context("cannot serialize lockfile")?;
    bytes.push(b'\n');
    Ok(bytes)
}

/// Atomically writes a validated lockfile and returns its SHA-256 digest.
///
/// # Errors
///
/// Returns an error when validation fails or the destination cannot be created or replaced.
pub fn write_lockfile(path: &Path, lockfile: &Lockfile) -> Result<String> {
    let bytes = serialize_lockfile(lockfile)?;
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)
        .with_context(|| format!("cannot create lockfile directory {}", parent.display()))?;
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .context("lockfile path must have a UTF-8 file name")?;
    let temporary = parent.join(format!(".{file_name}.tmp.{}", std::process::id()));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .with_context(|| format!("cannot create temporary lockfile {}", temporary.display()))?;
    if let Err(error) = (|| -> std::io::Result<()> {
        file.write_all(&bytes)?;
        file.sync_all()
    })() {
        let _ = fs::remove_file(&temporary);
        return Err(error)
            .with_context(|| format!("cannot write temporary lockfile {}", temporary.display()));
    }
    drop(file);
    if let Err(error) = replace_file(&temporary, path) {
        let _ = fs::remove_file(&temporary);
        return Err(error);
    }
    Ok(format!("sha256:{:x}", Sha256::digest(&bytes)))
}

#[cfg(not(windows))]
fn replace_file(temporary: &Path, destination: &Path) -> Result<()> {
    fs::rename(temporary, destination)
        .with_context(|| format!("cannot replace lockfile {}", destination.display()))
}

#[cfg(windows)]
fn replace_file(temporary: &Path, destination: &Path) -> Result<()> {
    let file_name = destination
        .file_name()
        .and_then(|name| name.to_str())
        .context("lockfile path must have a UTF-8 file name")?;
    let parent = destination.parent().unwrap_or_else(|| Path::new("."));
    let backup = parent.join(format!(".{file_name}.backup.{}", std::process::id()));
    if backup.exists() {
        bail!("refusing to overwrite stale backup {}", backup.display());
    }

    let had_destination = destination.exists();
    if had_destination {
        fs::rename(destination, &backup).with_context(|| {
            format!(
                "cannot stage existing lockfile {} as {}",
                destination.display(),
                backup.display()
            )
        })?;
    }
    if let Err(replace_error) = fs::rename(temporary, destination) {
        if had_destination {
            fs::rename(&backup, destination).with_context(|| {
                format!(
                    "cannot install new lockfile and cannot restore backup {} after: {replace_error}",
                    backup.display()
                )
            })?;
        }
        return Err(replace_error)
            .with_context(|| format!("cannot replace lockfile {}", destination.display()));
    }
    if had_destination {
        fs::remove_file(&backup)
            .with_context(|| format!("cannot remove lockfile backup {}", backup.display()))?;
    }
    Ok(())
}

/// Loads and validates an `AgentFreeze` lockfile.
///
/// # Errors
///
/// Returns an error when the file cannot be read, parsed, or validated.
pub fn load_lockfile(path: &Path) -> Result<Lockfile> {
    let bytes = fs::read(path).with_context(|| format!("cannot read {}", path.display()))?;
    let lockfile: Lockfile = serde_json::from_slice(&bytes)
        .with_context(|| format!("invalid JSON in {}", path.display()))?;
    validate_lockfile(&lockfile)?;
    Ok(lockfile)
}

/// Checks schema version, canonical ordering, identifiers, scopes, and digest syntax.
///
/// # Errors
///
/// Returns an error describing the first invalid lockfile invariant.
pub fn validate_lockfile(lockfile: &Lockfile) -> Result<()> {
    if lockfile.schema != LOCKFILE_SCHEMA {
        bail!(
            "unsupported $schema {}; expected {}",
            lockfile.schema,
            LOCKFILE_SCHEMA
        );
    }
    if lockfile.lockfile_version != LOCKFILE_VERSION {
        bail!(
            "unsupported lockfileVersion {}; supported version is {}",
            lockfile.lockfile_version,
            LOCKFILE_VERSION
        );
    }
    if lockfile.scan_scopes.as_slice() != [EntryScope::User] {
        bail!("AgentFreeze v1 lockfiles must use scanScopes [\"user\"]");
    }

    let mut previous = None;
    for entry in &lockfile.entries {
        if entry.scope != EntryScope::User {
            bail!("AgentFreeze v1 lockfiles cannot contain workspace entries");
        }
        if entry.id.trim().is_empty()
            || entry.name.trim().is_empty()
            || entry.source.trim().is_empty()
        {
            bail!("lockfile entries require non-empty id, name, and source");
        }
        if entry.source != "$HOME" && !entry.source.starts_with("$HOME/") {
            bail!("AgentFreeze v1 user entry sources must be anchored under $HOME");
        }
        let expected_id = format!("{}:{}:user:{}", entry.ecosystem, entry.kind, entry.source);
        if entry.id != expected_id {
            bail!("lockfile entry id does not match its ecosystem, kind, scope, and source");
        }
        validate_digest(&entry.digest)?;
        if let Some(previous_id) = previous {
            if previous_id >= entry.id.as_str() {
                bail!("lockfile entries must be sorted by unique id");
            }
        }
        previous = Some(entry.id.as_str());
    }
    Ok(())
}

fn validate_digest(digest: &str) -> Result<()> {
    let Some(hex) = digest.strip_prefix("sha256:") else {
        bail!("digest must start with sha256:");
    };
    if hex.len() != 64
        || !hex
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        bail!("digest must contain 64 lowercase hexadecimal characters");
    }
    Ok(())
}

/// Compares a trusted lockfile with a fresh inventory.
#[must_use]
pub fn compare(lockfile: &Lockfile, observed: &[Entry], allow_untracked: bool) -> Verification {
    let locked = lockfile
        .entries
        .iter()
        .map(|entry| (entry.id.as_str(), entry))
        .collect::<BTreeMap<_, _>>();
    let observed_map = observed
        .iter()
        .map(|entry| (entry.id.as_str(), entry))
        .collect::<BTreeMap<_, _>>();
    let mut changes = Vec::new();
    let mut matched = 0;

    for (id, expected) in &locked {
        match observed_map.get(id) {
            None => changes.push(Change {
                kind: ChangeKind::Missing,
                id: (*id).to_owned(),
                expected_digest: Some(expected.digest.clone()),
                actual_digest: None,
            }),
            Some(actual)
                if expected.digest != actual.digest
                    || expected.version != actual.version
                    || expected.revision != actual.revision =>
            {
                changes.push(Change {
                    kind: ChangeKind::Modified,
                    id: (*id).to_owned(),
                    expected_digest: Some(expected.digest.clone()),
                    actual_digest: Some(actual.digest.clone()),
                });
            }
            Some(_) => matched += 1,
        }
    }

    if !allow_untracked {
        for (id, actual) in &observed_map {
            if !locked.contains_key(id) {
                changes.push(Change {
                    kind: ChangeKind::Untracked,
                    id: (*id).to_owned(),
                    expected_digest: None,
                    actual_digest: Some(actual.digest.clone()),
                });
            }
        }
    }

    changes.sort_by(|left, right| left.id.cmp(&right.id));
    Verification {
        status: if changes.is_empty() {
            VerificationStatus::Clean
        } else {
            VerificationStatus::Drift
        },
        locked: locked.len(),
        observed: observed_map.len(),
        matched,
        changes,
    }
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::{
        ChangeKind, VerificationStatus, build_lockfile, compare, load_lockfile, write_lockfile,
    };
    use crate::model::{Ecosystem, Entry, EntryScope, ExtensionKind};

    fn entry(name: &str, digest_byte: char) -> Entry {
        let source = format!("$HOME/.claude/skills/{name}");
        Entry {
            id: format!("claude:skill:user:{source}"),
            ecosystem: Ecosystem::Claude,
            kind: ExtensionKind::Skill,
            scope: EntryScope::User,
            name: name.to_owned(),
            source,
            version: None,
            revision: None,
            digest: format!("sha256:{}", digest_byte.to_string().repeat(64)),
            files: 1,
            bytes: 10,
            symlinks: 0,
        }
    }

    #[test]
    fn lockfile_round_trips_and_is_canonical() {
        let fixture = tempdir().expect("tempdir");
        let path = fixture.path().join("agentfreeze.lock");
        let lockfile = build_lockfile(vec![entry("z", 'a'), entry("a", 'b')]);
        let digest = write_lockfile(&path, &lockfile).expect("write lockfile");
        assert!(digest.starts_with("sha256:"));
        let loaded = load_lockfile(&path).expect("load lockfile");
        assert!(loaded.entries[0].id.ends_with("/a"));
        assert_eq!(loaded, lockfile);
    }

    #[test]
    fn lockfile_rejects_a_user_entry_outside_the_home_anchor() {
        let fixture = tempdir().expect("tempdir");
        let path = fixture.path().join("agentfreeze.lock");
        let mut invalid = entry("outside", 'a');
        invalid.source = "$WORKSPACE/.claude/skills/outside".to_owned();
        invalid.id = "claude:skill:user:$WORKSPACE/.claude/skills/outside".to_owned();

        let error = write_lockfile(&path, &build_lockfile(vec![invalid]))
            .expect_err("workspace-anchored user source must fail");
        assert!(
            error
                .to_string()
                .contains("user entry sources must be anchored under $HOME")
        );
    }

    #[test]
    fn lockfile_rejects_an_id_that_does_not_match_entry_metadata() {
        let fixture = tempdir().expect("tempdir");
        let path = fixture.path().join("agentfreeze.lock");
        let mut invalid = entry("mismatch", 'a');
        invalid.source = "$HOME/.claude/skills/other".to_owned();

        let error = write_lockfile(&path, &build_lockfile(vec![invalid]))
            .expect_err("metadata mismatch must fail");
        assert!(error.to_string().contains("entry id does not match"));
    }

    #[test]
    fn comparison_reports_missing_modified_and_untracked() {
        let lockfile = build_lockfile(vec![entry("missing", 'a'), entry("modified", 'b')]);
        let observed = vec![entry("modified", 'c'), entry("untracked", 'd')];
        let verification = compare(&lockfile, &observed, false);
        assert_eq!(verification.status, VerificationStatus::Drift);
        assert_eq!(verification.changes.len(), 3);
        assert!(
            verification
                .changes
                .iter()
                .any(|change| change.kind == ChangeKind::Missing)
        );
        assert!(
            verification
                .changes
                .iter()
                .any(|change| change.kind == ChangeKind::Modified)
        );
        assert!(
            verification
                .changes
                .iter()
                .any(|change| change.kind == ChangeKind::Untracked)
        );
    }
}
