# AgentFreeze and Seal

Last checked: 2026-07-30

## Decision

AgentFreeze will not publish a second project-local bundle-authorization format.

[Seal](https://sealfile.dev/start/what-is-seal/) already provides a normative v1 specification,
the `seal.json` format, a Go reference CLI, deterministic serialization, CI instructions, and
prebuilt releases. Its specification deliberately keeps project-external user, organization,
global, admin, and cache-installed extensions outside Seal v1 authorization.

AgentFreeze does not claim that this boundary is an empty category: Microsoft APM already supports
global package locks, and Snyk Agent Scan already supports background monitoring. Its narrower v0.1
experiment is unmanaged-state evidence collected without starting extension code:

```text
Seal
  project-local bundle allowlist and future native load blocking

AgentFreeze
  no-exec inventory and drift baseline for supported, possibly unmanaged local state
```

## Behavioral comparison

| Question | AgentFreeze v0.1 | Seal v1 |
|---|---|---|
| Which supported Claude/Codex/Cursor/Gemini extensions exist in local user state? | primary behavior | discovery may exist, but project-external units are outside v1 authorization |
| Can a repository commit a project-local allowlist? | no; workspace inventory only | yes, normative core |
| Does a host have specified load-time blocking behavior? | no | yes |
| Are user/global extensions represented? | yes | discovered but not authorized by `seal.json` |
| Does it execute or score an extension? | no | no |
| Does it detect drift? | supported user state, including unmanaged installs | project-local sealed bundles |

## Interoperability requirements

Before AgentFreeze claims Seal integration, it must:

1. consume upstream conformance fixtures
2. match Seal's per-file and aggregate hashing exactly
3. preserve Seal's fail-closed discovery and nested-bundle rules
4. avoid translating a user/global profile into a project authorization silently
5. pass review by an independent implementation or upstream maintainer

Until those conditions are met, documentation must say “complementary,” not “compatible.”

## Why not fork now

Two incompatible lockfiles for the same project-local surface would increase operator confusion
and split host-integration effort. AgentFreeze must earn a separate role through demonstrated
no-exec evidence for manager-external state—not through a renamed copy of Seal. If users and
upstream maintainers do not validate that residual need, its detectors should be contributed
upstream and the standalone project stopped.

## Primary references

- [Seal: What is Seal?](https://sealfile.dev/start/what-is-seal/)
- [Seal full specification](https://github.com/SamyGhannad/seal-spec/blob/main/SPEC.md)
- [Seal CLI installation and release coverage](https://sealfile.dev/start/install/)
- [Seal agent integration status](https://sealfile.dev/integrate/agents/)
