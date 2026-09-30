#!/usr/bin/env bash
# Run both test suites against the disposable development database.
#
#   scripts/test.sh            # backend + frontend
#   scripts/test.sh backend    # only cargo test
#   scripts/test.sh frontend   # only vitest
#
# Backend repository tests use #[sqlx::test], which creates and drops a fresh
# database per test on the server at DATABASE_URL. That server must be the
# disposable one from scripts/dev-db.sh unless you set DATABASE_URL yourself.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
what="${1:-all}"

run_backend() {
  if [ -z "${DATABASE_URL:-}" ]; then
    "$root/scripts/dev-db.sh" up >/dev/null
    DATABASE_URL="$("$root/scripts/dev-db.sh" url)"
    export DATABASE_URL
  fi
  echo "== backend tests against $DATABASE_URL"
  (cd "$root/backend" && cargo test --workspace)
}

run_frontend() {
  echo "== frontend tests"
  (cd "$root/frontend" && npm test --silent)
}

case "$what" in
  all)      run_backend; run_frontend ;;
  backend)  run_backend ;;
  frontend) run_frontend ;;
  *) echo "usage: $0 [all|backend|frontend]" >&2; exit 2 ;;
esac
