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
    -Wl,--dynamic-list)
      skip_next=1   # also drop following list file
      ;;
    -Wl,--dynamic-list=*)
      ;;
    -Wl,-plugin-opt*)
      ;;
    *)
      filtered+=("$a")
      ;;
  esac
done

exec zig cc "${filtered[@]}"
