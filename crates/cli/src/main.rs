//! `machina` CLI — a deterministic demo that wires the research crates end-to-end.
//!
//! Research only: it loads the checked-in allowlist template, builds a tiny SYNTHETIC SOL/USDC bar
//! series (no network, no real market data), validates it, runs each strategy through the
//! deterministic simulator, computes metrics, and prints schema-valid `RunResult`s.
//!
//! There is no key handling, RPC client, or transaction path anywhere in this binary. Output is a
//! research comparison and implies NO profitability.

use market_data::binance_csv::{fnv1a64, load_dir};
use market_data::{validate_series_spacing, Allowlist, DataError};
use metrics::Metrics;
use portfolio::{run, CostModel};
use research_core::{Bar, Decimal, Timestamp};
use results::{RunInputs, RunResult};
use std::num::NonZeroUsize;
use std::path::Path;
use strategies::{
    BuyAndHoldSol, DcaIntoSol, HoldUsdc, Static5050, Strategy, ThresholdRebalanceV1, TrendAllocV1,
};
use sweep::{run_sweep, Parallelism, SweepSpec};

/// The allowlist template is embedded at compile time so the demo needs no runtime files.
const ALLOWLIST_TEMPLATE: &str = include_str!("../../../config/tokens/allowlist.example.toml");

/// The strategy-lab template is embedded at compile time so `sweep` needs no runtime files.
const STRATEGY_LAB_TEMPLATE: &str =
    include_str!("../../../config/strategies/strategy-lab.example.toml");

/// Bars per year for annualizing daily-bar statistics.
const PERIODS_PER_YEAR: f64 = 365.0;

fn main() {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("demo") => demo(),
        Some("sweep") => sweep_cmd(args),
        Some("sweep-verify") => sweep_verify(),
        Some("data-validate") => data_validate_cmd(args),
        Some("--help" | "-h" | "help") | None => usage(),
        Some(other) => {
            eprintln!("unknown command: {other}\n");
            usage();
            std::process::exit(2);
        }
    }
}

fn usage() {
    println!(
        "machina — paper-first Solana research platform (research mode only)\n\n\
         USAGE:\n  machina demo    Run the deterministic demo (no network, no keys)\n  \
         machina sweep [--threads N] [--out PATH] [--config PATH --data DIR]    Deterministic parameter sweep (research only)\n  \
         machina sweep-verify    Assert parallel == sequential byte-identical (CI-gate mirror)\n\n\
         This binary has no key, RPC, or transaction-submission path. See docs/invariants.md."
    );
}

/// `(parallelism, --out path, (--config path, --data dir))` — the last pair is `None` unless both
/// flags were given together.
type SweepArgs = (Parallelism, Option<String>, Option<(String, String)>);

/// Parse `sweep` options. Pure so it is unit-testable; `sweep_cmd` maps `Err` to `exit(2)`.
/// `--config`/`--data` must be given together (both or neither) — one without the other is a
/// usage error.
fn parse_sweep_args(mut args: impl Iterator<Item = String>) -> Result<SweepArgs, String> {
    let mut threads: Option<usize> = None;
    let mut out: Option<String> = None;
    let mut config: Option<String> = None;
    let mut data: Option<String> = None;
    while let Some(a) = args.next() {
        match a.as_str() {
            "--threads" => {
                let v = args
                    .next()
                    .ok_or_else(|| "--threads needs a value".to_string())?;
                threads = Some(
                    v.parse()
                        .map_err(|_| "--threads must be a positive integer".to_string())?,
                );
            }
            "--out" => {
                out = Some(
                    args.next()
                        .ok_or_else(|| "--out needs a path".to_string())?,
                );
            }
            "--config" => {
                config = Some(
                    args.next()
                        .ok_or_else(|| "--config needs a path".to_string())?,
                );
            }
            "--data" => {
                data = Some(
                    args.next()
                        .ok_or_else(|| "--data needs a path".to_string())?,
                );
            }
            other => return Err(format!("unknown sweep option: {other}")),
        }
    }
    let parallelism = match threads {
        None => Parallelism::Sequential,
        Some(0) => return Err("--threads must be >= 1".to_string()),
        Some(n) => Parallelism::Threads(NonZeroUsize::new(n).expect("n >= 1")),
    };
    let real_data = match (config, data) {
        (Some(c), Some(d)) => Some((c, d)),
        (None, None) => None,
        _ => return Err("--config and --data must be given together".to_string()),
    };
    Ok((parallelism, out, real_data))
}

fn sweep_cmd(args: impl Iterator<Item = String>) {
    let (parallelism, out, real_data) = parse_sweep_args(args).unwrap_or_else(|message| {
        eprintln!("{message}");
        std::process::exit(2)
    });
    let json = match real_data {
        Some((config, data)) => sweep_report_json_real(&config, &data, parallelism),
        None => sweep_report_json(parallelism),
    };
    match out {
        Some(p) => {
            std::fs::write(&p, &json).expect("write report file");
            eprintln!("wrote {p}");
        }
        None => println!("{json}"),
    }
}

/// Local mirror of the CI determinism gate: the canonical sweep must be byte-identical across
/// Sequential and explicit thread counts. Exits non-zero on any mismatch.
fn sweep_verify() {
    let sequential = sweep_report_json(Parallelism::Sequential);
    for n in [2usize, 8] {
        let parallel =
            sweep_report_json(Parallelism::Threads(NonZeroUsize::new(n).expect("n >= 1")));
        if parallel != sequential {
            eprintln!("sweep-verify: MISMATCH at {n} threads (parallel output != sequential)");
            std::process::exit(1);
        }
    }
    let repeat = sweep_report_json(Parallelism::Sequential);
    if repeat != sequential {
        eprintln!("sweep-verify: MISMATCH on repeated sequential run");
        std::process::exit(1);
    }
    println!(
        "sweep-verify: OK — byte-identical across sequential, 2 and 8 threads, and repeat ({} bytes)",
        sequential.len()
    );
}

/// Parse `data-validate` options. Pure so it is unit-testable; `data_validate_cmd` maps `Err` to
/// `exit(2)`.
fn parse_data_validate_args(
    mut args: impl Iterator<Item = String>,
) -> Result<(String, i64), String> {
    let mut dir: Option<String> = None;
    let mut interval_secs: i64 = 86_400;
    while let Some(a) = args.next() {
        match a.as_str() {
            "--dir" => {
                dir = Some(
                    args.next()
                        .ok_or_else(|| "--dir needs a path".to_string())?,
                );
            }
            "--interval-secs" => {
                let v = args
                    .next()
                    .ok_or_else(|| "--interval-secs needs a value".to_string())?;
                interval_secs = v
                    .parse()
                    .map_err(|_| "--interval-secs must be an integer".to_string())?;
            }
            other => return Err(format!("unknown data-validate option: {other}")),
        }
    }
    let dir = dir.ok_or_else(|| "--dir PATH is required".to_string())?;
    Ok((dir, interval_secs))
}

fn data_validate_cmd(args: impl Iterator<Item = String>) {
    let (dir, interval_secs) = parse_data_validate_args(args).unwrap_or_else(|message| {
        eprintln!("{message}");
        std::process::exit(2)
    });
    match run_data_validate(Path::new(&dir), interval_secs) {
        Ok(line) => println!("{line}"),
        Err(message) => {
            eprintln!("{message}");
            std::process::exit(1);
        }
    }
}

/// Load a Binance CSV snapshot directory and hygiene-check it; format the operator-facing report
/// line. Pure aside from the filesystem read, so its formatting is unit-testable.
fn run_data_validate(dir: &Path, interval_secs: i64) -> Result<String, String> {
    let bars = load_dir(dir).map_err(|e| e.to_string())?;
    if let Err(e) = validate_series_spacing(&bars, interval_secs) {
        let mut message = format!("data-validate: FAILED — {e}");
        if let DataError::Gap { index, .. } = e {
            message.push_str(&format!(
                "\nfirst offending bar: {} (shrink the span before this timestamp — Q3: never patch)",
                bars[index].ts.to_rfc3339()
            ));
        }
        return Err(message);
    }
    let first = bars
        .first()
        .expect("validate_series_spacing rejects empty series");
    let last = bars
        .last()
        .expect("validate_series_spacing rejects empty series");
    Ok(format!(
        "data-validate: OK — {} bars, {}..{}, spacing {interval_secs}s, fnv1a64 0x{:x}",
        bars.len(),
        &first.ts.to_rfc3339()[..10],
        &last.ts.to_rfc3339()[..10],
        fnv1a64(&bars)
    ))
}

/// Run the canonical M4 sweep on the embedded templates + synthetic series; return report JSON.
/// Proves the holdout stayed sealed before returning. NEVER calls evaluate_on_holdout (M5-only).
fn sweep_report_json(parallelism: Parallelism) -> String {
    let allowlist =
        Allowlist::from_toml_str(ALLOWLIST_TEMPLATE).expect("allowlist template parses");
    allowlist.require("SOL").expect("SOL allowlisted");
    allowlist.require("USDC").expect("USDC allowlisted");
    let spec =
        SweepSpec::from_toml_str(STRATEGY_LAB_TEMPLATE).expect("strategy-lab template parses");
    let bars = sweep_series();
    validate_series_spacing(&bars, 86_400).expect("sweep series is valid and gap-free");
    let cost = CostModel {
        dex_fee_bps: 5,
        slippage_bps: 20,
        base_fee_lamports: 5_000,
        priority_fee_lamports: 50_000,
    };
    let outcome = run_sweep(
        &spec,
        bars,
        &cost,
        Decimal::from(10_000),
        PERIODS_PER_YEAR,
        parallelism,
    )
    .expect("sweep runs");
    assert_eq!(
        outcome.sealed.holdout_read_count(),
        0,
        "M4 must never read the holdout"
    );
    outcome.report.to_json()
}

/// Run the sweep on the frozen `--config` spec + real `--data` snapshot (M5-C4). Same pipeline and
/// hardcoded cost model as the template path (M4's modeled-cost assumptions, unchanged and now
/// frozen with the cycle); proves the holdout stayed sealed before returning. NEVER calls
/// `evaluate_on_holdout` (M5-C6 only).
fn sweep_report_json_real(config_path: &str, data_dir: &str, parallelism: Parallelism) -> String {
    let toml = std::fs::read_to_string(config_path).unwrap_or_else(|e| {
        eprintln!("failed to read --config {config_path}: {e}");
        std::process::exit(2)
    });
    let spec = SweepSpec::from_toml_str(&toml).unwrap_or_else(|e| {
        eprintln!("failed to parse --config {config_path}: {e}");
        std::process::exit(2)
    });
    let bars = load_dir(Path::new(data_dir)).unwrap_or_else(|e| {
        eprintln!("failed to load --data {data_dir}: {e}");
        std::process::exit(2)
    });
    validate_series_spacing(&bars, 86_400).unwrap_or_else(|e| {
        eprintln!("--data {data_dir} failed hygiene validation: {e}");
        std::process::exit(2)
    });
    let digest = fnv1a64(&bars);
    let cost = CostModel {
        dex_fee_bps: 5,
        slippage_bps: 20,
        base_fee_lamports: 5_000,
        priority_fee_lamports: 50_000,
    };
    let outcome = run_sweep(
        &spec,
        bars,
        &cost,
        Decimal::from(10_000),
        PERIODS_PER_YEAR,
        parallelism,
    )
    .expect("sweep runs");
    assert_eq!(
        outcome.sealed.holdout_read_count(),
        0,
        "M5 must never read the holdout outside the M5-C6 decision card"
    );
    let provenance = format!(
        "binance data.binance.vision SOLUSDC 1d snapshot (CEX-proxy; see plans/m5-data-validation.md), fnv1a64 0x{digest:x}"
    );
    outcome.report.with_provenance(&provenance).to_json()
}

/// Deterministic synthetic daily series spanning the template's [partitions] dates
/// (2021-01-01..2025-12-31 = 1826 bars). Same sawtooth family as the sweep test fixtures.
/// NOT real market data (research demo only; real ingestion is the M5 data question, Q3).
fn sweep_series() -> Vec<Bar> {
    const DAY: i64 = 86_400;
    const START: i64 = 1_609_459_200; // 2021-01-01T00:00:00Z
    (0..1826)
        .map(|i| {
            let p = Decimal::from(100 + (i as i64 % 7) * 3 + (i as i64 / 7) * 2);
            Bar {
                ts: Timestamp::from_unix(START + i as i64 * DAY),
                open: p,
                high: p,
                low: p,
                close: p,
                volume: Decimal::from(1_000),
            }
        })
        .collect()
}

fn demo() {
    // 1. Load + check the allowlist (the trading gate). SOL and USDC must be present.
    let allowlist =
        Allowlist::from_toml_str(ALLOWLIST_TEMPLATE).expect("allowlist template parses");
    allowlist.require("SOL").expect("SOL allowlisted");
    allowlist.require("USDC").expect("USDC allowlisted");

    // 2. Build and validate a tiny SYNTHETIC daily series (deterministic; not real market data).
    //    The stricter spacing check also rejects any missing/gapped bar (no silent forward-fill).
    let bars = synthetic_series();
    validate_series_spacing(&bars, 86_400).expect("synthetic series is valid and gap-free");

    // 3. Conservative modeled costs (NOT measured). Same assumptions for every strategy.
    let cost = CostModel {
        dex_fee_bps: 5,
        slippage_bps: 20,
        base_fee_lamports: 5_000,
        priority_fee_lamports: 50_000,
    };

    println!("machina demo — deterministic research run (mode: research; no network, no keys)");
    println!(
        "allowlist {} | tokens: {} | bars: {} daily (synthetic)\n",
        allowlist.version(),
        allowlist.token_ids().collect::<Vec<_>>().join(", "),
        bars.len()
    );

    // 4. Run each strategy and print a one-line summary. Strategies are illustrative scaffolds;
    //    this is a wiring demo, NOT evidence any strategy works.
    let strategies: Vec<Box<dyn Strategy>> = vec![
        Box::new(HoldUsdc),
        Box::new(BuyAndHoldSol),
        Box::new(Static5050),
        Box::new(DcaIntoSol::illustrative()),
        Box::new(TrendAllocV1 {
            sma_period: 5,
            weight_above: Decimal::new(75, 2),
            weight_below: Decimal::ZERO,
        }),
        Box::new(ThresholdRebalanceV1::illustrative()),
    ];

    println!(
        "{:<24} {:>13} {:>9} {:>7} {:>12}",
        "strategy", "total_return", "max_dd", "trades", "fees_usdc"
    );
    let mut selected_json = String::new();
    for strat in &strategies {
        let rr = run_strategy(strat.as_ref(), &bars, &cost, &allowlist);
        println!(
            "{:<24} {:>13} {:>9} {:>7} {:>12}",
            rr.config.strategy,
            rr.metrics.total_return,
            rr.metrics.max_drawdown,
            rr.metrics.n_trades,
            rr.metrics.fees_paid_usdc.clone().unwrap_or_default(),
        );
        if rr.config.strategy == "trend_alloc_v1" {
            selected_json = rr.to_json();
        }
    }

    println!("\n--- RunResult (trend_alloc_v1), schema-valid ---\n{selected_json}");
    println!(
        "\nNote: synthetic data + illustrative parameters. No profitability is implied; \
         advancement requires the M4–M5 validation battery."
    );
}

/// Run one strategy through the simulator and assemble its `RunResult`.
fn run_strategy(
    strat: &dyn Strategy,
    bars: &[Bar],
    cost: &CostModel,
    allowlist: &Allowlist,
) -> RunResult {
    let initial_cash = Decimal::from(10_000);
    let out =
        run(bars, initial_cash, cost, |h, w| strat.target_weight(h, w)).expect("simulation runs");
    let equity: Vec<Decimal> = out.equity_curve.iter().map(|p| p.equity_quote).collect();
    let m = Metrics::from_equity(&equity, PERIODS_PER_YEAR);
    let final_price = bars.last().expect("non-empty").close;
    let inputs = RunInputs {
        run_id: format!("demo-{}-0001", strat.name()),
        // Deterministic: derived from the data, not the wall clock.
        created_at: bars.last().expect("non-empty").ts.to_rfc3339(),
        mode: "research",
        allowlist_version: allowlist.version().to_string(),
        strategy: strat.name().to_string(),
        base_symbol: "SOL".to_string(),
        quote_symbol: "USDC".to_string(),
        bar_interval: "1d".to_string(),
        initial_cash_usdc: initial_cash,
        cost,
        final_price,
        // Turnover is a first-class sweep output (M4); the wiring demo leaves it unset.
        turnover: None,
        include_series: strat.name() == "trend_alloc_v1",
    };
    RunResult::build(&inputs, &out, &m)
}

/// A deterministic synthetic SOL/USDC daily path: an uptrend with pullbacks. Degenerate intrabar
/// (open=high=low=close) keeps it trivially OHLC-valid. NOT real market data.
fn synthetic_series() -> Vec<Bar> {
    const CLOSES: [i64; 24] = [
        100, 102, 105, 103, 108, 112, 115, 110, 107, 111, 118, 125, 130, 128, 122, 119, 124, 131,
        138, 135, 129, 133, 140, 145,
    ];
    const DAY: i64 = 86_400;
    const START: i64 = 1_609_459_200; // 2021-01-01T00:00:00Z
    CLOSES
        .iter()
        .enumerate()
        .map(|(i, &c)| {
            let p = Decimal::from(c);
            Bar {
                ts: Timestamp::from_unix(START + i as i64 * DAY),
                open: p,
                high: p,
                low: p,
                close: p,
                volume: Decimal::from(1_000),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn allowlist() -> Allowlist {
        Allowlist::from_toml_str(ALLOWLIST_TEMPLATE).expect("allowlist template parses")
    }

    fn demo_cost() -> CostModel {
        CostModel {
            dex_fee_bps: 5,
            slippage_bps: 20,
            base_fee_lamports: 5_000,
            priority_fee_lamports: 50_000,
        }
    }

    fn trend_alloc() -> TrendAllocV1 {
        TrendAllocV1 {
            sma_period: 5,
            weight_above: Decimal::new(75, 2),
            weight_below: Decimal::ZERO,
        }
    }

    #[test]
    fn synthetic_series_is_ohlc_valid_and_gap_free() {
        let bars = synthetic_series();
        assert_eq!(bars.len(), 24);
        for b in &bars {
            b.check_ohlc().expect("degenerate bar is OHLC-valid");
            // Degenerate intrabar: open == high == low == close.
            assert_eq!(b.open, b.high);
            assert_eq!(b.high, b.low);
            assert_eq!(b.low, b.close);
            assert_eq!(b.volume, Decimal::from(1_000));
        }
        // Exactly-daily spacing, strictly increasing, no gaps (no silent forward-fill).
        validate_series_spacing(&bars, 86_400).expect("uniform daily spacing");
        assert_eq!(bars[0].ts, Timestamp::from_unix(1_609_459_200));
    }

    #[test]
    fn synthetic_series_is_deterministic() {
        // Determinism invariant: the demo's data source is a pure function of nothing.
        assert_eq!(synthetic_series(), synthetic_series());
    }

    #[test]
    fn run_strategy_is_deterministic() {
        // The whole demo pipeline (sim → metrics → RunResult) is byte-for-byte reproducible.
        let al = allowlist();
        let bars = synthetic_series();
        let cost = demo_cost();
        let s = trend_alloc();
        assert_eq!(
            run_strategy(&s, &bars, &cost, &al),
            run_strategy(&s, &bars, &cost, &al)
        );
    }

    #[test]
    fn run_strategy_stamps_schema_shaped_config() {
        let al = allowlist();
        let bars = synthetic_series();
        let rr = run_strategy(&HoldUsdc, &bars, &demo_cost(), &al);
        assert_eq!(rr.schema_version, results::SCHEMA_VERSION);
        assert_eq!(rr.mode, "research");
        assert_eq!(rr.config.strategy, "hold_usdc");
        assert_eq!(rr.config.base_symbol, "SOL");
        assert_eq!(rr.config.quote_symbol, "USDC");
        assert_eq!(rr.config.bar_interval, "1d");
        assert_eq!(rr.allowlist_version, al.version());
        assert_eq!(rr.config.initial_cash_usdc, "10000");
        // created_at is derived from the data (last bar), never the wall clock.
        assert_eq!(rr.created_at, bars.last().unwrap().ts.to_rfc3339());
    }

    #[test]
    fn hold_usdc_makes_no_trades_and_keeps_cash() {
        // Sanity-check the full wiring on a strategy with a known closed form.
        let al = allowlist();
        let bars = synthetic_series();
        let rr = run_strategy(&HoldUsdc, &bars, &demo_cost(), &al);
        assert_eq!(rr.metrics.n_trades, 0);
        assert_eq!(rr.final_balances.quote, "10000");
        assert_eq!(rr.final_balances.base, "0");
        assert_eq!(rr.final_balances.equity_quote, "10000");
        assert_eq!(rr.metrics.total_return, "0");
    }

    #[test]
    fn sweep_series_is_valid_and_spans_the_partitions() {
        let bars = sweep_series();
        assert_eq!(bars.len(), 1826);
        validate_series_spacing(&bars, 86_400).expect("uniform daily spacing");
        assert_eq!(bars[0].ts, Timestamp::from_unix(1_609_459_200));
    }

    #[test]
    fn sweep_report_is_deterministic_and_parallel_equal() {
        let a = sweep_report_json(Parallelism::Sequential);
        let b = sweep_report_json(Parallelism::Sequential);
        let c = sweep_report_json(Parallelism::Threads(NonZeroUsize::new(2).unwrap()));
        assert_eq!(a, b);
        assert_eq!(a, c);
    }

    #[test]
    fn sweep_verify_inputs_agree() {
        let sequential = sweep_report_json(Parallelism::Sequential);
        let threads_2 = sweep_report_json(Parallelism::Threads(NonZeroUsize::new(2).unwrap()));
        let threads_8 = sweep_report_json(Parallelism::Threads(NonZeroUsize::new(8).unwrap()));
        assert_eq!(sequential, threads_2);
        assert_eq!(sequential, threads_8);
    }

    #[test]
    fn parse_sweep_args_defaults_to_sequential_with_no_output_path() {
        let (parallelism, out, real_data) = parse_sweep_args(std::iter::empty()).unwrap();
        assert_eq!(parallelism, Parallelism::Sequential);
        assert_eq!(out, None);
        assert_eq!(real_data, None);
    }

    #[test]
    fn parse_sweep_args_reads_threads_and_out() {
        let args = ["--threads", "2", "--out", "x.json"].map(String::from);
        let (parallelism, out, real_data) = parse_sweep_args(args.into_iter()).unwrap();
        assert_eq!(
            parallelism,
            Parallelism::Threads(NonZeroUsize::new(2).unwrap())
        );
        assert_eq!(out, Some("x.json".to_string()));
        assert_eq!(real_data, None);
    }

    #[test]
    fn parse_sweep_args_reads_config_and_data_together() {
        let args = ["--config", "c.toml", "--data", "d/"].map(String::from);
        let (_, _, real_data) = parse_sweep_args(args.into_iter()).unwrap();
        assert_eq!(real_data, Some(("c.toml".to_string(), "d/".to_string())));
    }

    #[test]
    fn parse_sweep_args_rejects_config_without_data() {
        let args = ["--config", "c.toml"].map(String::from);
        assert!(parse_sweep_args(args.into_iter()).is_err());
    }

    #[test]
    fn parse_sweep_args_rejects_data_without_config() {
        let args = ["--data", "d/"].map(String::from);
        assert!(parse_sweep_args(args.into_iter()).is_err());
    }

    #[test]
    fn parse_sweep_args_rejects_zero_threads() {
        let args = ["--threads", "0"].map(String::from);
        assert!(parse_sweep_args(args.into_iter()).is_err());
    }

    #[test]
    fn parse_sweep_args_rejects_non_integer_threads() {
        let args = ["--threads", "abc"].map(String::from);
        assert!(parse_sweep_args(args.into_iter()).is_err());
    }

    #[test]
    fn parse_sweep_args_rejects_out_with_no_value() {
        let args = ["--out"].map(String::from);
        assert!(parse_sweep_args(args.into_iter()).is_err());
    }

    #[test]
    fn parse_sweep_args_rejects_unknown_option() {
        let args = ["--bogus"].map(String::from);
        assert!(parse_sweep_args(args.into_iter()).is_err());
    }

    #[test]
    fn include_series_only_for_trend_alloc_v1() {
        let al = allowlist();
        let bars = synthetic_series();
        let cost = demo_cost();
        // trend_alloc_v1 embeds the full equity curve...
        let rr_trend = run_strategy(&trend_alloc(), &bars, &cost, &al);
        assert_eq!(rr_trend.equity_curve.len(), bars.len());
        // ...every other strategy keeps the result compact.
        let rr_hold = run_strategy(&HoldUsdc, &bars, &cost, &al);
        assert!(rr_hold.equity_curve.is_empty());
        assert!(rr_hold.round_trips.is_empty());
    }

    #[test]
    fn data_validate_reports_ok_line_for_a_small_valid_snapshot() {
        let dir =
            std::env::temp_dir().join(format!("machina-data-validate-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("create temp snapshot dir");
        std::fs::write(
            dir.join("SOLUSDC-1d-2021-01.csv"),
            "1609459200000,100,110,90,105,1000,0,0,0,0,0,0\n\
             1609545600000,105,115,95,110,1200,0,0,0,0,0,0\n\
             1609632000000,110,120,100,115,1300,0,0,0,0,0,0\n",
        )
        .expect("write fixture csv");

        let result = run_data_validate(&dir, 86_400);
        std::fs::remove_dir_all(&dir).expect("clean up temp snapshot dir");

        assert_eq!(
            result.expect("valid snapshot validates"),
            "data-validate: OK — 3 bars, 2021-01-01..2021-01-03, spacing 86400s, \
             fnv1a64 0xf57d42c4827d80e0"
        );
    }
}
