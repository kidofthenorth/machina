//! `results` — the canonical run-result model and its JSON-schema contract.
//!
//! The DTOs here serialize to JSON that validates against `schemas/run-result.schema.json`. Money is
//! encoded as **exact decimal strings**, never JSON floats, so downstream consumers cannot silently
//! lose precision. The model is built from a [`portfolio::RunOutput`] and [`metrics::Metrics`].

use metrics::Metrics;
use portfolio::{CostModel, RunOutput};
use research_core::Decimal;
use serde::{Deserialize, Serialize};

/// Version of the run-result schema this crate targets (matches `schema_version` in the JSON).
pub const SCHEMA_VERSION: &str = "1.0.0";

/// Inputs needed to stamp a [`RunResult`] that the simulator/metrics cannot supply on their own.
pub struct RunInputs<'a> {
    pub run_id: String,
    /// RFC 3339 UTC. For deterministic research runs, pass a value derived from the data (e.g. the
    /// last bar's timestamp), NOT the wall clock.
    pub created_at: String,
    /// Execution mode. Research runs are always `"research"`.
    pub mode: &'a str,
    pub allowlist_version: String,
    pub strategy: String,
    pub base_symbol: String,
    pub quote_symbol: String,
    pub bar_interval: String,
    pub initial_cash_usdc: Decimal,
    pub cost: &'a CostModel,
    /// Price used to mark the final equity/balances (USDC per SOL).
    pub final_price: Decimal,
    /// Pre-computed turnover ratio (traded notional / mean equity) as an exact `Decimal`, if known.
    /// Sweeps fill this from `sweep::turnover`; the demo leaves it `None`. Populates
    /// `MetricsReport.turnover` (kept off the f64 path so it is exact and order-stable).
    pub turnover: Option<Decimal>,
    /// Whether to embed the full equity curve and round trips (large for long runs).
    pub include_series: bool,
}

/// Canonical, schema-validated result of a single research run.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RunResult {
    pub schema_version: String,
    pub run_id: String,
    pub created_at: String,
    pub mode: String,
    pub allowlist_version: String,
    pub config: RunConfig,
    pub metrics: MetricsReport,
    pub final_balances: Balances,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub equity_curve: Vec<EquityPointDto>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub round_trips: Vec<RoundTripDto>,
}

impl RunResult {
    /// Assemble a [`RunResult`] from a simulation output and its metrics.
    #[must_use]
    pub fn build(inputs: &RunInputs, out: &RunOutput, m: &Metrics) -> Self {
        let final_equity = out.final_state.equity(inputs.final_price);
        let metrics = MetricsReport {
            total_return: m.total_return.to_string(),
            max_drawdown: m.max_drawdown.to_string(),
            n_trades: out.n_trades,
            cagr: stat_to_string(m.cagr),
            volatility: stat_to_string(m.volatility),
            sharpe: stat_to_string(m.sharpe),
            sortino: stat_to_string(m.sortino),
            calmar: stat_to_string(m.calmar),
            turnover: inputs.turnover.map(|t| t.to_string()),
            time_in_market: Some(out.time_in_market().to_string()),
            fees_paid_usdc: Some(out.fees_paid_quote.to_string()),
            slippage_paid_usdc: Some(out.slippage_paid_quote.to_string()),
            priority_fees_paid_sol: Some(out.priority_fees_paid_sol.to_string()),
        };
        let (equity_curve, round_trips) = if inputs.include_series {
            (
                out.equity_curve
                    .iter()
                    .map(|p| EquityPointDto {
                        timestamp: p.ts.to_rfc3339(),
                        equity_quote: p.equity_quote.to_string(),
                    })
                    .collect(),
                out.round_trips
                    .iter()
                    .map(|r| RoundTripDto {
                        entry_ts: r.entry_ts.to_rfc3339(),
                        exit_ts: r.exit_ts.to_rfc3339(),
                        entry_price: r.entry_price.to_string(),
                        exit_price: r.exit_price.to_string(),
                        qty_base: r.qty_base.to_string(),
                        pnl_quote: r.pnl_quote.to_string(),
                    })
                    .collect(),
            )
        } else {
            (Vec::new(), Vec::new())
        };

        Self {
            schema_version: SCHEMA_VERSION.to_string(),
            run_id: inputs.run_id.clone(),
            created_at: inputs.created_at.clone(),
            mode: inputs.mode.to_string(),
            allowlist_version: inputs.allowlist_version.clone(),
            config: RunConfig {
                strategy: inputs.strategy.clone(),
                base_symbol: inputs.base_symbol.clone(),
                quote_symbol: inputs.quote_symbol.clone(),
                bar_interval: inputs.bar_interval.clone(),
                initial_cash_usdc: inputs.initial_cash_usdc.to_string(),
                cost_model: CostModelDto {
                    dex_fee_bps: inputs.cost.dex_fee_bps,
                    slippage_bps: inputs.cost.slippage_bps,
                    base_fee_lamports: inputs.cost.base_fee_lamports,
                    priority_fee_lamports: inputs.cost.priority_fee_lamports,
                },
            },
            metrics,
            final_balances: Balances {
                base: out.final_state.base_balance.to_string(),
                quote: out.final_state.quote_balance.to_string(),
                equity_quote: final_equity.to_string(),
            },
            equity_curve,
            round_trips,
        }
    }

    /// Serialize to a pretty JSON string.
    #[must_use]
    pub fn to_json(&self) -> String {
        // Infallible for this fixed, string/number-only shape.
        serde_json::to_string_pretty(self).expect("RunResult serializes")
    }

    /// Serialize to a [`serde_json::Value`] (for schema validation).
    #[must_use]
    pub fn to_value(&self) -> serde_json::Value {
        serde_json::to_value(self).expect("RunResult serializes")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RunConfig {
    pub strategy: String,
    pub base_symbol: String,
    pub quote_symbol: String,
    pub bar_interval: String,
    pub initial_cash_usdc: String,
    pub cost_model: CostModelDto,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CostModelDto {
    pub dex_fee_bps: u32,
    pub slippage_bps: u32,
    pub base_fee_lamports: i64,
    pub priority_fee_lamports: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MetricsReport {
    pub total_return: String,
    pub max_drawdown: String,
    pub n_trades: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cagr: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volatility: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sharpe: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sortino: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub calmar: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turnover: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_in_market: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fees_paid_usdc: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slippage_paid_usdc: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority_fees_paid_sol: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Balances {
    pub base: String,
    pub quote: String,
    pub equity_quote: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EquityPointDto {
    pub timestamp: String,
    pub equity_quote: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RoundTripDto {
    pub entry_ts: String,
    pub exit_ts: String,
    pub entry_price: String,
    pub exit_price: String,
    pub qty_base: String,
    pub pnl_quote: String,
}

/// Format a statistical (f64) metric as a decimal string, or `None` if absent/non-finite.
fn stat_to_string(x: Option<f64>) -> Option<String> {
    x.filter(|v| v.is_finite()).map(|v| format!("{v:.10}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use metrics::Metrics;
    use portfolio::{EquityPoint, PortfolioState, RoundTrip, RunOutput};
    use research_core::Timestamp;
    use rust_decimal_macros::dec;

    #[test]
    fn stat_formatting_handles_non_finite() {
        assert_eq!(stat_to_string(None), None);
        assert_eq!(stat_to_string(Some(f64::NAN)), None);
        assert_eq!(stat_to_string(Some(f64::INFINITY)), None);
        assert_eq!(stat_to_string(Some(0.5)), Some("0.5000000000".to_string()));
    }

    #[test]
    fn stat_formatting_is_fixed_ten_places_and_signed() {
        // Negative and rounding behavior of the `{:.10}` display format.
        assert_eq!(
            stat_to_string(Some(-2.0)),
            Some("-2.0000000000".to_string())
        );
        assert_eq!(
            stat_to_string(Some(f64::NEG_INFINITY)),
            None,
            "−∞ is non-finite → dropped"
        );
    }

    // ── fixtures ────────────────────────────────────────────────────────────
    // Hand-built so every emitted string is a known constant, isolating the
    // `build` mapping from the simulator's arithmetic.

    fn sample_cost() -> CostModel {
        CostModel {
            dex_fee_bps: 5,
            slippage_bps: 20,
            base_fee_lamports: 5_000,
            priority_fee_lamports: 50_000,
        }
    }

    fn sample_output() -> RunOutput {
        RunOutput {
            equity_curve: vec![
                EquityPoint {
                    ts: Timestamp::from_unix(1_609_459_200), // 2021-01-01T00:00:00Z
                    equity_quote: dec!(10000),
                },
                EquityPoint {
                    ts: Timestamp::from_unix(1_609_545_600), // 2021-01-02T00:00:00Z
                    equity_quote: dec!(10250.5),
                },
            ],
            round_trips: vec![RoundTrip {
                entry_ts: Timestamp::from_unix(1_609_459_200),
                exit_ts: Timestamp::from_unix(1_609_545_600),
                entry_price: dec!(100),
                exit_price: dec!(105),
                qty_base: dec!(9.5),
                pnl_quote: dec!(47.5),
            }],
            final_state: PortfolioState {
                quote_balance: dec!(250.5),
                base_balance: dec!(95),
            },
            n_trades: 2,
            traded_notional_quote: dec!(2000),
            fees_paid_quote: dec!(1.234567),
            slippage_paid_quote: dec!(4.5),
            gas_paid_sol: dec!(0.00011),
            priority_fees_paid_sol: dec!(0.0001),
            bars_in_market: 1,
        }
    }

    fn sample_metrics() -> Metrics {
        Metrics {
            total_return: dec!(0.025),
            max_drawdown: dec!(0.1),
            n_periods: 3,
            cagr: Some(0.5),
            volatility: Some(0.25),
            sharpe: Some(1.5),
            sortino: None, // exercise a None → omitted optional
            calmar: Some(-2.0),
        }
    }

    fn sample_inputs<'a>(
        cost: &'a CostModel,
        include_series: bool,
        turnover: Option<Decimal>,
    ) -> RunInputs<'a> {
        RunInputs {
            run_id: "unit-0001".to_string(),
            created_at: "2021-01-02T00:00:00Z".to_string(),
            mode: "research",
            allowlist_version: "2026-06-29".to_string(),
            strategy: "static_50_50".to_string(),
            base_symbol: "SOL".to_string(),
            quote_symbol: "USDC".to_string(),
            bar_interval: "1d".to_string(),
            initial_cash_usdc: dec!(10000),
            cost,
            final_price: dec!(105),
            turnover,
            include_series,
        }
    }

    #[test]
    fn build_maps_money_to_exact_decimal_strings() {
        let cost = sample_cost();
        let rr = RunResult::build(
            &sample_inputs(&cost, false, None),
            &sample_output(),
            &sample_metrics(),
        );
        assert_eq!(rr.schema_version, SCHEMA_VERSION);
        assert_eq!(rr.run_id, "unit-0001");
        assert_eq!(rr.mode, "research");
        assert_eq!(rr.config.initial_cash_usdc, "10000");
        assert_eq!(rr.metrics.total_return, "0.025");
        assert_eq!(rr.metrics.max_drawdown, "0.1");
        assert_eq!(rr.metrics.n_trades, 2);
        assert_eq!(rr.metrics.fees_paid_usdc.as_deref(), Some("1.234567"));
        assert_eq!(rr.metrics.slippage_paid_usdc.as_deref(), Some("4.5"));
        assert_eq!(rr.metrics.priority_fees_paid_sol.as_deref(), Some("0.0001"));
        // time_in_market = bars_in_market(1) / curve_len(2); rust_decimal division renders this
        // exact one-half at scale 2 ("0.50"). Pinned so the on-wire form can't silently change.
        assert_eq!(rr.metrics.time_in_market.as_deref(), Some("0.50"));
        // final_balances mark against final_price=105: 250.5 + 95*105 = 10225.5.
        assert_eq!(rr.final_balances.base, "95");
        assert_eq!(rr.final_balances.quote, "250.5");
        assert_eq!(rr.final_balances.equity_quote, "10225.5");
        // Cost model copied through verbatim.
        assert_eq!(rr.config.cost_model.dex_fee_bps, 5);
        assert_eq!(rr.config.cost_model.slippage_bps, 20);
        assert_eq!(rr.config.cost_model.base_fee_lamports, 5_000);
        assert_eq!(rr.config.cost_model.priority_fee_lamports, 50_000);
    }

    #[test]
    fn build_formats_statistical_metrics_and_omits_none() {
        let cost = CostModel::zero();
        let rr = RunResult::build(
            &sample_inputs(&cost, false, None),
            &sample_output(),
            &sample_metrics(),
        );
        assert_eq!(rr.metrics.cagr.as_deref(), Some("0.5000000000"));
        assert_eq!(rr.metrics.volatility.as_deref(), Some("0.2500000000"));
        assert_eq!(rr.metrics.sharpe.as_deref(), Some("1.5000000000"));
        assert_eq!(rr.metrics.calmar.as_deref(), Some("-2.0000000000"));
        assert_eq!(rr.metrics.sortino, None);
        assert_eq!(rr.metrics.turnover, None);
    }

    #[test]
    fn money_serializes_as_json_strings_never_numbers() {
        let cost = sample_cost();
        let rr = RunResult::build(
            &sample_inputs(&cost, false, None),
            &sample_output(),
            &sample_metrics(),
        );
        let v = rr.to_value();
        assert!(v["config"]["initial_cash_usdc"].is_string());
        assert!(v["final_balances"]["equity_quote"].is_string());
        assert!(v["metrics"]["total_return"].is_string());
        assert!(v["metrics"]["fees_paid_usdc"].is_string());
    }

    #[test]
    fn include_series_false_omits_series_and_optional_json_keys() {
        let cost = CostModel::zero();
        let rr = RunResult::build(
            &sample_inputs(&cost, false, None),
            &sample_output(),
            &sample_metrics(),
        );
        assert!(rr.equity_curve.is_empty());
        assert!(rr.round_trips.is_empty());
        let v = rr.to_value();
        // skip_serializing_if drops empty vecs and None options entirely.
        assert!(v.get("equity_curve").is_none());
        assert!(v.get("round_trips").is_none());
        assert!(v["metrics"].get("sortino").is_none());
        assert!(v["metrics"].get("turnover").is_none());
    }

    #[test]
    fn include_series_true_populates_dtos_with_rfc3339_timestamps() {
        let cost = CostModel::zero();
        let rr = RunResult::build(
            &sample_inputs(&cost, true, Some(dec!(0.3))),
            &sample_output(),
            &sample_metrics(),
        );
        assert_eq!(rr.equity_curve.len(), 2);
        assert_eq!(rr.equity_curve[0].timestamp, "2021-01-01T00:00:00Z");
        assert_eq!(rr.equity_curve[1].timestamp, "2021-01-02T00:00:00Z");
        assert_eq!(rr.equity_curve[1].equity_quote, "10250.5");
        assert_eq!(rr.round_trips.len(), 1);
        let rt = &rr.round_trips[0];
        assert_eq!(rt.entry_ts, "2021-01-01T00:00:00Z");
        assert_eq!(rt.exit_ts, "2021-01-02T00:00:00Z");
        assert_eq!(rt.entry_price, "100");
        assert_eq!(rt.exit_price, "105");
        assert_eq!(rt.qty_base, "9.5");
        assert_eq!(rt.pnl_quote, "47.5");
        assert_eq!(rr.metrics.turnover.as_deref(), Some("0.3"));
    }

    #[test]
    fn serde_round_trips_losslessly() {
        let cost = sample_cost();
        let rr = RunResult::build(
            &sample_inputs(&cost, true, Some(dec!(0.3))),
            &sample_output(),
            &sample_metrics(),
        );
        let json = rr.to_json();
        let back: RunResult = serde_json::from_str(&json).expect("RunResult round-trips");
        assert_eq!(rr, back);
    }

    #[test]
    fn to_json_is_pretty_printed() {
        let cost = CostModel::zero();
        let rr = RunResult::build(
            &sample_inputs(&cost, false, None),
            &sample_output(),
            &sample_metrics(),
        );
        let json = rr.to_json();
        assert!(json.starts_with("{\n"), "pretty JSON opens with a newline");
        assert!(
            json.contains("\n  \"run_id\""),
            "top-level keys are indented"
        );
    }
}
