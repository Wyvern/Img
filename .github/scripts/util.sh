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
    local file
    for file in "$@"; do
      [[ -f "$file" ]] || break
    done
    [[ -f "$file" ]] && break
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
  "$image" \\
  ${exec_cmd:+$exec_cmd \\
  }"\$@"
EOF
  chmod +x "$linker"
}
