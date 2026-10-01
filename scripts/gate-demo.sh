#!/usr/bin/env bash
# Phase 1 gate walkthrough (docs/hardening-plan.md, "Gate for beginning
# application integration"; issue #16). Drives the real HTTP API end to end,
# including a backend restart, and prints one PASS/FAIL line per gate step.
#
#   scripts/gate-demo.sh              # start a backend on $GATE_PORT, run, stop it
#   GATE_BASE=http://127.0.0.1:3000 scripts/gate-demo.sh   # against a running backend (step 4 restart skipped)
#
# Uses the disposable database from scripts/dev-db.sh unless DATABASE_URL is set,
# a scratch UPLOADS_DIR, and two throwaway learners with random emails. Nothing
# is deleted; the rows it creates stay in the development database.
set -uo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
GATE_PORT="${GATE_PORT:-3100}"
work="$(mktemp -d)"
fail=0
pass() { printf 'PASS  step %s  %s\n' "$1" "$2"; }
failed() { printf 'FAIL  step %s  %s\n' "$1" "$2"; fail=1; }
check() { # step description condition...
  local step="$1" desc="$2"; shift 2
  if "$@"; then pass "$step" "$desc"; else failed "$step" "$desc"; fi
}

backend_pid=""
start_backend() {
  (cd "$root/backend" && exec env PORT="$GATE_PORT" UPLOADS_DIR="$work/uploads" COOKIE_SECURE=false \
      RUST_LOG=lugia=warn ./target/debug/lugia >>"$work/backend.log" 2>&1) &
  backend_pid=$!
  for _ in $(seq 1 120); do
    curl -fs "$BASE/health" >/dev/null 2>&1 && return
    sleep 1
  done
  echo "backend did not start; see $work/backend.log" >&2; exit 1
}
stop_backend() {
  [ -n "$backend_pid" ] || return 0
  kill "$backend_pid" 2>/dev/null; wait "$backend_pid" 2>/dev/null
  backend_pid=""
  for _ in $(seq 1 20); do curl -fs "$BASE/health" >/dev/null 2>&1 || return; sleep 0.5; done
}
trap 'stop_backend' EXIT

if [ -n "${GATE_BASE:-}" ]; then
  BASE="$GATE_BASE"; managed=0
else
  BASE="http://127.0.0.1:$GATE_PORT"; managed=1
  if [ -z "${DATABASE_URL:-}" ]; then
    "$root/scripts/dev-db.sh" up >/dev/null; export DATABASE_URL; DATABASE_URL="$("$root/scripts/dev-db.sh" url)"
  fi
  export JWT_SECRET="${JWT_SECRET:-$(openssl rand -base64 48)}"
  echo "building and starting backend on $BASE (uploads in $work/uploads)"
  (cd "$root/backend" && cargo build -q) || exit 1
  start_backend
fi

# ---- helpers: every call returns body on stdout; last_code is its HTTP status --
last_code() { cat "$work/code"; }
api() { # method path token [json-body]
  local m="$1" p="$2" t="$3" b="${4:-}"
  local out
  if [ -n "$b" ]; then
    out="$(curl -s -o "$work/body" -w '%{http_code}' -X "$m" "$BASE$p" -H "Authorization: Bearer $t" -H 'Content-Type: application/json' --data "$b")"
  else
    out="$(curl -s -o "$work/body" -w '%{http_code}' -X "$m" "$BASE$p" -H "Authorization: Bearer $t")"
  fi
  printf %s "$out" >"$work/code"; cat "$work/body"
}
signup() { # email -> access token
  local email="$1"
  api POST /api/auth/signup "" "{\"email\":\"$email\",\"password\":\"gate-demo-pass\",\"displayName\":\"$2\"}" | jq -r .accessToken
}
login() { api POST /api/auth/login "" "{\"email\":\"$1\",\"password\":\"gate-demo-pass\"}" | jq -r .accessToken; }

run="$(date +%s)-$RANDOM"
A_EMAIL="gate-a-$run@example.test"; B_EMAIL="gate-b-$run@example.test"
A="$(signup "$A_EMAIL" "Learner A")"; B="$(signup "$B_EMAIL" "Learner B")"
[ "$A" != null ] && [ "$B" != null ] || { echo "signup failed"; cat "$work/body"; exit 1; }

# ---- author one source-linked activity as learner A ------------------------
topic="$(api POST /api/topics "$A" '{"name":"Gate demo"}' | jq -r .id)"
printf 'Photosynthesis converts light energy into chemical energy.\n' >"$work/notes.txt"
res1="$(curl -s "$BASE/api/resources/upload" -H "Authorization: Bearer $A" -F "file=@$work/notes.txt" -F "topicId=$topic" -F "title=notes v1")"
res1_id="$(jq -r .id <<<"$res1")"; art1="$(jq -r .artifactId <<<"$res1")"
activity="$(api POST /api/activities "$A" "$(jq -nc --arg r "$res1_id" '{kind:"recall",revision:{prompt:"Photosynthesis converts light energy into what?",options:["chemical energy","heat","sound"],answerKey:"chemical energy",sourceResourceId:$r,sourceLocation:{line:1}}}')")"
act_id="$(jq -r .id <<<"$activity")"; rev_id="$(jq -r .current.id <<<"$activity")"

# ---- 1. sign in, select the activity, load its exact revision + source ------
A="$(login "$A_EMAIL")"
listed="$(api GET /api/activities "$A")"
rev="$(api GET "/api/revisions/$rev_id" "$A")"
check 1 "signed in; activity listed with its current revision" \
  test "$(jq -r --arg id "$act_id" '.[]|select(.id==$id)|.current.id' <<<"$listed")" = "$rev_id"
check 1 "revision cites source resource and the exact artifact; answer key withheld" \
  test "$(jq -r '[.sourceResourceId,.sourceArtifactId,.hasAnswerKey,(has("answerKey"))]|join(",")' <<<"$rev")" = "$res1_id,$art1,true,false"

# ---- 2 + 3. submit; ownership checked; original attempt + assistance recorded;
#             deterministic assessment ---------------------------------------
key="$(uuidgen | tr A-F a-f)"
payload="$(jq -nc --arg k "$key" --arg r "$rev_id" '{requestKey:$k,activityRevisionId:$r,response:"chemical energy",assistance:[{kind:"hint",text:"think of glucose"}]}')"
receipt="$(api POST /api/attempts "$A" "$payload")"; first_code="$(last_code)"
attempt_id="$(jq -r .attemptId <<<"$receipt")"
check 2 "attempt recorded (201) against the exact revision with the response as submitted" \
  test "$first_code,$(jq -r '[.activityRevisionId,.response]|join(",")' <<<"$receipt")" = "201,$rev_id,chemical energy"
check 3 "deterministic choice assessment: status correct, method choice" \
  test "$(jq -r '[.status,.assessment.method]|join(",")' <<<"$receipt")" = "correct,choice"

# an unkeyed activity stays explicitly pending
pending_act="$(api POST /api/activities "$A" '{"kind":"explain","revision":{"prompt":"Explain why leaves are green."}}')"
pending_rev="$(jq -r .current.id <<<"$pending_act")"
pending="$(api POST /api/attempts "$A" "$(jq -nc --arg k "$(uuidgen)" --arg r "$pending_rev" '{requestKey:$k,activityRevisionId:$r,response:"chlorophyll reflects green"}')")"
check 3 "unkeyed activity: status pending, assessment null (pending is not incorrect)" \
  test "$(jq -r '[.status,(.assessment|tostring)]|join(",")' <<<"$pending")" = "pending,null"

# ---- 4. refresh + restart: activity and attempt remain -----------------------
if [ "$managed" = 1 ]; then
  stop_backend; start_backend
  check 4 "backend restarted (fresh process, same database)" test -n "$backend_pid"
else
  echo "skip  step 4  restart not performed against GATE_BASE"
fi
A="$(login "$A_EMAIL")"   # a page refresh re-reads everything from the backend
again="$(api GET "/api/attempts/$attempt_id" "$A")"
check 4 "after refresh/restart: same attempt id, revision, response, status" \
  test "$(jq -r '[.attemptId,.activityRevisionId,.response,.status]|join(",")' <<<"$again")" = "$attempt_id,$rev_id,chemical energy,correct"
check 4 "after restart: activity still lists the same current revision" \
  test "$(api GET "/api/activities/$act_id" "$A" | jq -r .current.id)" = "$rev_id"

# ---- 5. retry same key+payload -> existing result; conflicting reuse refused --
count_before="$(api GET /api/activities "$A" | jq length)"
replay="$(api POST /api/attempts "$A" "$payload")"; replay_code="$(last_code)"
check 5 "same key, same payload: 200 and the existing attempt id" \
  test "$replay_code,$(jq -r .attemptId <<<"$replay")" = "200,$attempt_id"
conflict="$(api POST /api/attempts "$A" "$(jq -c '.response="heat"' <<<"$payload")")"; conflict_code="$(last_code)"
original="$(api GET "/api/attempts/$attempt_id" "$A")"
check 5 "same key, different payload: 409 and the original is unchanged" \
  test "$conflict_code,$(jq -r '[.response,.status]|join(",")' <<<"$original")" = "409,chemical energy,correct"

# ---- 6. second learner is refused and mutates nothing -----------------------
b_rev="$(api GET "/api/revisions/$rev_id" "$B")"; c1="$(last_code)"
b_act="$(api GET "/api/activities/$act_id" "$B")"; c2="$(last_code)"
b_att="$(api GET "/api/attempts/$attempt_id" "$B")"; c3="$(last_code)"
b_art="$(api GET "/api/artifacts/$art1/bytes" "$B")"; c4="$(last_code)"
b_sub="$(api POST /api/attempts "$B" "$(jq -nc --arg k "$key" --arg r "$rev_id" '{requestKey:$k,activityRevisionId:$r,response:"heat"}')")"; c5="$(last_code)"
b_revise="$(api POST "/api/activities/$act_id/revisions" "$B" '{"prompt":"hijacked"}')"; c6="$(last_code)"
check 6 "learner B: revision, activity, attempt, artifact bytes all 404" test "$c1,$c2,$c3,$c4" = "404,404,404,404"
check 6 "learner B: submitting against A's revision (even reusing A's key) is 404; revising is 404" test "$c5,$c6" = "404,404"
check 6 "learner B's list of activities does not contain A's" \
  test "$(api GET /api/activities "$B" | jq -r --arg id "$act_id" '[.[]|select(.id==$id)]|length')" = 0
after="$(api GET "/api/activities/$act_id" "$A")"
check 6 "A's activity still at revision 1 and the attempt unchanged" \
  test "$(jq -r .current.revision <<<"$after"),$(api GET "/api/attempts/$attempt_id" "$A" | jq -r .status)" = "1,correct"

# ---- 7. same-name upload later does not replace the cited source version -----
printf 'EDITED: photosynthesis is about heat.\n' >"$work/notes.txt"
res2="$(curl -s "$BASE/api/resources/upload" -H "Authorization: Bearer $A" -F "file=@$work/notes.txt" -F "topicId=$topic" -F "title=notes v2")"
art2="$(jq -r .artifactId <<<"$res2")"
cited="$(api GET "/api/revisions/$rev_id" "$A" | jq -r .sourceArtifactId)"
bytes1="$(api GET "/api/artifacts/$art1/bytes" "$A")"
check 7 "second notes.txt is a new artifact; revision still cites the first" \
  test "$art1" != "$art2" -a "$cited" = "$art1"
check 7 "the cited version's bytes are the original text" \
  test "$bytes1" = "Photosynthesis converts light energy into chemical energy."

echo
if [ "$fail" = 0 ]; then echo "GATE: all steps passed  (learners $A_EMAIL, $B_EMAIL; scratch $work)"; else echo "GATE: FAILED  (see above; backend log $work/backend.log)"; exit 1; fi
