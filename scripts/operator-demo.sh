#!/usr/bin/env bash
# Phase 2 gate (docs/phase-2-build.md, 20d-ii; issue #20): the operator over
# a long horizon, against the real backend, the real database, and the real
# model. Prints one PASS/FAIL line per check.
#
#   scripts/operator-demo.sh        # build, start a backend on $GATE_PORT, run, stop it
#
# Needs ANTHROPIC_API_KEY in backend/.env (the backend reads it; this script
# never does) and the disposable database from scripts/dev-db.sh unless
# DATABASE_URL is set. The learner is demo@demo.test with password demodemo
# (DEMO_EMAIL, DEMO_PASSWORD), signed up on first use, so you can open the
# frontend afterwards and see what the run built. A workspace takes one goal,
# so when that account already has one the run uses demo+<n>@demo.test with
# the same password and says so. Nothing is deleted; the rows it creates stay
# in the development database. Every model call is billed to the key, about
# five per run.
#
# Steps:
#   1. intent in; the first activity lands
#   2. an answer wakes the operator; the follow-up lands
#   3. an answer, the backend killed mid-think (SIGKILL), restarted: the
#      same follow-up lands once, nothing duplicated
#   4. leave mid-activity; days pass (the lease long lapsed, a cold process);
#      return: the same activity waits, the plan continues, no model call
set -uo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
GATE_PORT="${GATE_PORT:-3200}"
BASE="http://127.0.0.1:$GATE_PORT"
work="$(mktemp -d)"
fail=0
pass() { printf 'PASS  step %s  %s\n' "$1" "$2"; }
failed() { printf 'FAIL  step %s  %s\n' "$1" "$2"; fail=1; }
check() { local step="$1" desc="$2"; shift 2; if "$@"; then pass "$step" "$desc"; else failed "$step" "$desc"; fi; }
note() { printf '      %s\n' "$*"; }

if [ -z "${DATABASE_URL:-}" ]; then
  "$root/scripts/dev-db.sh" up >/dev/null; export DATABASE_URL; DATABASE_URL="$("$root/scripts/dev-db.sh" url)"
fi
export JWT_SECRET="${JWT_SECRET:-$(openssl rand -base64 48)}"
grep -q '^ANTHROPIC_API_KEY=.\+' "$root/backend/.env" 2>/dev/null || { echo "backend/.env has no ANTHROPIC_API_KEY; the operator cannot think" >&2; exit 1; }

backend_pid=""
start_backend() {
  (cd "$root/backend" && exec env PORT="$GATE_PORT" UPLOADS_DIR="$work/uploads" COOKIE_SECURE=false \
      RUST_LOG="${RUST_LOG:-lugia=info,operator=info}" ./target/debug/lugia >>"$work/backend.log" 2>&1) &
  backend_pid=$!
  for _ in $(seq 1 120); do curl -fs "$BASE/health" >/dev/null 2>&1 && return; sleep 1; done
  echo "backend did not start; see $work/backend.log" >&2; exit 1
}
stop_backend() { # [signal]
  [ -n "$backend_pid" ] || return 0
  kill "-${1:-TERM}" "$backend_pid" 2>/dev/null; wait "$backend_pid" 2>/dev/null; backend_pid=""
  for _ in $(seq 1 40); do curl -fs "$BASE/health" >/dev/null 2>&1 || return; sleep 0.25; done
}
trap 'stop_backend' EXIT

echo "building and starting backend on $BASE (log $work/backend.log)"
(cd "$root/backend" && cargo build -q) || exit 1
start_backend

last_code() { cat "$work/code"; }
api() { # method path token [json-body]
  local m="$1" p="$2" t="$3" b="${4:-}" out
  if [ -n "$b" ]; then
    out="$(curl -s -o "$work/body" -w '%{http_code}' -X "$m" "$BASE$p" -H "Authorization: Bearer $t" -H 'Content-Type: application/json' --data "$b")"
  else
    out="$(curl -s -o "$work/body" -w '%{http_code}' -X "$m" "$BASE$p" -H "Authorization: Bearer $t")"
  fi
  printf %s "$out" >"$work/code"; cat "$work/body"
}
DEMO_EMAIL="${DEMO_EMAIL:-demo@demo.test}"; DEMO_PASSWORD="${DEMO_PASSWORD:-demodemo}"
login() { api POST /api/auth/login "" "$(jq -nc --arg e "$1" --arg p "$DEMO_PASSWORD" '{email:$e,password:$p}')" | jq -r .accessToken; }
signup() { api POST /api/auth/signup "" "$(jq -nc --arg e "$1" --arg p "$DEMO_PASSWORD" '{email:$e,password:$p,displayName:"Demo learner"}')" | jq -r .accessToken; }
# The demo account, or an existing one's token; null when the login fails.
sign_in() { local t; t="$(signup "$1")"; [ "$t" != null ] && [ -n "$t" ] && { printf %s "$t"; return; }; login "$1"; }
workspace() { api GET "/api/workspaces/$WS" "$A"; }
# Poll the workspace until the operator is neither thinking nor unavailable
# (a just-restarted process finds the dead one's lease held for up to ten
# seconds and says so, 20e).
# Prints the body; `waited` answers the seconds it took.
settled() {
  local started; started="$(date +%s)"
  local op
  for _ in $(seq 1 600); do
    local body; body="$(workspace)"
    op="$(jq -r .operator <<<"$body" 2>/dev/null)"
    if [ "$(last_code)" = 200 ] && [ "$op" != thinking ] && [ "$op" != unavailable ]; then
      echo $(( $(date +%s) - started )) >"$work/waited"; printf %s "$body"; return
    fi
    sleep 1
  done
  echo "the operator did not settle in ten minutes; see $work/backend.log" >&2; exit 1
}
waited() { cat "$work/waited"; }
activity_count() { api GET /api/activities "$A" | jq length; }
answer() { # revision text
  api POST /api/attempts "$A" "$(jq -nc --arg k "$(uuidgen | tr A-F a-f)" --arg r "$1" --arg t "$2" '{requestKey:$k,activityRevisionId:$r,response:$t}')"
}
revision_of() { api GET "/api/activities/$1" "$A" | jq -r .current.id; }
prompt_of() { api GET "/api/activities/$1" "$A" | jq -r .current.prompt; }
settled_log_lines() { grep -c "operator settled" "$work/backend.log" || true; }

A_EMAIL="$DEMO_EMAIL"
A="$(sign_in "$A_EMAIL")"
[ "$A" != null ] && [ -n "$A" ] || { echo "could not sign in as $A_EMAIL (is DEMO_PASSWORD the one it was made with?)"; cat "$work/body"; exit 1; }
WS="$(api GET /api/workspaces/current "$A" | jq -r .id)"
if [ "$(api GET "/api/workspaces/$WS" "$A" | jq -r '.goal != null')" = true ]; then
  n=1; while :; do
    A_EMAIL="${DEMO_EMAIL%%@*}+$n@${DEMO_EMAIL#*@}"
    A="$(sign_in "$A_EMAIL")"; [ "$A" != null ] && [ -n "$A" ] || { echo "could not sign in as $A_EMAIL"; exit 1; }
    WS="$(api GET /api/workspaces/current "$A" | jq -r .id)"
    [ "$(api GET "/api/workspaces/$WS" "$A" | jq -r '.goal != null')" = true ] || break
    n=$((n + 1))
  done
  note "$DEMO_EMAIL already has its goal; this run is $A_EMAIL (same password)"
fi

# ---- 1. intent in; the first activity lands ---------------------------------
intent="Understand the return and discounting in reinforcement learning."
accepted="$(api POST "/api/workspaces/$WS/goal" "$A" "$(jq -nc --arg i "$intent" '{intent:$i}')")"
check 1 "intent accepted (202), operator thinking" \
  test "$(last_code),$(jq -r .operator <<<"$accepted")" = "202,thinking"
first="$(settled)"
act1="$(jq -r .currentActivityId <<<"$first")"
check 1 "first activity landed; operator waiting on it ($(waited)s)" \
  test "$(jq -r .operator <<<"$first")" = waiting -a "$act1" != null
note "prompt 1: $(prompt_of "$act1")"

# ---- 2. an answer wakes the operator; the follow-up lands -------------------
rev1="$(revision_of "$act1")"
receipt="$(answer "$rev1" "The return is the sum of future rewards, each discounted by gamma to the power of its delay.")"
check 2 "attempt recorded (201)" test "$(last_code)" = 201
second="$(settled)"
act2="$(jq -r .currentActivityId <<<"$second")"
check 2 "follow-up landed: a different activity, operator waiting ($(waited)s)" \
  test "$(jq -r .operator <<<"$second")" = waiting -a "$act2" != "$act1" -a "$act2" != null
note "prompt 2: $(prompt_of "$act2")"

# ---- 3. answer, kill mid-think, restart; the same follow-up lands once ------
rev2="$(revision_of "$act2")"
before=$(activity_count)
answer "$rev2" "Discounting keeps the infinite sum bounded and values sooner reward more." >/dev/null
check 3 "attempt recorded (201); the operator is now thinking" test "$(last_code)" = 201
sleep 1   # into the model call
stop_backend KILL
check 3 "backend killed mid-think (SIGKILL)" test -z "$backend_pid"
start_backend
A="$(login "$A_EMAIL")"
third="$(settled)"
act3="$(jq -r .currentActivityId <<<"$third")"
check 3 "after restart the think resumed and the next activity landed ($(waited)s, lease included)" \
  test "$(jq -r .operator <<<"$third")" = waiting -a "$act3" != "$act2" -a "$act3" != null
check 3 "exactly one new activity: no duplicate from the interrupted think" \
  test "$(activity_count)" = "$((before + 1))"
check 3 "the log says the interrupted park was allowed again" \
  grep -q "interrupted think allowed again" "$work/backend.log"
note "prompt 3: $(prompt_of "$act3")"

# ---- 4. leave mid-activity; days pass; return: the same plan continues -------
stop_backend
# Days pass: the lease this process held has long lapsed, and nothing on
# the record depends on the clock. The one time-bearing row is moved back.
psql -q "$DATABASE_URL" -c "UPDATE workspace_sessions SET lease_until = now() - interval '3 days' WHERE workspace_id = '$WS'" >/dev/null
settled_before=$(settled_log_lines); count_before=$(activity_count 2>/dev/null || echo "$((before + 1))")
start_backend
A="$(login "$A_EMAIL")"
returned="$(settled)"
check 4 "days later, the same activity is waiting; the goal is the same" \
  test "$(jq -r .operator <<<"$returned")" = waiting -a "$(jq -r .currentActivityId <<<"$returned")" = "$act3" -a "$(jq -r .goal.intent <<<"$returned")" = "$intent"
check 4 "reopen asked no model and published nothing: no settle, same activity count" \
  test "$(settled_log_lines)" = "$settled_before" -a "$(activity_count)" = "$((before + 1))"
rev3="$(revision_of "$act3")"
answer "$rev3" "A larger gamma weighs distant rewards more; gamma near zero is myopic." >/dev/null
check 4 "the plan continues: the answer is accepted (201) and the operator thinks again" \
  test "$(last_code)" = 201
fourth="$(settled)"
check 4 "the operator went on from where it left off ($(waited)s): $(jq -r '.operator + (if .last then ": " + .last else "" end)' <<<"$fourth")" \
  test "$(jq -r .operator <<<"$fourth")" != stalled

echo
if [ "$fail" = 0 ]; then echo "GATE 2: all steps passed  (sign in as $A_EMAIL / $DEMO_PASSWORD; workspace $WS; log $work/backend.log)"; else echo "GATE 2: FAILED  (see above; backend log $work/backend.log)"; exit 1; fi
