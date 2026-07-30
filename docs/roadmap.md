# Roadmap

The roadmap is ordered by proof, not by feature count. No phase after v0.1 is committed until the
2026-08-13 demand gate passes.

## v0.1 — observable inventory and drift

- cross-agent discovery for supported macOS, Linux, and Windows config locations
- deterministic user/machine-environment baseline
- secret-aware MCP hashing
- machine-readable verification and CI exit codes
- bounded-interval polling monitor
- signed release provenance and checksums
- no extension code or MCP server execution during inventory

Two-week success gate, due 2026-08-13 with no extension:

- 20 unique non-maintainer successful scans; 10 include two or more supported hosts
- 3 real previously unnoticed changes with item, path, and action taken
- 10 interviews; 5 identify a concrete recurring checking or workaround burden
- 1 substantive response from APM, Snyk, or an agent-host maintainer
- if a team service is explicitly tested, 3 self-serve paid pilots or preorders at $49/month

## v0.2 — fixture coverage and evidence export

- add only host locations demonstrated by external fixtures
- distinguish active installs from caches where a host exposes that fact
- export redacted, deterministic drift evidence for upstream tools
- publish positive and negative detector fixtures
- migration fixtures for config-format changes

Success gate:

- three external fixtures reproduce the same discovery result
- one manager, scanner, or host maintainer accepts a fixture or consumes the output

## v0.3 — upstream interoperability

- contribute useful detector fixtures upstream before inventing an integration format
- reference APM and Seal state without translating or rewriting their lockfiles
- map to CycloneDX/SPDX or OWASP AgBOM only after a consuming implementation requests it
- publish a host-detector interface only after an external implementation commits to use it

Success gate:

- one independent implementation passes the shared fixtures
- one external host, manager, or scanner consumes the profile

## v0.4 — event-driven drift and notifications

- native file/config events with bounded resource use
- signed, redacted drift events
- privacy-preserving credential-identity drift detection that does not expose raw or cheaply
  guessable secrets
- GitHub, Slack, and local notification adapters
- no silent loss during restart or partial writes

## v0.5 — runtime-observation decision gate

- compare existing dynamic-audit research and implementations before writing code
- identify a user problem not already owned by SkillDetonate, SkillBench, Snyk, or runtime firewalls
- require an explicit threat model, isolation boundary, fixture corpus, and coverage model
- proceed only with external demand or an upstream collaboration path

Runtime observation is not a safety oracle. It covers exercised paths only and can be evaded. No
release may claim an extension is safe because a single run was clean.

## Stop conditions

On 2026-08-13:

- failure of either the usage gate or the three-drift-story gate ends the standalone OSS demand
  thesis
- failure to obtain a substantive maintainer response ends the standards bid
- failure of the paid-pilot gate, if opened, means no SaaS build

Keep useful detector code and fixtures available for upstream contribution. Do not extend the
deadline or add adjacent security features to disguise a failed demand test.
