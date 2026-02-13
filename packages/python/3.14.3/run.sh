#!/bin/bash
DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"
if [ $# -eq 0 ]; then
    "$DIR/bin/python3" "main.py"
else
    "$DIR/bin/python3" "$@"
fi
