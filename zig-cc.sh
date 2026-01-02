#!/bin/bash
args=()
for arg in "$@"; do
    if [[ "$arg" == "-Wl,-z,ignore" ]]; then
        continue
    fi
    args+=("$arg")
done
exec zig cc "$args[@]"
