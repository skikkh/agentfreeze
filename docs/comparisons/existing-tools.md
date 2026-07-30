# Existing tools and the AgentFreeze boundary

Last checked: 2026-07-30

AgentFreeze is intentionally narrow. It records a local, no-exec integrity baseline for supported
agent-extension state that may be undeclared, manually installed, cached, or modified outside a
dependency manager.

| Product | Existing primary surface | AgentFreeze boundary |
|---|---|---|
| [Microsoft APM](https://github.com/microsoft/apm) | project and global package state, dependency installation, cross-host deployment, content-hash lockfiles, audit, SBOM, and policy | do not become a package manager or claim global locking is absent |
| [Seal](https://sealfile.dev/start/what-is-seal/) | project-local bundle authorization and integrity | reject project-local entries in the v1 lockfile |
| [Snyk Agent Scan](https://github.com/snyk/agent-scan) | inventory, security scanning, and centralized background monitoring for harnesses, MCP servers, and skills; MCP inspection can start configured servers with consent | do not label an extension safe, replace security analysis, or claim monitoring is absent |
| [OpenAgentLock](https://github.com/openagentlock/OpenAgentLock) | pre-tool runtime policy and tamper-evident decisions | do not claim runtime enforcement |
| [SkillBench](https://pypi.org/project/skillbench/) | multi-adapter skill scenarios, activation, and behavior evaluation | do not claim skill activation testing |
| [SchemaPin](https://schemapin.org/) | signing and verification for tool schemas and skills | do not invent a competing signature system |

## What remains distinct in v0.1

1. Discover supported extension state across local harnesses, including state not declared to APM.
2. Normalize paths and redact secret-bearing MCP configuration before hashing.
3. Freeze only the user-scoped profile without starting extension code or MCP servers.
4. Verify that profile once or poll it for additions, removals, and content changes.

This is a residual experiment, not a uniqueness claim, durable standard, or acquisition thesis.
Those stronger claims require external dependency: a host, manager, or scanner consuming the
output; a second implementation; and observed non-maintainer use.

## Revisit condition

If APM, Snyk, or a host already answers the same no-exec unmanaged-state question for the needed
fixtures, prefer contributing detectors upstream over maintaining a redundant product.
