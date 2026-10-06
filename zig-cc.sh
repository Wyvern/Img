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
    -Wl,-z,pack-relative-relocs)
      ;;
    *)
      filtered+=("$a")
      ;;
  esac
done

printf 'zig cc args:\n' >&2
printf '  %q\n' "${filtered[@]}" >&2
exec zig cc -### "${filtered[@]}"
# exec zig cc "${filtered[@]}"
