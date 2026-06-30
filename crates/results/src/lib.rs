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
            turnover: None,
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

    #[test]
    fn stat_formatting_handles_non_finite() {
        assert_eq!(stat_to_string(None), None);
        assert_eq!(stat_to_string(Some(f64::NAN)), None);
        assert_eq!(stat_to_string(Some(f64::INFINITY)), None);
        assert_eq!(stat_to_string(Some(0.5)), Some("0.5000000000".to_string()));
    }
}
