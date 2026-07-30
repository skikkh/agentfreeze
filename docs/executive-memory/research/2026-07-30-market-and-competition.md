# Market and competition evidence — 2026-07-30

Status: current evidence ledger
Scope: agent extensions, local integrity, activation, runtime controls, and acquisition hypothesis

## Executive finding

The underlying pain is real, but the original “agent extension lockfile” chair is occupied.
AgentFreeze is a **PIVOT-stage no-exec unmanaged-state experiment**, not a validated category
winner.
Public evidence does not establish that OpenAI, Anthropic, Google, or a security vendor wants to
acquire an external lockfile tool.

## Evidence ledger

| # | Observed fact | Implication | Confidence | Source |
|---:|---|---|---|---|
| 1 | Microsoft APM deploys skills, plugins, hooks, instructions, and MCP across major coding agents; project and global locking, content hashes, drift audit, SBOM, CI, and policy already ship. | Both the original project lock and the later “global lock” distinction overlap directly. | high | [APM](https://github.com/microsoft/apm), [global lock reference](https://microsoft.github.io/apm/reference/cli/lock/), [audit](https://microsoft.github.io/apm/reference/cli/audit/) |
| 2 | Seal v1 hashes project-local bundles into `seal.json` and defines verification before load. It explicitly excludes user/global/project-external extensions from v1 authorization. | Do not fork the project-local lock; this boundary alone does not prove a separate category. | high | [Seal](https://sealfile.dev/start/what-is-seal/) |
| 3 | Snyk Agent Scan discovers harnesses, MCP servers, and skills, scans for security findings, and offers centralized background monitoring. Its MCP inspection can execute configured commands after consent. | Inventory and monitoring are occupied; AgentFreeze can test only the narrower local no-exec evidence workflow. | high | [Snyk Agent Scan](https://github.com/snyk/agent-scan) |
| 4 | OpenAgentLock detects local harnesses and gates tool calls with deterministic policy and a tamper-evident ledger. | Both the name and runtime-firewall position are taken. | high | [OpenAgentLock](https://github.com/openagentlock/OpenAgentLock) |
| 5 | SkillBench supports Claude, Codex, Cursor, OpenCode, API adapters, scenarios, deterministic assertions, and LLM judges. | “発火検証” is not an empty category. | high | [SkillBench](https://pypi.org/project/skillbench/) |
| 6 | SchemaPin signs tool schemas and skill directories and supports trust bundles, revocation, and multiple language implementations. | Do not invent another signature standard without interoperability demand. | high | [SchemaPin](https://schemapin.org/) |
| 7 | A Vercel/Gen/Socket/Snyk audit covers more than 60,000 skills. | Extension security is a large active surface, but this does not prove paid demand for drift monitoring. | high | [Vercel](https://vercel.com/changelog/automated-security-audits-now-available-for-skills-sh) |
| 8 | The malicious `postmark-mcp` package built trust over releases, then added a BCC backdoor in a later version. | Post-review update risk is concrete. | high | [Postmark incident](https://postmarkapp.com/blog/information-regarding-malicious-postmark-mcp-package) |
| 9 | Users report that existing skill lockfiles cannot restore all dependencies and are unreliable for reproducible team/CI onboarding. | Reproducibility pain exists, but APM and existing installers can address it. | medium | [Vercel skills #283](https://github.com/vercel-labs/skills/issues/283), [#549](https://github.com/vercel-labs/skills/issues/549) |
| 10 | MCPTrust, mcpindex, Tirith, and related tools pin live MCP contracts or configuration and can block drift. | Remote MCP drift is also occupied; feature aggregation alone is not a moat. | high | [MCPTrust](https://github.com/mcptrust/mcptrust), [mcpindex](https://mcpindex.ai/docs), [Tirith](https://github.com/sheeki03/tirith) |
| 11 | Snyk acquired Invariant Labs and integrated agent-security research. | Agent-security M&A exists, but it does not prove demand for this product. | high | [Snyk](https://go.snyk.io/the-future-of-agentic-ai-security.html) |
| 12 | Official host documentation shows OpenAI, Anthropic, and Google can pin or control plugin/extension updates themselves; Claude plugin sources can pin exact commit SHAs and update policy applies to auto-update. | Big-tech buyers can internalize this layer; external dependency must be demonstrated. | high | [OpenAI plugins](https://developers.openai.com/plugins/build/plugins), [Claude plugin marketplaces](https://code.claude.com/docs/en/plugin-marketplaces), [Gemini extensions](https://geminicli.com/docs/extensions/reference/) |
| 13 | No public primary evidence found that OpenAI, Anthropic, or Google depends on or intends to acquire an external extension-lock tool. | Acquisition must remain an unvalidated hypothesis. | medium | Official documentation and announcement search, 2026-07-30 |
| 14 | The names AgentLock and OpenAgentLock already have same-field uses; AgentLock also has a recent same-class trademark application in a preliminary search. | Do not publish under either name. | high for product collision; medium for trademark status | [AgentLock](https://agentlock.dev/), [OpenAgentLock](https://github.com/openagentlock/OpenAgentLock), [trademark listing](https://uspto.report/TM/99921632) |
| 15 | One local release build found 79 supported entries and 23 internal symlinks; complete freeze/verify/watch was clean. | MVP is usable on one real environment, not validated across users. | high | Local execution evidence, 2026-07-30 |

## What remains unconfirmed

- willingness to pay and the budget owner for endpoint extension integrity
- category TAM, revenue, and search demand
- whether existing vendors leave any material unmanaged-state workflow unserved
- whether a host/installer will consume the format
- whether AgentFreeze can obtain a second independent implementation
- buyer interest, valuation, or acquisition timing
- legal trademark clearance for AgentFreeze

## Next evidence to collect

1. By 2026-08-13, twenty unique non-maintainer successful scans, including ten environments with
   two or more supported hosts.
2. Three real unnoticed-drift stories with the changed item, path, and action taken.
3. Ten interviews, with five concrete recurring checking or workaround burdens.
4. One substantive APM, Snyk, or host-maintainer response that confirms an outside boundary,
   accepts a fixture, or consumes the output.
5. A same-fixture comparison against existing tools before adding provenance or runtime features.
6. Direct buyer/workflow dependency evidence before revisiting acquisition language.

## Provenance

This artifact reconciles:

- the user-provided “agentlock final decision” memo
- a zero-base market-scout investigation on 2026-07-30
- primary-source product documentation linked above
- local source, test, release-build, schema, and smoke-test evidence
