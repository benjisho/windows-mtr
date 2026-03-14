#!/usr/bin/env bash
set -euo pipefail

BASE_REF=${1:-}
HEAD_SPEC=${2:-docs/api/openapi.yaml}
OASDIFF_IMAGE=${OASDIFF_IMAGE:-tufin/oasdiff:v1.12.0}

if [[ -z "${BASE_REF}" ]]; then
  echo "Usage: $0 <base-ref> [head-spec-path]" >&2
  exit 2
fi

if [[ ! -f "${HEAD_SPEC}" ]]; then
  echo "Missing head OpenAPI spec at ${HEAD_SPEC}" >&2
  exit 2
fi

if ! command -v docker >/dev/null 2>&1; then
  echo "docker is required to run OpenAPI compatibility checks. Install docker or run this check in CI." >&2
  exit 2
fi

resolve_base_ref() {
  local requested_ref=$1

  if git rev-parse --verify "origin/${requested_ref}" >/dev/null 2>&1; then
    printf 'origin/%s\n' "${requested_ref}"
    return 0
  fi

  if git rev-parse --verify "${requested_ref}" >/dev/null 2>&1; then
    printf '%s\n' "${requested_ref}"
    return 0
  fi

  if git rev-parse --verify "HEAD~1" >/dev/null 2>&1; then
    echo "warning: could not resolve ${requested_ref} or origin/${requested_ref}; falling back to HEAD~1" >&2
    printf 'HEAD~1\n'
    return 0
  fi

  echo "Unable to resolve base reference '${requested_ref}' or fallback 'HEAD~1'." >&2
  exit 2
}

BASE_RESOLVED_REF=$(resolve_base_ref "${BASE_REF}")

base_spec=".openapi-base-${BASE_RESOLVED_REF//\//-}.yaml"
trap 'rm -f "${base_spec}"' EXIT

git show "${BASE_RESOLVED_REF}:docs/api/openapi.yaml" > "${base_spec}"

extract_version() {
  local file=$1
  awk '
    /^info:[[:space:]]*$/ { in_info=1; next }
    in_info && /^[^[:space:]]/ { in_info=0 }
    in_info && /^[[:space:]]*version:[[:space:]]*/ {
      value=$0
      sub(/^[[:space:]]*version:[[:space:]]*/, "", value)
      gsub(/^["\x27]|["\x27]$/, "", value)
      print value
      exit
    }
  ' "$file"
}

base_version=$(extract_version "${base_spec}")
head_version=$(extract_version "${HEAD_SPEC}")

if [[ -z "${base_version}" || -z "${head_version}" ]]; then
  echo "Failed to parse OpenAPI info.version from base or head specification." >&2
  exit 2
fi

set +e
docker run --rm -v "${PWD}:/work" "${OASDIFF_IMAGE}" \
  breaking "/work/${base_spec}" "/work/${HEAD_SPEC}"
status=$?
set -e

if [[ ${status} -eq 0 ]]; then
  echo "No breaking OpenAPI changes detected against ${BASE_RESOLVED_REF}."
  exit 0
fi

if [[ "${base_version}" == "${head_version}" ]]; then
  echo "Breaking OpenAPI changes detected, but API version was not bumped (${head_version})." >&2
  exit 1
fi

echo "Breaking OpenAPI changes detected and version changed (${base_version} -> ${head_version}); allowing." >&2
exit 0
