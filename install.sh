#!/bin/sh
set -eu

repository="skikkh/agentfreeze"
release_version="latest"
binary_directory="${AGENTFREEZE_BIN_DIR:-${HOME}/.local/bin}"

while [ "$#" -gt 0 ]; do
  case "$1" in
    --version)
      if [ "$#" -lt 2 ]; then
        echo "error: --version requires a tag such as v0.1.0" >&2
        exit 2
      fi
      release_version="$2"
      shift 2
      ;;
    --bin-dir)
      if [ "$#" -lt 2 ]; then
        echo "error: --bin-dir requires a directory" >&2
        exit 2
      fi
      binary_directory="$2"
      shift 2
      ;;
    *)
      echo "error: unknown option $1" >&2
      exit 2
      ;;
  esac
done

case "$(uname -s)" in
  Darwin) platform="darwin" ;;
  Linux) platform="linux" ;;
  *)
    echo "error: prebuilt installation supports macOS and Linux; use a release zip on Windows" >&2
    exit 2
    ;;
esac

case "$(uname -m)" in
  arm64 | aarch64) architecture="arm64" ;;
  x86_64 | amd64) architecture="amd64" ;;
  *)
    echo "error: unsupported architecture $(uname -m)" >&2
    exit 2
    ;;
esac

if [ "$platform" = "linux" ] && [ "$architecture" = "arm64" ]; then
  echo "error: a Linux arm64 binary is not published yet; install from source with cargo" >&2
  exit 2
fi

archive="agentfreeze-${platform}-${architecture}.tar.gz"
if [ "$release_version" = "latest" ]; then
  base_url="https://github.com/${repository}/releases/latest/download"
else
  base_url="https://github.com/${repository}/releases/download/${release_version}"
fi

temporary_directory="$(mktemp -d)"
trap 'rm -rf "$temporary_directory"' EXIT HUP INT TERM

curl --proto '=https' --tlsv1.2 -fLsS \
  "${base_url}/${archive}" \
  -o "${temporary_directory}/${archive}"
curl --proto '=https' --tlsv1.2 -fLsS \
  "${base_url}/SHA256SUMS" \
  -o "${temporary_directory}/SHA256SUMS"

checksum_line="$(grep "  ${archive}\$" "${temporary_directory}/SHA256SUMS" || true)"
if [ -z "$checksum_line" ]; then
  echo "error: ${archive} is missing from SHA256SUMS" >&2
  exit 2
fi
printf '%s\n' "$checksum_line" >"${temporary_directory}/CHECKSUM"

if command -v sha256sum >/dev/null 2>&1; then
  (
    cd "$temporary_directory"
    sha256sum -c CHECKSUM
  )
elif command -v shasum >/dev/null 2>&1; then
  (
    cd "$temporary_directory"
    shasum -a 256 -c CHECKSUM
  )
else
  echo "error: sha256sum or shasum is required to verify the release" >&2
  exit 2
fi

if command -v gh >/dev/null 2>&1 && gh attestation verify --help >/dev/null 2>&1; then
  gh attestation verify \
    "${temporary_directory}/${archive}" \
    --repo "$repository" >/dev/null
  echo "Verified GitHub build provenance for ${archive}"
else
  echo "warning: gh attestation verification is unavailable; verified checksum only" >&2
fi

tar -xzf "${temporary_directory}/${archive}" -C "$temporary_directory"
mkdir -p "$binary_directory"
install -m 0755 "${temporary_directory}/agentfreeze" "${binary_directory}/agentfreeze"

echo "Installed agentfreeze to ${binary_directory}/agentfreeze"
case ":${PATH}:" in
  *":${binary_directory}:"*) ;;
  *) echo "Add ${binary_directory} to PATH, then run: agentfreeze scan" ;;
esac
