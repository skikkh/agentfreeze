# Threat model

## Security objective

Given a lockfile that the operator trusts, AgentFreeze detects whether supported, discoverable
local agent-extension state has changed.

The tool answers only:

> Does the current observed state match the reviewed baseline?

The operator or host decides what to do with that evidence. AgentFreeze does not block loading and
a clean result is not a safety verdict.

## In scope

- a Skill, plugin, or MCP definition changes after it was frozen
- an extension is silently added or removed
- a plugin registry points to different content or recorded provenance
- payload material is added under a normally ignored directory such as `.git`
- a config parser encounters malformed data and would otherwise omit inventory
- a filesystem error would otherwise produce a partial “clean” result
- an internal symlink is retargeted after the baseline
- a lockfile uses an unsupported schema or malformed digest

## Out of scope

- an extension that was already malicious when first frozen
- an attacker who can modify both the extension and the trusted lockfile
- runtime-only behavior, remote server behavior, prompt injection, or model behavior
- blocking an agent at its actual load boundary
- filesystem changes between verification and later use
- operating-system permissions and executable-bit changes
- secrets intentionally redacted before MCP hashing
- the content behind an internal symlink whose target is outside the scanned extension tree
- extension locations or new host formats the detector does not yet know

## Trust assumptions

1. The executable and operating system are not compromised.
2. The reviewed lockfile comes from a trusted version-control state.
3. The configured home and workspace roots are correct.
4. AgentFreeze runs with permission to read every in-scope extension.
5. Operators treat a scanner error as a failed verification, not as “no findings.”

## Secret handling

AgentFreeze does not write raw MCP configuration to the lockfile. It hashes a sanitized,
canonical form. Likely secret fields—including secret-like environment, header, CLI-flag, and
URL-query keys—plus URL credentials are replaced before hashing. Non-secret routing values remain
covered. This reduces credential exposure and offline guessing risk, but means a tenant or account
change encoded only inside a redacted credential is not drift. Secret detection is not a proof: an
unrecognized low-entropy secret can influence the final digest and may be vulnerable to offline
guessing when the surrounding definition is known. The sanitized form itself is not serialized.

Names of extensions, MCP servers, and anchored source locations remain in the lockfile. Review a
lockfile before committing it to a public repository.

## Symlinks

Following symlinks can escape an extension root or create platform-dependent hash surfaces.
AgentFreeze rejects a symlinked extension root. For internal symlinks it hashes the normalized link
target string without following it. Retargeting the link is detected. Changes behind a stable link
are detected only when the target is also a regular file inside the scanned extension tree.

## Lockfile integrity

The SHA-256 printed after `freeze` identifies the serialized lockfile; it does not authenticate it.
Use signed commits, protected branches, release provenance, or a separate attestation system when
authentication is required.

## Safe failure behavior

Unreadable sources, malformed known config files, unsupported filesystem entries, invalid
lockfiles, and sources changing size during hashing return exit code `2`. CI and hooks must treat
both `1` and `2` as failures.
