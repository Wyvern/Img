#!/bin/bash
# exec zig cc "$@"

set -e

filtered=()
skip_next=0

for a in "$@"; do
  if [[ $skip_next -eq 1 ]]; then
    skip_next=0
    continue
  fi

  case "$a" in
    -Wl,--fix-cortex-a53-843419)
    ;;
    -Wl,--no-rosegment)
    ;;
    -Wl,-O*)
    ;;
    *)
    filtered+=("$a")
    ;;
  esac
done

exec zig cc "${filtered[@]}"
