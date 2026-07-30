# Decision: pivot AgentFreeze to a no-exec unmanaged-state experiment

Status: PIVOT — public v0.1 demand test only; independent business, standardization, and acquisition
theses are NO-GO today
Decision date: 2026-07-30
Owner: skikkh
Supersedes: “最終決定 — 買収狙いOSSは agentlock” (attached memo, 2026-07-30)

## Decision

Publish the implementation as **AgentFreeze**, not AgentLock or OpenAgentLock, and constrain v0.1
to one question:

> What changed outside the agent dependency manager, without starting extension code?

The release may inventory workspace entries for visibility, but its lockfile must reject them.
AgentFreeze must not present itself as:

- a project package manager or project authorization lock
- a malware/safety scanner
- a skill activation evaluator
- a runtime firewall or sandbox
- an industry standard
- evidence that an acquisition is likely
- the first or only global agent-extension lock or monitor

## Why the original decision changed

The original memo assumed there was no agent-extension lockfile and that combining inventory,
locking, drift, and activation would create an open standard. Subsequent primary-source checks
invalidated that premise:

- [Microsoft APM](https://github.com/microsoft/apm) already has cross-agent project and global
  package state, content-hash lockfiles, drift audit, SBOM, CI, and policy.
- [Seal](https://sealfile.dev/start/what-is-seal/) already specifies project-local bundle
  authorization and integrity.
- [Snyk Agent Scan](https://github.com/snyk/agent-scan) already inventories and scans harnesses,
  MCP servers, and skills, and offers centralized background monitoring.
- [SkillBench](https://pypi.org/project/skillbench/) already runs multi-adapter skill scenarios and
  activation/behavior assertions.
- [OpenAgentLock](https://github.com/openagentlock/OpenAgentLock) already occupies both the name and
  cross-harness runtime-firewall position.

Shipping the original surface would look like a renamed Seal/APM subset. “User/global” alone is
also not a defensible distinction because APM has global locks and Snyk has background monitoring.
The remaining v0.1 experiment is local-only, no-exec evidence for supported files and definitions
that are undeclared, manually installed, cached, or modified outside a manager.

## Naming

- **AgentLock:** rejected. Existing security products and a same-field trademark application make
  collision risk unacceptable.
- **OpenAgentLock:** rejected. An active project and npm CLI already use it.
- **AgentFreeze:** selected for the experiment after an exact-name search found no same-field
  GitHub/npm/crates.io collision. This is preliminary screening, not legal trademark clearance.

## One concrete user flow

1. A developer runs `agentfreeze scan` and sees every supported user-level Skill, plugin, and MCP
   definition discoverable in local host configuration.
2. They run `agentfreeze freeze` after review.
3. `agentfreeze verify` gates a session or `agentfreeze watch` polls the endpoint profile.
4. A silent install/update produces an explicit modified, missing, or untracked event.

No server account, extension execution, MCP startup, or safety score is involved.

## Evidence required before stronger claims

### Product-demand gate

Deadline: **2026-08-13, with no extension**.

1. At least 20 unique non-maintainer users complete a scan; at least 10 use two or more supported
   agent hosts.
2. At least 3 users report a real, previously unnoticed drift event with the changed item, path,
   and action taken.
3. Complete 10 interviews; at least 5 users concretely describe monthly checking, workaround time,
   or an equivalent recurring burden.
4. Obtain one substantive response from an APM, Snyk, or agent-host maintainer: confirmation that
   the state is outside their boundary, acceptance of a fixture, or consumption of AgentFreeze
   output.
5. Only if testing a team service, obtain 3 self-serve paid pilots or preorders at $49/month. Do not
   build or price a service before that test is intentionally opened.

Failure of gate 1 or 2 ends the standalone OSS demand thesis. Failure of gate 4 ends the
standardization thesis. Failure of gate 5 means no SaaS build.

### Standardization gate

The two-week maintainer-response gate is necessary but not sufficient. All of the following are
also required before any standards claim:

- one external host or installer consumes the format
- one independent implementation passes shared positive and negative fixtures
- an existing standards community accepts or sponsors the relevant profile work

### Acquisition gate

Stars alone do not qualify. Require an external vendor to depend on the workflow, a documented
roadmap gap it fills, and direct strategic interest. No such buyer evidence is confirmed today.
OpenAI and Anthropic are not named or implied as interested buyers in public materials.

## Do now

1. Release a technically honest v0.1 with `scan`, `freeze`, `verify`, and polling `watch`.
2. Recruit the first 20 users around the exact no-exec unmanaged-state question and record missing
   paths.
3. Ask APM, Snyk, and host maintainers whether the fixtures belong upstream before expanding the
   format.

## Do not do

- Do not create a competing project-local lockfile.
- Do not implement a scanner, activation bench, or runtime firewall merely to fill a feature list.
- Do not call the format a standard or imply OpenAI/Anthropic acquisition interest.
- Do not claim global locking or background monitoring is an unoccupied category.
- Do not add package resolution, project authorization, malware scoring, or centralized monitoring
  to manufacture feature breadth.
- Do not build a detonation sandbox until there is an explicit isolation design, adversarial
  corpus, coverage model, and demand evidence.

## Revisit and stop conditions

Decide on **2026-08-13**. Do not extend the measurement window to rescue weak results. If gate 1 or
2 fails, stop positioning AgentFreeze as a standalone product and offer useful detectors or
fixtures upstream. If gate 4 fails, stop the standards bid even if individual users like the CLI.

## Implementation evidence at decision time

- Rust 1.85 tests: 16 unit tests and 19 CLI integration tests passed.
- Rust 1.85 clippy with warnings denied passed.
- A release build scanned one maintainer machine in 6.92 seconds.
- The observed profile contained 79 entries: 34 skills, 26 plugins, and 19 MCP servers.
- Freeze, verify, and one-cycle watch returned a clean 79-entry result.
- The v1 metaschema and generated lockfile validation passed; a heuristic scan found no raw home
  path or common secret pattern.

The `79` result is one observed machine, not a population estimate.

## Release safety gate update — 2026-07-31

The first fresh release review examined commit
`87eaa329fbda9eb563f09ebee8e92858883f7b2a` and kept public announcement and binary publication on
hold. Four release blockers were identified:

1. the default README executed the installer before verifying the installer itself;
2. binary archives omitted third-party dependency license texts;
3. the release workflow accepted whichever commit was current `main`, rather than an explicitly
   reviewed commit;
4. every documented install URL was necessarily unavailable before a tag and release existed.

The replacement release contract is:

- download the fixed-version installer, bind its attestation to the repository, workflow, tag, and
  exact commit, and only then execute it;
- generate `THIRD_PARTY_LICENSES.html` reproducibly from `Cargo.lock`, fail CI on drift, and include
  it in every binary archive;
- require the external repository variable `RELEASE_APPROVED_SHA` to equal the exact independently
  reviewed commit;
- publish no announcement until tag dereference, immutable release assets, attestations, and a
  clean temporary install are read back successfully.

After source visibility changes, the repository must immediately enable protected `main` and `v*`
tag rules plus private vulnerability reporting. The release remains `PRODUCTION_HOLD` until the
public GitHub settings, workflow run, attestations, release assets, install smoke test, schema-v2
audit matrix, and a new exact-SHA independent review all pass.

One bounded residual risk is accepted only through 2026-08-13: v0.1 has no explicit byte,
file-count, or tree-depth limits for otherwise valid local extension trees. The impact is local
time or memory exhaustion; the README must disclose it, and the two-week review must decide
explicit limits or continued risk acceptance.
