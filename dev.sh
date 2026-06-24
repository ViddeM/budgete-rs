#!/usr/bin/env bash
set -euo pipefail

# ---------------------------------------------------------------------------
# dev.sh — start the budgete-rs development server
#
# Requires:
#   sqlx-cli   — cargo install sqlx-cli --no-default-features --features postgres
#   dx         — cargo install dioxus-cli --version 0.7.9
#
# Postgres — one of:
#   a) docker / docker compose  (used automatically if postgres isn't reachable)
#   b) a locally running postgres with the budgete user/database already set up
# ---------------------------------------------------------------------------

export DATABASE_URL="postgres://budgete:budgete@127.0.0.1:5432/budgete"
export LOCAL_MODE="true"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# -- Dependency checks -------------------------------------------------------

if ! command -v sqlx &>/dev/null; then
  echo "error: sqlx not found — run:" >&2
  echo "  cargo install sqlx-cli --no-default-features --features postgres" >&2
  exit 1
fi

if ! command -v dx &>/dev/null; then
  echo "error: dx not found — run:" >&2
  echo "  cargo install dioxus-cli --version 0.7.9" >&2
  exit 1
fi

# -- Helper: check if postgres is accepting connections ----------------------
# Uses pg_isready when available, falls back to a TCP port check.

pg_check() {
  if command -v pg_isready &>/dev/null; then
    pg_isready -h 127.0.0.1 -p 5432 -U budgete -d budgete -q 2>/dev/null
  else
    # bash built-in TCP probe — no external tools required
    (echo > /dev/tcp/127.0.0.1/5432) 2>/dev/null
  fi
}

# -- Helper: print with timestamp --------------------------------------------

log() { echo "[$(date +%H:%M:%S)] $*"; }

# -- Postgres: start via Docker only if not already reachable ----------------

log "==> Checking postgres on localhost:5432..."
if pg_check; then
  log "==> Postgres already reachable — skipping docker compose"
else
  if ! command -v docker &>/dev/null; then
    echo "error: postgres is not reachable on localhost:5432 and docker is not installed" >&2
    echo "" >&2
    echo "Start postgres manually, or install it natively (Arch):" >&2
    echo "  sudo pacman -S postgresql" >&2
    echo "  sudo -u postgres initdb -D /var/lib/postgres/data" >&2
    echo "  sudo systemctl start postgresql" >&2
    echo "  sudo -u postgres psql -c \"CREATE USER budgete WITH PASSWORD 'budgete';\"" >&2
    echo "  sudo -u postgres psql -c \"CREATE DATABASE budgete OWNER budgete;\"" >&2
    exit 1
  fi

  log "==> Starting postgres via docker compose..."
  docker compose -f "$SCRIPT_DIR/docker-compose.yml" up -d

  log "==> Waiting for postgres (up to 10s)..."
  for i in $(seq 1 20); do
    if pg_check; then
      log "==> Postgres ready (attempt $i)"
      break
    fi
    log "    attempt $i/20 — not ready yet, retrying in 0.5s..."
    if [ "$i" -eq 20 ]; then
      echo "error: postgres did not become ready in time" >&2
      log "    docker container status:"
      docker compose -f "$SCRIPT_DIR/docker-compose.yml" ps >&2
      log "    last 20 lines of postgres logs:"
      docker compose -f "$SCRIPT_DIR/docker-compose.yml" logs --tail=20 db >&2
      exit 1
    fi
    sleep 0.5
  done
fi

# -- Run migrations ----------------------------------------------------------

log "==> Running migrations..."
sqlx migrate run --source "$SCRIPT_DIR/packages/api/migrations"

# -- Serve -------------------------------------------------------------------

log "==> Starting dev server (LOCAL_MODE=true, no OAuth required)..."
log "    http://localhost:8080"
echo ""

cd "$SCRIPT_DIR/packages/web"
exec dx serve
