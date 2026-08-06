#!/usr/bin/env bash
# GATE — the machine definition of done for machina. Exit 0 = green.
# Agents run this unprompted before claiming green, flipping a card, or writing a
# handoff/worklog entry — and paste its printed counts as evidence (FOREMAN §2).
set -uo pipefail
cd "$(dirname "$0")/.."

fail=0

# Determinism hashes need `shasum` (a perl script — absent on stock Git-for-Windows).
# Fail loud and early: empty hashes would otherwise compare equal as a silent false PASS.
if ! command -v shasum >/dev/null 2>&1; then
  echo "== GATE: FAIL == (shasum not found — run under WSL, or install perl/coreutils)"
  exit 1
fi

echo "== gate: fmt =="
cargo fmt --all --check || fail=1

echo "== gate: clippy =="
cargo clippy --all-targets --all-features -- -D warnings || fail=1

echo "== gate: tests =="
test_out=$(cargo test --workspace --all-features 2>&1) || fail=1
echo "$test_out" | grep -E "^test result:" \
  | awk -F'[ ;]+' '{p+=$4; f+=$6; i+=$8} END {printf "total: %d passed; %d failed; %d ignored\n", p, f, i}'
if echo "$test_out" | grep -qE "^test result: FAILED"; then
  echo "$test_out" | grep -E "^(test .* FAILED|failures:)" | head -20
  fail=1
fi

echo "== gate: determinism (demo x2 identical; sweep-verify) =="
d1=$(cargo run -q -p cli -- demo | shasum | awk '{print $1}')
d2=$(cargo run -q -p cli -- demo | shasum | awk '{print $1}')
if [ "$d1" = "$d2" ]; then
  echo "demo shasum: $d1 (x2 identical)"
else
  echo "demo shasum MISMATCH: $d1 vs $d2"
  fail=1
fi
s1=$(cargo run -q -p cli -- sweep | shasum | awk '{print $1}')
echo "sweep shasum: $s1"
cargo run -q -p cli -- sweep-verify || fail=1

echo "== gate: no-execution-deps scan =="
pattern='solana-sdk|solana-client|solana-program|solana-rpc|jupiter|jito|ed25519-dalek|keypair|bip39|tiny-bip39|secp256k1'
if grep -REn --include='Cargo.toml' "$pattern" . | grep -vE '^[^:]+:[0-9]+:[[:space:]]*#'; then
  echo "no-exec-deps: FAIL (execution dependency found above)"
  fail=1
else
  echo "no-exec-deps: OK"
fi

if [ "$fail" -eq 0 ]; then echo "== GATE: PASS =="; else echo "== GATE: FAIL =="; fi
exit "$fail"
