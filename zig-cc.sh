#!/bin/bash
# exec zig cc "$@"

args=()
for arg in "$@"; do
    case "$arg" in
        -Wl,-z,ignore) continue ;;
        -Wl,--dynamic-list) continue ;;
        *) args+=("$arg") ;;
    esac
done
exec zig cc "${args[@]}"
