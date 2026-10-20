#!/usr/bin/env bash
set -euo pipefail

tests_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
repo_root=$(cd "$tests_dir/../.." && pwd)
widget="$repo_root/examples/widgets/tclock-agents"
fixtures="$tests_dir/fixtures/agents"

fail() { printf 'FAIL %s\n' "$1" >&2; exit 1; }

expect_line() {
  local out=$1 expected=$2
  grep -qF -- "$expected" <<<"$out" || fail "missing [$expected] in:
$out"
}

out=$(NO_COLOR=1 TZ=UTC \
  TCLOCK_AGENTS_CLAUDE_HOME="$fixtures/claude" \
  TCLOCK_AGENTS_CODEX_HOME="$fixtures/codex" \
  "$widget" --processes "$fixtures/processes.txt" --now 2026-10-20T12:40:00Z)

expect_line "$out" 'Claude Code · acme'
expect_line "$out" 'Plano de hardening'
expect_line "$out" 'claude-opus-5-5 · high · 2h40m · since 10:00 · 3.2k tok'

expect_line "$out" 'Codex · beta'
expect_line "$out" 'Refatorar login'
expect_line "$out" 'gpt-6-luna · medium · 1h40m · since 11:00 · 4.2M tok'

expect_line "$out" 'Kimi · gamma'
expect_line "$out" '2m · since 12:38'

grep -qF 'vim' <<<"$out" && fail "non-agent process listed"
grep -qF 'Claude.app' <<<"$out" && fail "desktop app listed as a session"

[ "$(grep -c '·' <<<"$out")" -ge 5 ] || fail "unexpected layout:
$out"

next_day=$(NO_COLOR=1 TZ=UTC \
  TCLOCK_AGENTS_CLAUDE_HOME="$fixtures/claude" \
  TCLOCK_AGENTS_CODEX_HOME="$fixtures/codex" \
  "$widget" --processes "$fixtures/processes.txt" --now 2026-10-21T09:00:00Z)
expect_line "$next_day" '23h00m · since 20/10 10:00'

big=$(NO_COLOR=1 python3 -I -c '
import importlib.machinery, importlib.util, sys
loader = importlib.machinery.SourceFileLoader("w", sys.argv[1])
spec = importlib.util.spec_from_loader("w", loader)
w = importlib.util.module_from_spec(spec); loader.exec_module(w)
print(w.format_tokens(1659200000), w.format_tokens(999), w.format_duration(59))
' "$widget")
[ "$big" = "1.7B 999 0m" ] || fail "formatting: [$big]"

empty=$(NO_COLOR=1 "$widget" --processes /dev/null --now 2026-10-20T12:40:00Z)
[ "$empty" = "No agent sessions running" ] || fail "empty state: [$empty]"

printf 'agents widget: ok\n'
