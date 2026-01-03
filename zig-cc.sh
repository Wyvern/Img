#!/bin/bash
# exec zig cc "$@"

args=()
skip_next=0

for arg in "$@"; do
    if [ $skip_next -eq 1 ]; then
        # skip the argument after --dynamic-list
        skip_next=0
        continue
    fi

    case "$arg" in
        -Wl,-z,ignore) continue ;;
        -Wl,--dynamic-list)
            skip_next=1
            continue
            ;;
        *) args+=("$arg") ;;
    esac
done
exec zig cc "${args[@]}"
