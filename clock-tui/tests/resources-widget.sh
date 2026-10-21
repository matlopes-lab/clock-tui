#!/usr/bin/env bash
set -euo pipefail

tests_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
repo_root=$(cd "$tests_dir/../.." && pwd)
widget="$repo_root/examples/widgets/tclock-resources"
fixtures="$tests_dir/fixtures/resources"
state=$(mktemp -d)
trap 'rm -rf "$state"' EXIT

fail() { printf 'FAIL %s\n' "$1" >&2; exit 1; }
expect_line() {
  grep -qF -- "$2" <<<"$1" || fail "missing [$2] in:
$1"
}
run() {
  NO_COLOR=1 TCLOCK_RESOURCES_STATE="$state" "$widget" --fixtures "$1" --now-epoch "$2"
}

first=$(run "$fixtures" 1000)

expect_line "$first" 'Servers'
expect_line "$first" ':3000  node  acme/web  12.5% cpu  400 MB'
expect_line "$first" ':5432  postgres  var/postgresql@17  0.3% cpu  80 MB'
grep -qF 'ClickUp' <<<"$(sed -n '/Servers/,/Docker/p' <<<"$first")" && fail "desktop app listed as server"

expect_line "$first" 'Docker'
expect_line "$first" 'mentalcorp-api  Up 2 hours  3.10% cpu  512MiB'
expect_line "$first" '! old-worker  Restarting (1) 5 seconds ago'
expect_line "$first" '! mentalcorp-cache  Up 3 minutes (unhealthy)  1.00% cpu  30MiB'
expect_line "$first" '  1 stopped'
grep -qF 'legacy-db' <<<"$first" && fail "stopped container listed"
grep -qF 'com.docker.backend' <<<"$first" && fail "docker backend listed as server"

expect_line "$first" 'Top processes'
expect_line "$first" 'Google Chrome Helper  45.2% cpu  1.9 GB'

expect_line "$first" 'CPU 9%'
expect_line "$first" 'RAM 7.6 of 16.0 GB (48%)'
expect_line "$first" '! Disk 6.2 GB free of 460.4 GB (99% used)'
expect_line "$first" 'Net collecting first sample'
expect_line "$first" 'Battery 85% discharging'

# A second run 10 s later with 10 MB more down and 1 MB more up.
sed -e 's/3000000000/3010000000/g' -e 's/ 500000000/ 501000000/g' "$fixtures/netstat.txt" > "$state/netstat.txt"
later=$(mktemp -d)
cp "$fixtures"/*.txt "$later"/ && cp "$state/netstat.txt" "$later/netstat.txt"
second=$(run "$later" 1010)
rm -rf "$later"
expect_line "$second" 'Net down 1.0 MB/s · up 100 KB/s'

# Commands run under a pt_BR locale print decimal commas.
comma=$(python3 -I -B -c '
import importlib.machinery, importlib.util, sys
loader = importlib.machinery.SourceFileLoader("w", sys.argv[1])
spec = importlib.util.spec_from_loader("w", loader)
w = importlib.util.module_from_spec(spec); loader.exec_module(w)
print(w.processes("  7  0,4 2048 node\n")["7"]["cpu"], w.COMMAND_ENV["LC_ALL"], "/opt/homebrew/bin" in w.COMMAND_ENV["PATH"].split(":"))
' "$widget")
[ "$comma" = "0.4 C True" ] || fail "decimal comma / locale / PATH: [$comma]"

nodocker=$(run "$tests_dir/fixtures/resources-nodocker" 2000)
expect_line "$nodocker" 'Docker not running'

printf 'resources widget: ok\n'
