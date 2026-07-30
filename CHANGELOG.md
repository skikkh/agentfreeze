# Changelog

All notable changes to AgentFreeze are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and releases use
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2026-07-30

### Added

- Cross-agent discovery for user and workspace Skills, including Cursor user and managed-skill
  locations.
- Claude installed-plugin and Codex/Gemini plugin discovery.
- MCP discovery for Claude, Codex, Cursor, Gemini, VS Code, and generic project config.
- Deterministic `agentfreeze.lock` v1 with anchored paths and secret-aware MCP hashing.
- `scan`, `freeze`, `verify`, and polling `watch` commands with JSON output and stable exit codes.
- Fail-closed handling for unreadable sources, malformed configs, and symlinked extension roots.
- Non-following hashes for internal symlink targets and complete coverage of hidden files.
- CI, release checksums, and build-provenance attestation workflow.
- Installer-side build-provenance verification when GitHub CLI is available.

[Unreleased]: https://github.com/skikkh/agentfreeze/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/skikkh/agentfreeze/releases/tag/v0.1.0
