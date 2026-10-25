targets_from_env() {
  { set +x; } &>/dev/null
  targets=()
  if [[ -n "$TARGETS" ]]; then
    while IFS= read -r line; do
      targets+=("$line")
    done <<< "$TARGETS"
  fi
  { set -x; } &>/dev/null
}

waitfor() {
  { set +x; } &>/dev/null
  while :; do
    local file ready=1
    for file in "$@"; do
      [[ -e "$file" ]] || { ready=0; break; }
    done
    (( ready )) && break
    sleep 1
  done
  { set -x; } &>/dev/null
}

cr()
{
  cross build -q --profile src -Zbuild-std=std,panic_abort --target "$@"
}

docker_linker() {
  local linker="$1" image="$2" exec_cmd="$3"
  cat > "$linker" <<EOF
#!/bin/sh
exec docker run --rm \\
  -v "\$PWD:\$PWD" \\
  -w "\$PWD" \\
  ${exec_cmd:+--entrypoint "$exec_cmd" \\
  }"$image" \\
  "\$@"
EOF
  chmod +x "$linker"
}


push_docker() (
  local dir="$1"
  local image="$2"
  local type="$3"

  if [[ $# -ne 3 || ! -d "$dir" || -z "$image" || -z "$type" ]]; then
    echo "Usage: push_docker <dir> <image:tag> <type>" >&2
    return 2
  fi

  local image_component="${image##*/}"
  if [[ "$image_component" != *:* || "${image_component##*:}" == "" ]]; then
    echo "Error: image must include a tag: $image" >&2
    return 2
  fi

  local repo="${image%:*}"
  local layer_type="application/vnd.wyvern.${type}.tar.zstd"
  local config_digest="sha256:44136fa355b3678a1146ad16f7e8649e94fb4fc21fe77e8310c060f61caaff8a"
  local metadata size digest

  set -o pipefail

  # Pass 1: calculate the compressed archive size and digest.
  metadata=$(
    sudo tar --zstd -C "$dir" -cf - . |
      python3 -c '
import hashlib
import sys

h = hashlib.sha256()
size = 0

while True:
    chunk = sys.stdin.buffer.read(1024 * 1024)
    if not chunk:
        break
    h.update(chunk)
    size += len(chunk)

print(size, "sha256:" + h.hexdigest())
'
  ) || {
    echo "Error: failed to calculate archive size and digest" >&2
    return 1
  }

  read -r size digest <<< "$metadata"

  if [[ ! "$size" =~ ^[0-9]+$ || ! "$digest" =~ ^sha256:[0-9a-f]{64}$ ]]; then
    echo "Error: invalid archive size or digest" >&2
    return 1
  fi

  # Upload the empty OCI config blob.
  printf '{}' |
    oras blob push \
      --size 2 \
      "${repo}@${config_digest}" - || return 1

  # Pass 2: stream the archive directly to the registry.
  # ORAS verifies the content against the supplied digest.
  sudo tar --zstd -C "$dir" -cf - . |
    oras blob push \
      --size "$size" \
      "${repo}@${digest}" - || return 1

  # Publish the tagged OCI manifest.
  printf '{
  "schemaVersion": 2,
  "mediaType": "application/vnd.oci.image.manifest.v1+json",
  "config": {
    "mediaType": "application/vnd.oci.image.config.v1+json",
    "digest": "%s",
    "size": 2
  },
  "layers": [{
    "mediaType": "%s",
    "digest": "%s",
    "size": %s
  }]
}
' "$config_digest" "$layer_type" "$digest" "$size" |
    oras manifest push \
      --media-type application/vnd.oci.image.manifest.v1+json \
      "$image" -
)
