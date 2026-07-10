#!/usr/bin/env bash
set -euo pipefail

# One-time, operator-run snapshot of daily SOLUSDC klines from data.binance.vision
# into gitignored files. Never invoked by any card, test, CI job, or Rust code.
#
# Usage: ingest-binance-solusdc-1d.sh START_MONTH END_MONTH [--dry-run]
#   START_MONTH, END_MONTH: YYYY-MM (inclusive)
#   --dry-run: print the zip URLs that would be fetched; fetch nothing.

DRY_RUN=0
ARGS=()
for arg in "$@"; do
  if [[ "$arg" == "--dry-run" ]]; then
    DRY_RUN=1
  else
    ARGS+=("$arg")
  fi
done

if [[ ${#ARGS[@]} -ne 2 ]]; then
  echo "Usage: $0 START_MONTH END_MONTH [--dry-run]" >&2
  exit 1
fi

START_MONTH="${ARGS[0]}"
END_MONTH="${ARGS[1]}"

OUT_DIR="data/raw/binance/SOLUSDC-1d"
BASE_URL="https://data.binance.vision/data/spot/monthly/klines/SOLUSDC/1d"

if ! git check-ignore -q "data/raw/"; then
  echo "refusing to run: data/raw/ is not git-ignored (check .gitignore)" >&2
  exit 1
fi

# Expand START_MONTH..END_MONTH (inclusive) into a list of YYYY-MM strings.
start_year=$((10#${START_MONTH%-*}))
start_mon=$((10#${START_MONTH#*-}))
end_year=$((10#${END_MONTH%-*}))
end_mon=$((10#${END_MONTH#*-}))

months=()
y="$start_year"
m="$start_mon"
while (( y < end_year || (y == end_year && m <= end_mon) )); do
  months+=("$(printf '%04d-%02d' "$y" "$m")")
  m=$((m + 1))
  if (( m > 12 )); then
    m=1
    y=$((y + 1))
  fi
done

if [[ "$DRY_RUN" -eq 1 ]]; then
  for month in "${months[@]}"; do
    echo "${BASE_URL}/SOLUSDC-1d-${month}.zip"
  done
  exit 0
fi

mkdir -p "$OUT_DIR"

fetched=0
skipped=0

for month in "${months[@]}"; do
  zip_name="SOLUSDC-1d-${month}.zip"
  zip_url="${BASE_URL}/${zip_name}"
  checksum_url="${zip_url}.CHECKSUM"
  zip_path="${OUT_DIR}/${zip_name}"
  checksum_path="${zip_path}.CHECKSUM"

  http_code=$(curl -sS -o "$zip_path" -w '%{http_code}' "$zip_url")
  if [[ "$http_code" == "404" ]]; then
    echo "skip: ${month} (404 not found)"
    rm -f "$zip_path"
    skipped=$((skipped + 1))
    continue
  fi
  if [[ "$http_code" != "200" ]]; then
    echo "fatal: unexpected HTTP ${http_code} for ${zip_url}" >&2
    exit 1
  fi

  curl -sS -o "$checksum_path" "$checksum_url"

  (cd "$OUT_DIR" && shasum -a 256 -c "$(basename "$checksum_path")")

  unzip -o -q "$zip_path" -d "$OUT_DIR"
  rm -f "$zip_path"
  fetched=$((fetched + 1))
done

csv_count=$(find "$OUT_DIR" -maxdepth 1 -name '*.csv' | wc -l | tr -d ' ')

echo "months fetched: ${fetched}"
echo "months skipped: ${skipped}"
echo "csv file count: ${csv_count}"
echo "combined content hash:"
shasum -a 256 "${OUT_DIR}"/*.csv | shasum -a 256
