#!/bin/bash
# exec zig cc "$@"

set -e

args=()

for a in "$@"; do
  case "$a" in
    -Wl,--dynamic-list* )
      # strip unsupported linker flag
      ;;
    *)
      args+=("$a")
      ;;
  esac
done

exec zig cc "${args[@]}"
