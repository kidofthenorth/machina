//! Non-lookahead `CongestionRegime` classifier over trailing bar volume (m-hf-track C8.6a).
//!
//! Nothing calls this yet — `run_hf_priced` (C8.6b) is its first real caller. Standalone and
//! independently testable.

use portfolio::CongestionRegime;
use research_core::Bar;

/// Classify each bar's landing congestion regime from trailing volume history.
///
/// `regimes[i]` prices the trade landing at `bars[i]`'s OPEN, so it must depend only on bars
/// with index `< i`: it classifies the most recently CLOSED bar (`bars[i-1]`) against a
/// reference window strictly before that (`bars[i-1-lookback..i-1]`) — never `bars[i]` itself,
/// whose volume is not known until bar `i` closes.
///
/// For `i < lookback + 1` (not enough trailing history), defaults to the pessimistic
/// `CongestionRegime::Hot` (fail-closed — a cost to us, never a benefit).
///
/// For `i >= lookback + 1`: ranks `bars[i-1].volume` by counting reference-window volumes
/// strictly less than it. Ties in the reference window count toward the higher (more
/// pessimistic) band. `lookback` splits into three near-equal bands by integer division
/// (`lo = lookback/3`, `hi = lookback - lookback/3`): rank `< lo` → `Calm`; `lo..hi` → `Busy`;
/// `>= hi` → `Hot`.
#[must_use]
pub fn classify_congestion_regimes(bars: &[Bar], lookback: usize) -> Vec<CongestionRegime> {
    let lo = lookback / 3;
    let hi = lookback - lookback / 3;

    bars.iter()
        .enumerate()
        .map(|(i, _)| {
            if i < lookback + 1 {
                return CongestionRegime::Hot;
            }
            let reference = &bars[i - 1 - lookback..i - 1];
            let current = bars[i - 1].volume;
            let rank = reference.iter().filter(|b| b.volume < current).count();
            if rank < lo {
                CongestionRegime::Calm
            } else if rank < hi {
                CongestionRegime::Busy
            } else {
                CongestionRegime::Hot
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use research_core::{Decimal, Timestamp};

    fn bar(ts: i64, volume: i64) -> Bar {
        Bar {
            ts: Timestamp::from_unix(ts),
            open: Decimal::ONE,
            high: Decimal::ONE,
            low: Decimal::ONE,
            close: Decimal::ONE,
            volume: Decimal::from(volume),
        }
    }

    fn series(volumes: &[i64]) -> Vec<Bar> {
        volumes
            .iter()
            .enumerate()
            .map(|(i, &v)| bar(i as i64 * 60, v))
            .collect()
    }

    #[test]
    fn deterministic_repeat_call() {
        let bars = series(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]);
        let a = classify_congestion_regimes(&bars, 3);
        let b = classify_congestion_regimes(&bars, 3);
        assert_eq!(a, b);
    }

    #[test]
    fn non_lookahead_mutating_own_bar_does_not_move_regime() {
        let lookback = 3;
        let mut bars = series(&[10, 20, 30, 40, 50, 60, 70, 80]);
        let i = 6;
        let before = classify_congestion_regimes(&bars, lookback);
        bars[i].volume = Decimal::from(999_999);
        let after = classify_congestion_regimes(&bars, lookback);
        assert_eq!(
            before[i], after[i],
            "mutating bars[i] must not move regimes[i]"
        );
    }

    #[test]
    fn non_lookahead_mutating_prior_bar_does_move_regime() {
        let lookback = 3;
        // bars[i-1] = bars[5] = 1 starts below all of reference [30,40,50] -> rank 0 -> Calm.
        let mut bars = series(&[10, 20, 30, 40, 50, 1, 70, 80]);
        let i = 6;
        let before = classify_congestion_regimes(&bars, lookback);
        bars[i - 1].volume = Decimal::from(999_999);
        let after = classify_congestion_regimes(&bars, lookback);
        assert_ne!(
            before[i], after[i],
            "mutating bars[i-1] must move regimes[i] (test would otherwise pass vacuously)"
        );
    }

    #[test]
    fn pessimistic_default_below_history_threshold() {
        let lookback = 4;
        let bars = series(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
        let regimes = classify_congestion_regimes(&bars, lookback);
        for (i, r) in regimes.iter().enumerate().take(lookback + 1) {
            assert_eq!(*r, CongestionRegime::Hot, "bar {i} should default to Hot");
        }
    }

    #[test]
    fn tercile_boundary_correctness_with_tie_break_high() {
        // lookback = 6 -> lo = 2, hi = 4.
        let lookback = 6;
        // Reference window (indices 0..6): volumes [10,20,30,40,50,60].
        // current = bars[6].volume, evaluated for regimes[7].
        let mut volumes = vec![10, 20, 30, 40, 50, 60];
        // Calm: current ranks below all but at most 1 (rank < 2).
        volumes.push(15); // rank among reference = 1 (only 10 < 15) -> Calm
        volumes.push(1); // dummy filler to keep series length consistent below
        let bars = series(&volumes);
        let regimes = classify_congestion_regimes(&bars, lookback);
        assert_eq!(regimes[7], CongestionRegime::Calm);

        // Busy: rank in [2, 4).
        let mut volumes = vec![10, 20, 30, 40, 50, 60];
        volumes.push(35); // rank = 3 (10,20,30 < 35) -> Busy
        volumes.push(1);
        let bars = series(&volumes);
        let regimes = classify_congestion_regimes(&bars, lookback);
        assert_eq!(regimes[7], CongestionRegime::Busy);

        // Hot: rank >= 4.
        let mut volumes = vec![10, 20, 30, 40, 50, 60];
        volumes.push(55); // rank = 5 -> Hot
        volumes.push(1);
        let bars = series(&volumes);
        let regimes = classify_congestion_regimes(&bars, lookback);
        assert_eq!(regimes[7], CongestionRegime::Hot);

        // Tie-break-high: current equals a reference value exactly; ties count toward the
        // higher band. current = 30 ties with reference[2]=30, so only 10,20 < 30 -> rank 2
        // -> Busy (not Calm), proving ties push pessimistic.
        let mut volumes = vec![10, 20, 30, 40, 50, 60];
        volumes.push(30);
        volumes.push(1);
        let bars = series(&volumes);
        let regimes = classify_congestion_regimes(&bars, lookback);
        assert_eq!(regimes[7], CongestionRegime::Busy);
    }

    #[test]
    fn empty_and_zero_lookback_do_not_panic() {
        let empty: Vec<Bar> = Vec::new();
        assert_eq!(classify_congestion_regimes(&empty, 3), Vec::new());

        let bars = series(&[1, 2, 3, 4]);
        let regimes = classify_congestion_regimes(&bars, 0);
        assert_eq!(regimes.len(), bars.len());
        // lookback = 0 -> lo = 0, hi = 0, so i < 1 defaults Hot; i >= 1 has an empty reference
        // window (rank always 0) and 0 >= hi(=0) -> Hot for every bar.
        for r in &regimes {
            assert_eq!(*r, CongestionRegime::Hot);
        }
    }
}
