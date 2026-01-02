#!/bin/bash
args=()
for arg in "$@"; do
    case "$arg" in
            -Wl,-z,ignore) continue ;;
            -m64[@]*) args+=("-m64") ;;
            *) args+=("$arg") ;;
        esac
done
exec zig cc "$args[@]"
