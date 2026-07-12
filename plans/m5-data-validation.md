# M5 data validation record (card M5-C3 sitting, 2026-07-11)

The hygiene record the Q5 freeze rests on. Ingested and validated by the operator by hand
(scripts/ingest-binance-solusdc-1d.sh + `machina data-validate`); frozen numbers in
[config/strategies/m5-frozen.toml](../config/strategies/m5-frozen.toml); operator rulings appended
to [questions.md](questions.md) Q5.

## Source
- URL pattern: `https://data.binance.vision/data/spot/monthly/klines/SOLUSDC/1d/SOLUSDC-1d-<YYYY-MM>.zip`
  (+ `.CHECKSUM` sibling, sha256-verified per month; keyless public archive).
- Ingestion run (operator, 2026-07-11): requested 2020-08..2026-06 → **44 months fetched, 27
  skipped (HTTP 404)**. The 404s form two blocks: pre-listing (2020-08..2021-08) and a
  **delisting gap 2022-10..2023-11**.

## Span-shrink ruling (Q3: shrink, never patch)
The raw snapshot held two contiguous islands: 2021-09..2022-09 (13 months) and 2023-12..2026-06
(31 months), separated by the 14-month delisting gap — `data-validate` correctly FAILED on the
gap (first offending bar 2023-12-28T00:00:00Z). **Operator ruling 2026-07-11: keep the later,
longer, more-recent island; the 2021-09..2022-09 island was deleted from the snapshot.** No
patching, no bridging, no forward-fill.

## Confirmed hygiene record (post-shrink)
- `data-validate: OK — 916 bars, 2023-12-28..2026-06-30, spacing 86400s, fnv1a64 0x65a1e18b7554a1c5`
- Combined CSV content hash (`shasum -a 256 data/raw/binance/SOLUSDC-1d/*.csv | shasum -a 256`):
  `1c7e70bc31d7ca76f135144a42877b34fdeaa5bd28c47f43ac9216e987838e25`
  (31 CSVs, 2023-12..2026-06). Pre-shrink 44-CSV hash, for the record:
  `513e262f9a3af7257f74d23223bc54373b1adf758d0b0b9728437a13a56aa6a1`.

## Partition + window arithmetic (frozen)
- Holdout = final ~20% by date: 80% point = 2025-12-29 → rounded to the 1st of the next month →
  `holdout.start = 2026-01-01`. Holdout = 181 bars (19.8%); dev/val = 735 bars.
  `validation.start = 2025-01-01` (one year before holdout.start).
- Walk-forward: the span supports at most **4** rolling 365/90/90/5 windows in the 735-bar
  dev/val, vs Q5's min_windows 6 — an operator ruling was required. **Ruled 2026-07-11:
  rolling 365/60/60/5, min_windows 6** (train length and window count preserved; test/step
  shortened 90 → 60; total out-of-sample coverage unchanged at 360 days).
  Check: floor((735 − 365 − 5 − 60)/60) + 1 = **6 windows** ≥ min_windows 6.

## CEX-proxy caveat (Q3 resolution, verbatim)
> The CEX-proxy caveat is acknowledged and must be recorded in the sweep report's provenance
> notes; execution realism is owned by M6 shadow + the S10 cost ladder.

(Binance SOLUSDC is a CEX price series standing proxy for on-chain SOL/USDC — DEX pool prices,
spreads, and liquidity differ. Any M5 advancement decision carries this caveat forward.)

## Ratchet
Frozen 2026-07-11, before any strategy result was computed on real data. `m5-frozen.toml` is
immutable for the M5 cycle (Q5 one-way ratchet). If the snapshot is ever re-ingested, the fnv1a64
and combined hash above must reproduce exactly, or the freeze is void and the sitting re-runs.
