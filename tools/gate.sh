#!/usr/bin/env bash
# Per-repo gate entry point. Declares nothing of its own: the step list lives
# in .gatesrc (GOH_CI_STEPS) and the checks in gates_of_heck.
#   --staged : pre-commit scope (fast) -- structural gates over the index
#   --full   : pre-push scope -- every layer, via gates/local_ci.sh
set -euo pipefail
GOH="${GOH_DIR:-${GOH:-$HOME/Projects/gates_of_heck}}"
case "${1:-}" in
  --full) exec "$GOH/gates/local_ci.sh" . ;;
  *) exec "$GOH/gates/structural.sh" "$@" ;;
esac
