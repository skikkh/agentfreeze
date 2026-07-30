use std::fs::{self, File};
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

use crate::model::EntryScope;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HashResult {
    pub digest: String,
    pub files: u64,
    pub bytes: u64,
    pub symlinks: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
enum RecordKind {
    Directory,
    File,
    Symlink,
}

#[derive(Debug)]
struct Record {
    kind: RecordKind,
    relative: String,
    absolute: PathBuf,
}

/// Computes the canonical `AgentFreeze` v1 digest for a file or directory.
///
/// # Errors
///
/// Returns an error when the source cannot be read completely, changes type or size while being
/// hashed, contains a non-UTF-8 path, or contains an unsupported filesystem entry.
pub fn hash_path(path: &Path) -> Result<HashResult> {
    let metadata =
        fs::symlink_metadata(path).with_context(|| format!("cannot inspect {}", path.display()))?;
    let mut records = Vec::new();

    if metadata.file_type().is_symlink() {
        bail!(
            "symlinked extension roots are not supported; use a real directory: {}",
            path.display()
        );
    } else if metadata.is_file() {
        records.push(Record {
            kind: RecordKind::File,
            relative: ".".to_owned(),
            absolute: path.to_path_buf(),
        });
    } else if metadata.is_dir() {
        collect_records(path, path, &mut records)?;
    } else {
        bail!("unsupported extension source type: {}", path.display());
    }

    records.sort_by(|left, right| {
        left.relative
            .cmp(&right.relative)
            .then(left.kind.cmp(&right.kind))
    });

    hash_records(records)
}

fn hash_records(records: Vec<Record>) -> Result<HashResult> {
    let mut hasher = Sha256::new();
    feed_field(&mut hasher, b"agentfreeze-path-v1");
    let mut files = 0_u64;
    let mut bytes = 0_u64;
    let mut symlinks = 0_u64;
    for record in records {
        match record.kind {
            RecordKind::Directory => {
                feed_field(&mut hasher, b"directory");
                feed_field(&mut hasher, record.relative.as_bytes());
            }
            RecordKind::File => {
                feed_field(&mut hasher, b"file");
                feed_field(&mut hasher, record.relative.as_bytes());
                let metadata = fs::symlink_metadata(&record.absolute)
                    .with_context(|| format!("cannot inspect {}", record.absolute.display()))?;
                if !metadata.is_file() || metadata.file_type().is_symlink() {
                    bail!(
                        "source changed type while hashing: {}",
                        record.absolute.display()
                    );
                }
                feed_field(&mut hasher, &metadata.len().to_be_bytes());
                let mut reader = BufReader::new(
                    File::open(&record.absolute)
                        .with_context(|| format!("cannot read {}", record.absolute.display()))?,
                );
                let mut buffer = vec![0_u8; 64 * 1024];
                let mut observed = 0_u64;
                loop {
                    let count = reader
                        .read(&mut buffer)
                        .with_context(|| format!("cannot read {}", record.absolute.display()))?;
                    if count == 0 {
                        break;
                    }
                    hasher.update(&buffer[..count]);
                    observed = observed
                        .checked_add(u64::try_from(count).context("file is too large")?)
                        .context("file byte count overflow")?;
                }
                if observed != metadata.len() {
                    bail!(
                        "source changed size while hashing: {}",
                        record.absolute.display()
                    );
                }
                files += 1;
                bytes = bytes
                    .checked_add(observed)
                    .context("extension byte count overflow")?;
            }
            RecordKind::Symlink => {
                feed_field(&mut hasher, b"symlink");
                feed_field(&mut hasher, record.relative.as_bytes());
                let metadata = fs::symlink_metadata(&record.absolute)
                    .with_context(|| format!("cannot inspect {}", record.absolute.display()))?;
                if !metadata.file_type().is_symlink() {
                    bail!(
                        "source changed type while hashing: {}",
                        record.absolute.display()
                    );
                }
                let target = fs::read_link(&record.absolute).with_context(|| {
                    format!("cannot read symlink {}", record.absolute.display())
                })?;
                let target = portable_relative(&target)?;
                feed_field(&mut hasher, target.as_bytes());
                symlinks += 1;
            }
        }
    }

    Ok(HashResult {
        digest: format!("sha256:{:x}", hasher.finalize()),
        files,
        bytes,
        symlinks,
    })
}

fn collect_records(root: &Path, directory: &Path, records: &mut Vec<Record>) -> Result<()> {
    let mut children = fs::read_dir(directory)
        .with_context(|| format!("cannot list {}", directory.display()))?
        .collect::<std::io::Result<Vec<_>>>()
        .with_context(|| format!("cannot list {}", directory.display()))?;
    children.sort_by_key(std::fs::DirEntry::file_name);

    for child in children {
        let absolute = child.path();
        let relative = absolute
            .strip_prefix(root)
            .context("extension path escaped its root")?;
        let relative = portable_relative(relative)?;
        let metadata = fs::symlink_metadata(&absolute)
            .with_context(|| format!("cannot inspect {}", absolute.display()))?;

        if metadata.file_type().is_symlink() {
            records.push(Record {
                kind: RecordKind::Symlink,
                relative,
                absolute,
            });
        } else if metadata.is_dir() {
            records.push(Record {
                kind: RecordKind::Directory,
                relative,
                absolute: absolute.clone(),
            });
            collect_records(root, &absolute, records)?;
        } else if metadata.is_file() {
            records.push(Record {
                kind: RecordKind::File,
                relative,
                absolute,
            });
        } else {
            bail!("unsupported file type in extension: {}", absolute.display());
        }
    }
    Ok(())
}

fn portable_relative(path: &Path) -> Result<String> {
    let value = path
        .to_str()
        .with_context(|| format!("non-UTF-8 path in extension: {}", path.display()))?;
    Ok(value.replace('\\', "/"))
}

fn feed_field(hasher: &mut Sha256, bytes: &[u8]) {
    hasher.update(u64::try_from(bytes.len()).unwrap_or(u64::MAX).to_be_bytes());
    hasher.update(bytes);
}

/// Computes the canonical `AgentFreeze` v1 digest for a sanitized MCP server definition.
///
/// # Errors
///
/// Returns an error when the value cannot be serialized or its size cannot be represented.
pub fn hash_json_value(value: &Value) -> Result<HashResult> {
    let canonical = serde_json::to_vec(value).context("cannot canonicalize MCP configuration")?;
    let mut hasher = Sha256::new();
    feed_field(&mut hasher, b"agentfreeze-mcp-v1");
    feed_field(&mut hasher, &canonical);
    Ok(HashResult {
        digest: format!("sha256:{:x}", hasher.finalize()),
        files: 1,
        bytes: u64::try_from(canonical.len()).context("MCP configuration is too large")?,
        symlinks: 0,
    })
}

/// Removes likely secret values and replaces machine-specific path prefixes with stable anchors.
#[must_use]
pub fn sanitize_mcp_value(
    value: &Value,
    workspace: &Path,
    home: &Path,
    scope: EntryScope,
) -> Value {
    sanitize_inner(value, None, workspace, home, scope, false)
}

fn sanitize_inner(
    value: &Value,
    key: Option<&str>,
    workspace: &Path,
    home: &Path,
    scope: EntryScope,
    redact_scalar: bool,
) -> Value {
    if redact_scalar || key.is_some_and(is_secret_key) {
        return redact_shape(value);
    }

    match value {
        Value::Object(object) => {
            let redact_children =
                key.is_some_and(|name| is_header_container(name) || is_environment_container(name));
            let mut sanitized = Map::new();
            let mut keys = object.keys().collect::<Vec<_>>();
            keys.sort();
            for child_key in keys {
                let child = &object[child_key];
                sanitized.insert(
                    child_key.clone(),
                    sanitize_inner(
                        child,
                        Some(child_key),
                        workspace,
                        home,
                        scope,
                        redact_children,
                    ),
                );
            }
            Value::Object(sanitized)
        }
        Value::Array(items) if key == Some("args") => {
            Value::Array(sanitize_args(items, workspace, home, scope))
        }
        Value::Array(items) => Value::Array(
            items
                .iter()
                .map(|item| sanitize_inner(item, None, workspace, home, scope, false))
                .collect(),
        ),
        Value::String(string) => {
            let mut normalized = normalize_anchored_string(string, workspace, home, scope);
            if key.is_some_and(is_url_key) {
                normalized = redact_url(&normalized);
            }
            Value::String(normalized)
        }
        _ => value.clone(),
    }
}

fn redact_shape(value: &Value) -> Value {
    match value {
        Value::Array(items) => Value::Array(items.iter().map(redact_shape).collect()),
        Value::Object(object) => {
            let mut redacted = Map::new();
            let mut keys = object.keys().collect::<Vec<_>>();
            keys.sort();
            for key in keys {
                redacted.insert(key.clone(), redact_shape(&object[key]));
            }
            Value::Object(redacted)
        }
        _ => Value::String("<redacted>".to_owned()),
    }
}

fn sanitize_args(items: &[Value], workspace: &Path, home: &Path, scope: EntryScope) -> Vec<Value> {
    let mut redact_next = false;
    items
        .iter()
        .map(|item| {
            let Value::String(argument) = item else {
                return sanitize_inner(item, None, workspace, home, scope, redact_next);
            };
            if redact_next {
                redact_next = false;
                return Value::String("<redacted>".to_owned());
            }
            if argument.starts_with('-') {
                if let Some((flag, _value)) = argument.split_once('=') {
                    if is_secret_key(flag.trim_start_matches('-')) {
                        return Value::String(format!("{flag}=<redacted>"));
                    }
                }
                if is_secret_key(argument.trim_start_matches('-')) {
                    redact_next = true;
                }
            }
            let normalized = normalize_anchored_string(argument, workspace, home, scope);
            Value::String(if normalized.contains("://") {
                redact_url(&normalized)
            } else {
                normalized
            })
        })
        .collect()
}

fn normalize_anchored_string(
    value: &str,
    workspace: &Path,
    home: &Path,
    scope: EntryScope,
) -> String {
    let workspace = workspace.to_string_lossy();
    let home = home.to_string_lossy();
    let anchors = match scope {
        EntryScope::User => [("$HOME", home.as_ref()), ("$WORKSPACE", workspace.as_ref())],
        EntryScope::Workspace => [("$WORKSPACE", workspace.as_ref()), ("$HOME", home.as_ref())],
    };
    for (anchor, prefix) in anchors {
        if value == prefix {
            return anchor.to_owned();
        }
        if let Some(suffix) = value.strip_prefix(prefix) {
            if suffix.starts_with(['/', '\\']) {
                return format!("{anchor}{}", suffix.replace('\\', "/"));
            }
        }
    }
    value.to_owned()
}

fn redact_url(value: &str) -> String {
    let (without_fragment, had_fragment) = value
        .split_once('#')
        .map_or((value, false), |(base, _)| (base, true));
    let (base, query) = without_fragment
        .split_once('?')
        .map_or((without_fragment, None), |(base, query)| {
            (base, Some(query))
        });

    let mut sanitized_base = base.to_owned();
    if let Some(scheme_end) = base.find("://") {
        let authority_start = scheme_end + 3;
        let authority_end = base[authority_start..]
            .find('/')
            .map_or(base.len(), |offset| authority_start + offset);
        let authority = &base[authority_start..authority_end];
        if let Some(at) = authority.rfind('@') {
            sanitized_base = format!(
                "{}<redacted>@{}{}",
                &base[..authority_start],
                &authority[at + 1..],
                &base[authority_end..]
            );
        }
    }

    if let Some(query) = query {
        let redacted = query
            .split('&')
            .map(|pair| {
                let key = pair.split_once('=').map_or(pair, |(key, _)| key);
                format!("{key}=<redacted>")
            })
            .collect::<Vec<_>>()
            .join("&");
        sanitized_base.push('?');
        sanitized_base.push_str(&redacted);
    }
    if had_fragment {
        sanitized_base.push_str("#<redacted>");
    }
    sanitized_base
}

fn is_header_container(key: &str) -> bool {
    matches!(
        normalize_key(key).as_str(),
        "header" | "headers" | "http_header" | "http_headers"
    )
}

fn is_environment_container(key: &str) -> bool {
    matches!(
        normalize_key(key).as_str(),
        "env" | "environment" | "environment_variables"
    )
}

fn is_url_key(key: &str) -> bool {
    let key = normalize_key(key);
    matches!(key.as_str(), "url" | "uri" | "dsn" | "endpoint")
        || key.ends_with("_url")
        || key.ends_with("_uri")
        || key.ends_with("_dsn")
}

fn is_secret_key(key: &str) -> bool {
    let key = normalize_key(key);
    if key.ends_with("_env_var") || key.ends_with("_env_variable") {
        return false;
    }
    matches!(
        key.as_str(),
        "api_key"
            | "apikey"
            | "authorization"
            | "bearer"
            | "cookie"
            | "credential"
            | "credentials"
            | "password"
            | "private_key"
            | "privatekey"
            | "secret"
            | "token"
    ) || key.ends_with("_key")
        || key.ends_with("_apikey")
        || key.ends_with("_api_key")
        || key.ends_with("_access_key")
        || key.ends_with("_accesskey")
        || key.ends_with("_private_key")
        || key.ends_with("_privatekey")
        || key.ends_with("_password")
        || key.ends_with("_secret")
        || key.ends_with("_token")
}

fn normalize_key(key: &str) -> String {
    key.trim_start_matches('-')
        .chars()
        .fold(String::new(), |mut output, character| {
            if character.is_ascii_alphanumeric() {
                output.push(character.to_ascii_lowercase());
            } else if !output.ends_with('_') {
                output.push('_');
            }
            output
        })
        .trim_matches('_')
        .to_owned()
}

#[cfg(test)]
mod tests {
    use std::fs;

    use serde_json::json;
    use tempfile::tempdir;

    use super::{hash_path, redact_url, sanitize_mcp_value};
    use crate::model::EntryScope;

    #[test]
    fn directory_hash_is_stable_and_detects_content_changes() {
        let fixture = tempdir().expect("tempdir");
        fs::create_dir(fixture.path().join("nested")).expect("nested directory");
        fs::write(fixture.path().join("nested/b.txt"), "beta").expect("write beta");
        fs::write(fixture.path().join("a.txt"), "alpha").expect("write alpha");

        let first = hash_path(fixture.path()).expect("first hash");
        let second = hash_path(fixture.path()).expect("second hash");
        assert_eq!(first, second);
        assert_eq!(first.files, 2);

        fs::write(fixture.path().join("a.txt"), "changed").expect("change file");
        let changed = hash_path(fixture.path()).expect("changed hash");
        assert_ne!(first.digest, changed.digest);
    }

    #[test]
    fn hidden_metadata_is_part_of_the_digest() {
        let fixture = tempdir().expect("tempdir");
        fs::write(fixture.path().join("SKILL.md"), "content").expect("write skill");
        let first = hash_path(fixture.path()).expect("first hash");
        fs::write(fixture.path().join(".DS_Store"), "noise").expect("write metadata");
        fs::create_dir(fixture.path().join(".git")).expect("git directory");
        fs::write(fixture.path().join(".git/config"), "noise").expect("git config");
        let second = hash_path(fixture.path()).expect("second hash");
        assert_ne!(first.digest, second.digest);
    }

    #[cfg(unix)]
    #[test]
    fn internal_symlink_target_is_hashed_without_being_followed() {
        use std::os::unix::fs::symlink;

        let fixture = tempdir().expect("tempdir");
        fs::write(fixture.path().join("first"), "same content").expect("write first");
        fs::write(fixture.path().join("second"), "same content").expect("write second");
        let link = fixture.path().join("current");
        symlink("first", &link).expect("create link");

        let first = hash_path(fixture.path()).expect("first hash");
        assert_eq!(first.symlinks, 1);
        fs::remove_file(&link).expect("remove link");
        symlink("second", &link).expect("retarget link");
        let second = hash_path(fixture.path()).expect("second hash");
        assert_ne!(first.digest, second.digest);
    }

    #[test]
    fn url_sanitizer_redacts_user_information_query_values_and_fragments() {
        assert_eq!(
            redact_url("https://arg-user:arg-password@example.test/run?token=arg-secret#private"),
            "https://<redacted>@example.test/run?token=<redacted>#<redacted>"
        );
    }

    #[test]
    fn mcp_sanitizer_redacts_secrets_and_normalizes_paths() {
        let value = json!({
            "command": "/Users/example/project/bin/server",
            "args": [
                "--token",
                "secret-value",
                "--mode=safe",
                "https://arg-user:arg-password@example.test/run?token=arg-secret"
            ],
            "url": "https://example.test/mcp?key=secret&mode=safe",
            "database_url": "postgres://db-user:db-password@example.test/database",
            "env": {
                "API_KEY": "secret-value",
                "MODE": "safe"
            },
            "headers": {
                "X-Custom": "secret-value"
            }
        });
        let sanitized = sanitize_mcp_value(
            &value,
            std::path::Path::new("/Users/example/project"),
            std::path::Path::new("/Users/example"),
            EntryScope::Workspace,
        );
        let text = serde_json::to_string(&sanitized).expect("serialize");
        assert!(!text.contains("secret-value"), "{text}");
        assert!(!text.contains("arg-password"), "{text}");
        assert!(!text.contains("arg-secret"), "{text}");
        assert!(!text.contains("db-password"), "{text}");
        assert!(text.contains("$WORKSPACE/bin/server"));
        assert!(text.contains("\"MODE\":\"<redacted>\""));
        assert!(text.contains("key=<redacted>&mode=<redacted>"));
        assert!(text.contains("postgres://<redacted>@example.test/database"));
    }
}
