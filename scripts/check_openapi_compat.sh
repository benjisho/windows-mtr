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
base_spec_for_diff=".openapi-base-${BASE_RESOLVED_REF//\//-}.oasdiff.yaml"
head_spec_for_diff=".openapi-head.oasdiff.yaml"
oasdiff_output=".openapi-oasdiff-output.log"
trap 'rm -f "${base_spec}" "${base_spec_for_diff}" "${head_spec_for_diff}" "${oasdiff_output}"' EXIT

git show "${BASE_RESOLVED_REF}:docs/api/openapi.yaml" > "${base_spec}"

normalize_for_oasdiff() {
  local input_file=$1
  local output_file=$2

  ruby -ryaml -e '
    input_path = ARGV.fetch(0)
    output_path = ARGV.fetch(1)

    normalize = lambda do |node|
      case node
      when Hash
        transformed = node.each_with_object({}) { |(k, v), memo| memo[k] = normalize.call(v) }

        ex_min = transformed["exclusiveMinimum"]
        if ex_min.is_a?(Numeric)
          transformed["minimum"] = ex_min unless transformed.key?("minimum")
          transformed["exclusiveMinimum"] = true
        end

        ex_max = transformed["exclusiveMaximum"]
        if ex_max.is_a?(Numeric)
          transformed["maximum"] = ex_max unless transformed.key?("maximum")
          transformed["exclusiveMaximum"] = true
        end

        transformed
      when Array
        node.map { |item| normalize.call(item) }
      else
        node
      end
    end

    data = YAML.load_file(input_path)
    normalized = normalize.call(data)

    if normalized.is_a?(Hash) && normalized["openapi"].to_s.start_with?("3.1")
      normalized["openapi"] = "3.0.3"
    end

    File.write(output_path, YAML.dump(normalized))
  ' "${input_file}" "${output_file}"
}

normalize_for_oasdiff "${base_spec}" "${base_spec_for_diff}"
normalize_for_oasdiff "${HEAD_SPEC}" "${head_spec_for_diff}"

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
  breaking "/work/${base_spec_for_diff}" "/work/${head_spec_for_diff}" >"${oasdiff_output}" 2>&1
status=$?
set -e

if [[ -f "${oasdiff_output}" ]]; then
  cat "${oasdiff_output}"
fi

if [[ ${status} -ne 0 ]] && grep -Eq \
  'failed to load .* spec|failed to unmarshal data|Error response from daemon|permission denied while trying to connect to the Docker daemon socket' \
  "${oasdiff_output}"; then
  echo "OpenAPI compatibility check failed due a tooling/runtime issue; not interpreting this as a contract-breaking diff." >&2
  exit 2
fi

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
