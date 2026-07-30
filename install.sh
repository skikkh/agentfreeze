#!/bin/sh
set -eu

repository="skikkh/agentfreeze"
release_version="latest"
binary_directory="${AGENTFREEZE_BIN_DIR:-${HOME}/.local/bin}"
allow_checksum_only=false

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
    --allow-checksum-only)
      allow_checksum_only=true
      shift
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
staging_binary=""
install_lock=""
cleanup() {
  rm -rf "$temporary_directory"
  if [ -n "$staging_binary" ]; then
    rm -f "$staging_binary"
  fi
  if [ -n "$install_lock" ]; then
    rmdir "$install_lock" 2>/dev/null || true
  fi
}
trap cleanup EXIT
trap 'exit 129' HUP
trap 'exit 130' INT
trap 'exit 143' TERM

mkdir -p "$binary_directory"
if [ -d "${binary_directory}/agentfreeze" ]; then
  echo "error: ${binary_directory}/agentfreeze is a directory" >&2
  exit 2
fi
lock_candidate="${binary_directory}/.agentfreeze-install.lock"
if ! mkdir "$lock_candidate"; then
  echo "error: another AgentFreeze installation is using ${binary_directory}" >&2
  exit 2
fi
install_lock="$lock_candidate"

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
  if [ "$release_version" = "latest" ]; then
    resolved_tag="$(gh api "repos/${repository}/releases/latest" --jq ".tag_name")"
  else
    resolved_tag="$release_version"
  fi
  case "$resolved_tag" in
    v[0-9]*)
      case "$resolved_tag" in
        *[!A-Za-z0-9._-]*)
          echo "error: unsafe release tag ${resolved_tag}" >&2
          exit 2
          ;;
      esac
      ;;
    *)
      echo "error: release tag must begin with v and a digit: ${resolved_tag}" >&2
      exit 2
      ;;
  esac

  ref_type="$(gh api "repos/${repository}/git/ref/tags/${resolved_tag}" --jq ".object.type")"
  ref_digest="$(gh api "repos/${repository}/git/ref/tags/${resolved_tag}" --jq ".object.sha")"
  if [ "$ref_type" = "tag" ]; then
    target_type="$(gh api "repos/${repository}/git/tags/${ref_digest}" --jq ".object.type")"
    source_digest="$(gh api "repos/${repository}/git/tags/${ref_digest}" --jq ".object.sha")"
    if [ "$target_type" != "commit" ]; then
      echo "error: release tag ${resolved_tag} does not resolve directly to a commit" >&2
      exit 2
    fi
  elif [ "$ref_type" = "commit" ]; then
    source_digest="$ref_digest"
  else
    echo "error: release tag ${resolved_tag} has unsupported target type ${ref_type}" >&2
    exit 2
  fi

  gh attestation verify \
    "${temporary_directory}/${archive}" \
    --repo "$repository" \
    --signer-workflow "github.com/${repository}/.github/workflows/release.yml" \
    --signer-digest "$source_digest" \
    --source-ref "refs/tags/${resolved_tag}" \
    --source-digest "$source_digest" \
    --deny-self-hosted-runners >/dev/null
  echo "Verified GitHub build provenance for ${archive} from ${resolved_tag}@${source_digest}"
else
  if [ "$allow_checksum_only" != "true" ]; then
    echo "error: GitHub CLI with attestation support is required for provenance verification" >&2
    echo "       install gh, or explicitly accept same-origin checksum trust with --allow-checksum-only" >&2
    exit 2
  fi
  echo "warning: provenance was not verified; explicit checksum-only mode is active" >&2
fi

tar -xzf "${temporary_directory}/${archive}" -C "$temporary_directory"
staging_binary="$(mktemp "${binary_directory}/.agentfreeze.XXXXXX")"
install -m 0755 "${temporary_directory}/agentfreeze" "$staging_binary"
mv -f "$staging_binary" "${binary_directory}/agentfreeze"
staging_binary=""

echo "Installed agentfreeze to ${binary_directory}/agentfreeze"
case ":${PATH}:" in
  *":${binary_directory}:"*) ;;
  *) echo "Add ${binary_directory} to PATH, then run: agentfreeze scan" ;;
esac
