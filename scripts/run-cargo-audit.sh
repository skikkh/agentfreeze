#!/usr/bin/env bash
set -euo pipefail

readonly audit_version="0.22.2"
readonly audit_archive="cargo-audit-x86_64-unknown-linux-gnu-v${audit_version}.tgz"
readonly audit_sha256="ab28a1bdb54db4d5d8ad5981cf1f959410370b3d28250dbd35f6a44248620e39"
readonly audit_url="https://github.com/RustSec/rustsec/releases/download/cargo-audit/v${audit_version}/${audit_archive}"

audit_dir="$(mktemp -d)"
trap 'rm -rf -- "${audit_dir}"' EXIT

curl --fail --location --silent --show-error \
  "${audit_url}" \
  --output "${audit_dir}/${audit_archive}"
printf '%s  %s\n' \
  "${audit_sha256}" \
  "${audit_dir}/${audit_archive}" |
  sha256sum -c -

tar -xzf "${audit_dir}/${audit_archive}" \
  -C "${audit_dir}" \
  --strip-components=1
"${audit_dir}/cargo-audit" audit --deny warnings
