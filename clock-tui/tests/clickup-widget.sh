#!/usr/bin/env bash
set -euo pipefail

tests_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
repo_root=$(cd "$tests_dir/../.." && pwd)
widget="$repo_root/examples/widgets/tclock-clickup"
fixtures="$tests_dir/fixtures/clickup"

fail() { printf 'FAIL %s\n' "$1" >&2; exit 1; }
expect_line() {
  grep -qF -- "$2" <<<"$1" || fail "missing [$2] in:
$1"
}
# 2026-10-15 12:00 UTC; t2 is due later today, t1 and t4 are overdue.
now=1792065600000
run() { NO_COLOR=1 TZ=UTC "$widget" --fixtures "$fixtures/$1" --now-ms "$now" "${@:2}"; }

out=$(run running)
expect_line "$out" '8 tasks · 2 overdue · 1 today · 3 doing · 2 backlog'
expect_line "$out" '▶ Refatorar auth · 0h20m'
expect_line "$out" 'Acme · Sprint 9'
expect_line "$out" '! overdue  Validar onda 1 · 13/10'
expect_line "$out" '! overdue  Corrigir login · 14/10'
expect_line "$out" '  today    Deploy staging'
expect_line "$out" '  doing    Refatorar auth · in review'
expect_line "$out" 'Acme · Backlog & Bugs'
expect_line "$out" '  2 in backlog'
expect_line "$out" 'Personal · Personal List'
expect_line "$out" '  doing    Treino · to do'
expect_line "$out" 'Hidden List'
grep -qF '? ·' <<<"$out" && fail "unknown space shown as ?"
grep -qF 'Ja entregue' <<<"$out" && fail "closed task listed"
grep -qF 'Ideia futura' <<<"$out" && fail "backlog task listed in compact view"
first_list=$(grep -m1 ' · ' <<<"$(sed -n '3,$p' <<<"$out")")
[ "$first_list" = "Acme · Sprint 9" ] || fail "lists with overdue tasks come first, got [$first_list]"

ordered=$(run running --space Personal --space Acme --exclude-list 'Hidden List')
expect_line "$ordered" '7 tasks · 2 overdue · 1 today · 2 doing · 2 backlog'
headers=$(grep -E '^[^ !▶0-9].* · |^Hidden List' <<<"$ordered")
expected_headers='Personal · Personal List
Acme · Sprint 9
Acme · Backlog & Bugs'
[ "$headers" = "$expected_headers" ] || fail "space order: got
$headers"

details=$(run running --details)
expect_line "$details" 'Ideia futura · backlog'
expect_line "$details" 'https://app.clickup.com/t/t5'
expect_line "$details" 'Corrigir login · in progress · due 14/10'

idle=$(run idle)
grep -qF '▶' <<<"$idle" && fail "timer shown while idle"

missing=$(NO_COLOR=1 TCLOCK_CLICKUP_TOKEN_FILE=/nonexistent "$widget" 2>&1 || true)
expect_line "$missing" 'ClickUp token not found'

printf 'clickup widget: ok\n'
