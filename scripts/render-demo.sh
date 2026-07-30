#!/bin/sh
set -eu

script_directory="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
project_root="$(dirname "$script_directory")"

exec swift "${script_directory}/render-demo.swift" "$project_root"
