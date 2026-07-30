# Security policy

## Reporting a vulnerability

Please do not open a public issue for a vulnerability.

Use GitHub's private vulnerability reporting on the
[Security tab](https://github.com/skikkh/agentfreeze/security/advisories/new). Include:

- affected version and operating system
- the smallest reproducible extension/config fixture
- expected and observed behavior
- whether secrets or arbitrary files could be exposed
- a proposed fix, if you have one

You should receive an acknowledgement within 72 hours. We will coordinate disclosure after a fix
is available. Please do not include real credentials, private source code, or personal data.

## Supported versions

Until AgentFreeze reaches 1.0, only the newest tagged release receives security fixes.

## Security boundaries

AgentFreeze detects byte-level and sanitized-config drift against a trusted lockfile. Its polling
watcher does not inspect execution. AgentFreeze is not a malware classifier, sandbox, policy
engine, runtime-behavior monitor, or load-time enforcement point.

The lockfile is not a trust anchor when the same attacker can modify both the extensions and the
lockfile. Protect it with version control, review, and branch rules.

See [docs/threat-model.md](docs/threat-model.md) for the complete boundary.
