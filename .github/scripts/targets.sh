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
