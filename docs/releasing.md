# Releasing AgentFreeze

Every binary release is bound to one independently reviewed commit.

1. Keep the repository private while changing release code. Run the complete CI matrix.
2. Complete the release-readiness matrix and a fresh independent review for the exact commit.
3. Set the repository variable `RELEASE_APPROVED_SHA` to that 40-character commit only after the
   review passes.
4. On a public repository, require the four CI checks on `main`, block force-push and deletion, and
   protect `v*` tags from update or deletion.
5. Run the Release workflow on `main`; inspect all four build attestations.
6. Create annotated tag `v0.1.0` at the approved commit. The workflow must create a draft, validate
   the exact six release assets, and publish only after every build and attestation succeeds.
7. Read back immutable-release status, tag dereference, checksums, archive contents, attestations,
   and a clean install into an empty temporary directory.
8. Clear `RELEASE_APPROVED_SHA` after the immutable release is verified. Set it again only for a new
   exact-SHA review.

Never announce a version while its documented installer, source tag, or attestation path returns
404.
