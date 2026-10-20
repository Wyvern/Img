targets_from_env() {
  { set +x; } 2>/dev/null
  targets=()
  if [[ -n "$TARGETS" ]]; then
    while IFS= read -r line; do
      targets+=("$line")
    done <<< "$TARGETS"
  fi
  { set -x; } 2>/dev/null
}

waitfor() {
  { set +x; } 2>/dev/null
  while :; do
    local file
    for file in "$@"; do
      [[ -f "$file" ]] || break
    done
    [[ -f "$file" ]] && break
    sleep 1
  done
  { set -x; } 2>/dev/null
}
