#!/bin/bash
# exec zig cc "$@"

set -e

filtered=()
skip_next=0

for a in "$@"; do
  case "$a" in
    -Wl,--dynamic-list*)
      ;;
    -Wl,-plugin-opt=*)
      ;;
    *)
      filtered+=("$a")
      ;;
  esac
done

exec zig cc "${filtered[@]}"
