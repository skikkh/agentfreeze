# Contributing

Thank you for helping make agent extension inventory verifiable across vendors.

## Before opening code

1. Open or find an issue describing the host format, bug, or specification change.
2. For a new detector, link the host's primary documentation or a minimal redacted fixture.
3. Keep detection read-only. Never execute discovered commands or contact configured endpoints.
4. Do not include credentials, private paths, or copied user configuration in fixtures.

## Local checks

```sh
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
```

Every behavior change needs a focused test. Format changes also need:

- a versioning decision
- canonical serialization fixtures
- backwards-compatibility analysis
- migration behavior

## Pull requests

Keep changes narrowly scoped. Explain what changed, why, user impact, and the checks run. A pull
request that changes `agentfreeze.lock` semantics must follow the RFC process in
[docs/governance.md](docs/governance.md).

By contributing, you agree that your contribution is licensed under the MIT License.
