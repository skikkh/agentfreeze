# Governance

## Current state

AgentFreeze is maintainer-led before 1.0. The v1 file is a documented reference format, not an
industry standard.

## Principles

1. Vendor neutral: no Claude-, Codex-, Gemini-, or editor-specific format becomes the portable
   core.
2. Existing work first: compatible contribution is preferred over a competing format.
3. Fail closed: incomplete inventory cannot become a clean verification.
4. Reproducible: the same supported input must produce the same bytes.
5. No safety theater: integrity, provenance, static analysis, and runtime observation are separate
   claims.
6. Open fixtures: every normative behavior needs positive and negative conformance cases.

## RFC requirement

An RFC is required for:

- lockfile fields or hashing changes
- discovery-boundary changes
- a new verification outcome
- provenance or signature semantics
- compatibility claims with Seal, CycloneDX, SPDX, AgBOM, or a vendor format

An RFC must include motivation, threat model, alternatives, compatibility, privacy effects,
test vectors, rollout, and rollback.

## Compatibility

- Pre-1.0 releases may change, but every format change requires an explicit version and migration.
- A reader must never interpret an unknown version as v1.
- Existing conclusions are marked superseded; history is not rewritten.
- Compatibility is demonstrated by conformance fixtures, not by similar field names.

## Standard claim gate

The project will use “standard” without qualification only after:

1. at least one implementation exists independently of the Rust reference CLI
2. the reference and independent implementation pass the same conformance suite
3. at least one external host or installer consumes the format
4. governance includes a maintainer unaffiliated with the original repository owner
5. security and privacy review findings are public

Before that gate, use “reference format.” Do not call v1 a standards proposal until an existing
host, manager, scanner, or standards community substantively engages with the work.

## Maintainer succession

If the original maintainer cannot respond for 60 days, active maintainers with accepted
contributions may nominate a successor through a public issue and seven-day comment period.
