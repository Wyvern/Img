#!/bin/bash
# exec zig cc "$@"

set -e

filtered=()

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
