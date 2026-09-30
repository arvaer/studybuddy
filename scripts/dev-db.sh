#!/usr/bin/env bash
# Disposable PostgreSQL for local development and tests.
#
# Runs a throwaway Postgres 16 container on a non-default port so it can
# never collide with, or be mistaken for, an existing database. Nothing here
# touches any other database.
#
#   scripts/dev-db.sh up       start (or restart) the container and wait for it
#   scripts/dev-db.sh migrate  apply backend/migrations with sqlx-cli
#   scripts/dev-db.sh url      print DATABASE_URL for eval/export
#   scripts/dev-db.sh psql     open a psql shell
#   scripts/dev-db.sh reset    drop the container and start a fresh empty one
#   scripts/dev-db.sh down     stop and remove the container
set -euo pipefail

NAME="${STUDYBUDDY_PG_NAME:-studybuddy-dev-pg}"
PORT="${STUDYBUDDY_PG_PORT:-55432}"
USER_="${STUDYBUDDY_PG_USER:-studybuddy}"
PASS="${STUDYBUDDY_PG_PASSWORD:-studybuddy}"
DB="${STUDYBUDDY_PG_DB:-studybuddy_dev}"
IMAGE="${STUDYBUDDY_PG_IMAGE:-postgres:16}"
URL="postgres://${USER_}:${PASS}@127.0.0.1:${PORT}/${DB}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"

up() {
  if docker ps -a --format '{{.Names}}' | grep -qx "$NAME"; then
    docker start "$NAME" >/dev/null
  else
    docker run -d --name "$NAME" \
      -e POSTGRES_USER="$USER_" -e POSTGRES_PASSWORD="$PASS" -e POSTGRES_DB="$DB" \
      -p "127.0.0.1:${PORT}:5432" "$IMAGE" >/dev/null
  fi
  for _ in $(seq 1 30); do
    docker exec "$NAME" pg_isready -U "$USER_" -d "$DB" -q 2>/dev/null && { echo "postgres ready: $URL"; return; }
    sleep 1
  done
  echo "postgres did not become ready" >&2; exit 1
}

case "${1:-}" in
  up) up ;;
  migrate) (cd "$ROOT/backend" && DATABASE_URL="$URL" sqlx migrate run) ;;
  url) echo "$URL" ;;
  psql) docker exec -it "$NAME" psql -U "$USER_" -d "$DB" ;;
  reset) docker rm -f "$NAME" >/dev/null 2>&1 || true; up ;;
  down) docker rm -f "$NAME" >/dev/null 2>&1 || true; echo "removed $NAME" ;;
  *) sed -n '2,13p' "$0"; exit 1 ;;
esac
