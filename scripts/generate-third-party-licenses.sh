#!/usr/bin/env bash
set -euo pipefail

readonly about_version="0.9.1"
case "$(uname -s)-$(uname -m)" in
  Darwin-arm64 | Darwin-aarch64)
    about_target="aarch64-apple-darwin"
    about_sha256="6a38fe166d17a674269d4373256c0b6bd93acc2553e12de0517cb9ecc73c9c02"
    ;;
  Linux-x86_64 | Linux-amd64)
    about_target="x86_64-unknown-linux-musl"
    about_sha256="c0e7dc6f5d74b0beec5c0053d39ab24514c717d19acd91886907a22457ea9e98"
    ;;
  *)
    echo "error: unsupported cargo-about host $(uname -s)-$(uname -m)" >&2
    exit 2
    ;;
esac

readonly about_archive="cargo-about-${about_version}-${about_target}.tar.gz"
readonly about_url="https://github.com/EmbarkStudios/cargo-about/releases/download/${about_version}/${about_archive}"

about_dir="$(mktemp -d)"
trap 'rm -rf -- "${about_dir}"' EXIT

curl --fail --location --silent --show-error \
  "${about_url}" \
  --output "${about_dir}/${about_archive}"
if command -v sha256sum >/dev/null 2>&1; then
  printf '%s  %s\n' "${about_sha256}" "${about_dir}/${about_archive}" | sha256sum -c -
else
  printf '%s  %s\n' "${about_sha256}" "${about_dir}/${about_archive}" | shasum -a 256 -c -
fi

tar -xzf "${about_dir}/${about_archive}" -C "${about_dir}" --strip-components=1
cargo fetch --locked
"${about_dir}/cargo-about" generate \
  --config about.toml \
  --locked \
  --offline \
  --fail \
  --output-file THIRD_PARTY_LICENSES.html \
  scripts/third-party-licenses.hbs
