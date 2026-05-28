//! Backward scheduling core.
//!
//! See docs/PLAN.md §4 for the algorithm. Key points:
//! - Demand is exploded into units with due dates (spread mode: even/start/end).
//! - For each unit, required window = [due - lead_time, due] using the lead time
//!   of the *due date's* quarter for the unit's product.
//! - Units placed in order of ascending required_start ("earliest start first").
//! - Bays across all factories are one global pool (greedy first-fit).

use chrono::{Datelike, Duration, NaiveDate};
use std::collections::HashMap;

// ---------- Public input types (scheduler is decoupled from DB) ----------

#[derive(Debug, Clone)]
pub struct FactoryInput {
    pub id: String,
    pub name: String,
    /// Baseline bay count used for any quarter without an override.
    pub bays: i64,
    /// Per-(year, quarter) overrides. When present, overrides `bays` for that
    /// specific quarter only.
    pub bay_counts_by_quarter: Vec<BayCountInput>,
}

#[derive(Debug, Clone)]
pub struct BayCountInput {
    pub year: i64,
    pub quarter: i64,
    pub bays: i64,
}

#[derive(Debug, Clone)]
pub struct LeadTimeInput {
    pub year: i64,
    pub quarter: i64,
    pub lead_time_days: i64,
}

#[derive(Debug, Clone)]
pub struct ProductInput {
    pub id: String,
    pub name: String,
    pub lead_times: Vec<LeadTimeInput>,
}

#[derive(Debug, Clone)]
pub struct DemandInput {
    pub id: String,
    pub product_id: String,
    pub period_type: String, // "month" | "quarter"
    pub year: i64,
    pub period_index: i64,
    pub quantity: i64,
    pub spread_mode: String, // "even" | "start" | "end"
}

#[derive(Debug, Clone)]
pub struct ScheduleInput {
    pub factories: Vec<FactoryInput>,
    pub products: Vec<ProductInput>,
    pub demand: Vec<DemandInput>,
}

// ---------- Output types ----------

#[derive(Debug, Clone)]
pub struct ScheduledUnitOut {
    pub demand_id: String,
    pub product_id: String,
    pub factory_id: Option<String>, // None if unshippable
    pub bay_index: Option<i64>,
    pub required_start: NaiveDate,
    pub due_date: NaiveDate,
    pub status: UnitStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnitStatus {
    Shipped,
    Unshippable,
}

impl UnitStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            UnitStatus::Shipped => "shipped",
            UnitStatus::Unshippable => "unshippable",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ScheduleOutput {
    pub units: Vec<ScheduledUnitOut>,
    pub total_demand: usize,
    pub shipped_on_time: usize,
    pub unshippable: usize,
}

// ---------- Period helpers ----------

/// Quarter index (1..=4) for a given date.
pub fn quarter_of(d: NaiveDate) -> i64 {
    ((d.month0() as i64) / 3) + 1
}

/// First date of the given period.
pub fn period_start(period_type: &str, year: i64, period_index: i64) -> Option<NaiveDate> {
    match period_type {
        "quarter" => {
            let month = match period_index {
                1 => 1,
                2 => 4,
                3 => 7,
                4 => 10,
                _ => return None,
            };
            NaiveDate::from_ymd_opt(year as i32, month, 1)
        }
        "month" => {
            if !(1..=12).contains(&period_index) {
                return None;
            }
            NaiveDate::from_ymd_opt(year as i32, period_index as u32, 1)
        }
        _ => None,
    }
}

/// Last date of the given period (inclusive).
pub fn period_end(period_type: &str, year: i64, period_index: i64) -> Option<NaiveDate> {
    let start = period_start(period_type, year, period_index)?;
    let (next_year, next_month) = match period_type {
        "quarter" => {
            let m = start.month() + 3;
            if m > 12 {
                (start.year() + 1, m - 12)
            } else {
                (start.year(), m)
            }
        }
        "month" => {
            let m = start.month() + 1;
            if m > 12 {
                (start.year() + 1, 1)
            } else {
                (start.year(), m)
            }
        }
        _ => return None,
    };
    let next = NaiveDate::from_ymd_opt(next_year, next_month, 1)?;
    Some(next.pred_opt()?)
}

/// Explode an aggregate demand row into per-unit due dates.
pub fn explode_due_dates(d: &DemandInput) -> Vec<NaiveDate> {
    let Some(start) = period_start(&d.period_type, d.year, d.period_index) else {
        return vec![];
    };
    let Some(end) = period_end(&d.period_type, d.year, d.period_index) else {
        return vec![];
    };
    let n = d.quantity.max(0) as usize;
    if n == 0 {
        return vec![];
    }
    let days_span = (end - start).num_days() as i64;

    match d.spread_mode.as_str() {
        "start" => vec![start; n],
        "end" => vec![end; n],
        _ => {
            // "even" — spread quantity across the inclusive period
            if n == 1 {
                // single unit lands at the end of the period (most "expected" due)
                return vec![end];
            }
            // Distribute over [start, end] using fractional positions
            // i in 0..n -> position = (i + 1) / n  (so the last unit lands on end)
            let mut out = Vec::with_capacity(n);
            for i in 0..n {
                let frac = (i as f64 + 1.0) / (n as f64);
                let offset_days = (frac * (days_span as f64 + 1.0) - 1.0).round() as i64;
                let off = offset_days.clamp(0, days_span);
                let d = start + Duration::days(off);
                out.push(d);
            }
            out
        }
    }
}

// ---------- Lead-time lookup ----------

pub struct LtIndex {
    /// product_id -> (year, quarter) -> days
    map: HashMap<String, HashMap<(i64, i64), i64>>,
}

impl LtIndex {
    pub fn new(products: &[ProductInput]) -> Self {
        let mut map: HashMap<String, HashMap<(i64, i64), i64>> = HashMap::new();
        for p in products {
            let mut inner: HashMap<(i64, i64), i64> = HashMap::new();
            for lt in &p.lead_times {
                inner.insert((lt.year, lt.quarter), lt.lead_time_days);
            }
            map.insert(p.id.clone(), inner);
        }
        LtIndex { map }
    }

    /// Lead-time for product on a given due date.
    /// Falls back to (a) latest year/quarter at-or-before, then (b) any defined LT.
    pub fn lookup(&self, product_id: &str, due: NaiveDate) -> Option<i64> {
        let inner = self.map.get(product_id)?;
        let year = due.year() as i64;
        let q = quarter_of(due);
        if let Some(&v) = inner.get(&(year, q)) {
            return Some(v);
        }
        // Fallback: closest defined (year, q) ≤ requested
        let target = (year, q);
        let mut best: Option<((i64, i64), i64)> = None;
        for (&k, &v) in inner.iter() {
            if k <= target {
                if best.map_or(true, |(bk, _)| k > bk) {
                    best = Some((k, v));
                }
            }
        }
        if let Some((_, v)) = best {
            return Some(v);
        }
        // Last resort: any defined LT
        inner.values().next().copied()
    }
}

// ---------- Bay-count lookup (per (factory, quarter)) ----------

/// Effective bay count per (factory, quarter), backed by a baseline.
#[derive(Debug, Clone)]
pub struct BayCountIndex {
    /// factory_id -> baseline bays
    baseline: HashMap<String, i64>,
    /// factory_id -> (year, quarter) -> bays
    overrides: HashMap<String, HashMap<(i64, i64), i64>>,
}

impl BayCountIndex {
    pub fn new(factories: &[FactoryInput]) -> Self {
        let mut baseline: HashMap<String, i64> = HashMap::new();
        let mut overrides: HashMap<String, HashMap<(i64, i64), i64>> = HashMap::new();
        for f in factories {
            baseline.insert(f.id.clone(), f.bays.max(0));
            let mut o: HashMap<(i64, i64), i64> = HashMap::new();
            for bc in &f.bay_counts_by_quarter {
                if (1..=4).contains(&bc.quarter) {
                    o.insert((bc.year, bc.quarter), bc.bays.max(0));
                }
            }
            if !o.is_empty() {
                overrides.insert(f.id.clone(), o);
            }
        }
        BayCountIndex { baseline, overrides }
    }

    /// Effective bay count for the factory in the given quarter. Override wins
    /// over baseline; missing factory returns 0.
    pub fn effective(&self, factory_id: &str, year: i64, quarter: i64) -> i64 {
        if let Some(o) = self.overrides.get(factory_id) {
            if let Some(&v) = o.get(&(year, quarter)) {
                return v;
            }
        }
        self.baseline.get(factory_id).copied().unwrap_or(0)
    }

    /// Maximum effective bay count across baseline and all defined overrides.
    /// Used to size the bay pool so every quarter's needs can be represented.
    pub fn max_for(&self, factory_id: &str) -> i64 {
        let base = self.baseline.get(factory_id).copied().unwrap_or(0);
        let omax = self
            .overrides
            .get(factory_id)
            .and_then(|m| m.values().copied().max())
            .unwrap_or(0);
        base.max(omax)
    }

    /// Minimum effective bay count across every quarter touched by [start, end].
    /// A bay slot with index `i` may be used only if `i < this value`.
    pub fn min_in_window(
        &self,
        factory_id: &str,
        start: NaiveDate,
        end: NaiveDate,
    ) -> i64 {
        debug_assert!(start <= end);
        let mut quarters: Vec<(i64, i64)> = Vec::new();
        let mut d = start;
        loop {
            let key = (d.year() as i64, quarter_of(d));
            if quarters.last() != Some(&key) {
                quarters.push(key);
            }
            // Jump to the first day of the next month to walk forward cheaply
            let nm_year = if d.month() == 12 { d.year() + 1 } else { d.year() };
            let nm_month = if d.month() == 12 { 1 } else { d.month() + 1 };
            let next = match NaiveDate::from_ymd_opt(nm_year, nm_month, 1) {
                Some(v) => v,
                None => break,
            };
            if next > end {
                // capture end's quarter if not yet added
                let key_end = (end.year() as i64, quarter_of(end));
                if quarters.last() != Some(&key_end) {
                    quarters.push(key_end);
                }
                break;
            }
            d = next;
        }
        quarters
            .into_iter()
            .map(|(y, q)| self.effective(factory_id, y, q))
            .min()
            .unwrap_or(0)
    }
}

// ---------- Bay pool ----------

/// A single bay tracks its reserved intervals (sorted, non-overlapping).
#[derive(Debug, Default, Clone)]
pub struct Bay {
    /// Sorted by start. Each interval is [start, end] inclusive.
    intervals: Vec<(NaiveDate, NaiveDate)>,
    /// Cached running total of reserved days across intervals (for load-balancing).
    reserved_days: i64,
}

impl Bay {
    /// Is [start, end] free? (inclusive endpoints)
    pub fn is_free(&self, start: NaiveDate, end: NaiveDate) -> bool {
        for (s, e) in &self.intervals {
            // overlap if s <= end && e >= start
            if *s <= end && *e >= start {
                return false;
            }
        }
        true
    }

    pub fn reserve(&mut self, start: NaiveDate, end: NaiveDate) {
        let pos = self
            .intervals
            .binary_search_by(|(s, _)| s.cmp(&start))
            .unwrap_or_else(|p| p);
        self.intervals.insert(pos, (start, end));
        self.reserved_days += (end - start).num_days() + 1;
    }

    /// Total reserved bay-days. Used for load-balanced placement.
    pub fn reserved_days(&self) -> i64 {
        self.reserved_days
    }
}

#[derive(Debug, Clone)]
pub struct PoolBay {
    pub factory_id: String,
    pub factory_name: String,
    pub bay_index: i64, // 0-based within factory
    pub bay: Bay,
}

#[derive(Debug, Clone)]
pub struct BayPool {
    pub bays: Vec<PoolBay>,
    bay_counts: BayCountIndex,
}

impl BayPool {
    pub fn from_factories(factories: &[FactoryInput]) -> Self {
        let bay_counts = BayCountIndex::new(factories);
        let mut bays = Vec::new();
        for f in factories {
            // Pool size = max over baseline and any per-quarter override, so the
            // bay slot exists when it might be needed. Slots are gated by the
            // per-window effective count in `find_free`.
            let max_bays = bay_counts.max_for(&f.id).max(f.bays.max(0));
            for i in 0..max_bays {
                bays.push(PoolBay {
                    factory_id: f.id.clone(),
                    factory_name: f.name.clone(),
                    bay_index: i,
                    bay: Bay::default(),
                });
            }
        }
        BayPool { bays, bay_counts }
    }

    pub fn bay_counts(&self) -> &BayCountIndex {
        &self.bay_counts
    }

    /// Find the **least-loaded** bay free in [start, end]. Returns index into
    /// `bays`. This spreads demand across factories and across bays within a
    /// factory, rather than always piling on the first bay.
    ///
    /// Slots are also filtered to those that exist for the entire window —
    /// i.e. `bay_index < bay_counts.min_in_window(factory, start, end)`. This
    /// is what lets the bay count vary by quarter.
    ///
    /// Ranking is:
    ///   1. lowest per-factory total reserved days  (split across factories)
    ///   2. lowest per-bay reserved days            (split within a factory)
    ///   3. lowest pool index                       (deterministic tiebreak)
    pub fn find_free(&self, start: NaiveDate, end: NaiveDate) -> Option<usize> {
        // Cache the per-window factory cap so we compute it once per factory.
        let mut window_cap: std::collections::HashMap<&str, i64> =
            std::collections::HashMap::new();

        // Precompute total load per factory once per call.
        let mut factory_load: std::collections::HashMap<&str, i64> =
            std::collections::HashMap::new();
        for b in &self.bays {
            *factory_load.entry(b.factory_id.as_str()).or_insert(0) += b.bay.reserved_days();
        }

        let mut best: Option<(i64, i64, usize)> = None; // (factory_load, bay_load, idx)
        for (i, b) in self.bays.iter().enumerate() {
            // Per-window bay-count gate
            let cap = *window_cap
                .entry(b.factory_id.as_str())
                .or_insert_with(|| self.bay_counts.min_in_window(&b.factory_id, start, end));
            if b.bay_index >= cap {
                continue;
            }
            if !b.bay.is_free(start, end) {
                continue;
            }
            let fl = factory_load.get(b.factory_id.as_str()).copied().unwrap_or(0);
            let bl = b.bay.reserved_days();
            let key = (fl, bl, i);
            match best {
                None => best = Some(key),
                Some(cur) if key < cur => best = Some(key),
                _ => {}
            }
        }
        best.map(|(_, _, i)| i)
    }

    pub fn reserve(&mut self, idx: usize, start: NaiveDate, end: NaiveDate) {
        self.bays[idx].bay.reserve(start, end);
    }
}

// ---------- Run ----------

/// Run the backward-scheduling algorithm.
pub fn run_schedule(input: &ScheduleInput) -> ScheduleOutput {
    run_schedule_with_lt(input, |_pid, lt| lt) // identity transform
}

/// Run scheduling with an optional per-product lead-time transformation.
/// Used by Phase 3 recommendations (e.g. uniform % reduction, per-product overrides).
pub fn run_schedule_with_lt<F>(input: &ScheduleInput, mut lt_transform: F) -> ScheduleOutput
where
    F: FnMut(&str, i64) -> i64,
{
    let lt_index = LtIndex::new(&input.products);

    // 1. Explode demand into units
    struct U {
        demand_id: String,
        product_id: String,
        due: NaiveDate,
        lt: i64,
        req_start: NaiveDate,
    }
    let mut units: Vec<U> = Vec::new();
    for d in &input.demand {
        let dates = explode_due_dates(d);
        for due in dates {
            let raw_lt = lt_index.lookup(&d.product_id, due).unwrap_or(0);
            let lt = lt_transform(&d.product_id, raw_lt).max(1);
            let req_start = due - Duration::days(lt - 1); // inclusive: lt days = [due-lt+1, due]
            units.push(U {
                demand_id: d.id.clone(),
                product_id: d.product_id.clone(),
                due,
                lt,
                req_start,
            });
        }
    }

    // 2. Sort by required_start ascending (decision 8a)
    units.sort_by(|a, b| a.req_start.cmp(&b.req_start));

    // 3. Greedy place into BayPool
    let mut pool = BayPool::from_factories(&input.factories);
    let mut out_units: Vec<ScheduledUnitOut> = Vec::with_capacity(units.len());
    let mut shipped = 0usize;
    let mut unshippable = 0usize;

    for u in units {
        let placed = pool.find_free(u.req_start, u.due);
        if let Some(idx) = placed {
            pool.reserve(idx, u.req_start, u.due);
            let pb = &pool.bays[idx];
            out_units.push(ScheduledUnitOut {
                demand_id: u.demand_id,
                product_id: u.product_id,
                factory_id: Some(pb.factory_id.clone()),
                bay_index: Some(pb.bay_index),
                required_start: u.req_start,
                due_date: u.due,
                status: UnitStatus::Shipped,
            });
            shipped += 1;
            // suppress unused-warning on lt
            let _ = u.lt;
        } else {
            out_units.push(ScheduledUnitOut {
                demand_id: u.demand_id,
                product_id: u.product_id,
                factory_id: None,
                bay_index: None,
                required_start: u.req_start,
                due_date: u.due,
                status: UnitStatus::Unshippable,
            });
            unshippable += 1;
        }
    }

    ScheduleOutput {
        total_demand: out_units.len(),
        shipped_on_time: shipped,
        unshippable,
        units: out_units,
    }
}

// ---------- Tests ----------

#[cfg(test)]
mod tests {
    use super::*;

    fn ymd(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[test]
    fn quarter_lookup() {
        assert_eq!(quarter_of(ymd(2026, 1, 1)), 1);
        assert_eq!(quarter_of(ymd(2026, 3, 31)), 1);
        assert_eq!(quarter_of(ymd(2026, 4, 1)), 2);
        assert_eq!(quarter_of(ymd(2026, 12, 31)), 4);
    }

    #[test]
    fn period_bounds() {
        assert_eq!(period_start("quarter", 2026, 3), Some(ymd(2026, 7, 1)));
        assert_eq!(period_end("quarter", 2026, 3), Some(ymd(2026, 9, 30)));
        assert_eq!(period_start("month", 2026, 2), Some(ymd(2026, 2, 1)));
        assert_eq!(period_end("month", 2026, 2), Some(ymd(2026, 2, 28)));
    }

    #[test]
    fn explode_even_quarter() {
        let d = DemandInput {
            id: "d1".into(),
            product_id: "p".into(),
            period_type: "quarter".into(),
            year: 2026,
            period_index: 3,
            quantity: 4,
            spread_mode: "even".into(),
        };
        let dates = explode_due_dates(&d);
        assert_eq!(dates.len(), 4);
        // Last unit should land on the period end
        assert_eq!(*dates.last().unwrap(), ymd(2026, 9, 30));
        // Dates should be non-decreasing
        for w in dates.windows(2) {
            assert!(w[0] <= w[1]);
        }
    }

    #[test]
    fn explode_start_end_modes() {
        let d = DemandInput {
            id: "d1".into(),
            product_id: "p".into(),
            period_type: "quarter".into(),
            year: 2026,
            period_index: 1,
            quantity: 3,
            spread_mode: "start".into(),
        };
        assert_eq!(explode_due_dates(&d), vec![ymd(2026, 1, 1); 3]);
        let d_end = DemandInput {
            spread_mode: "end".into(),
            ..d
        };
        assert_eq!(explode_due_dates(&d_end), vec![ymd(2026, 3, 31); 3]);
    }

    fn sample_scenario_one_unit_fits() -> ScheduleInput {
        ScheduleInput {
            factories: vec![FactoryInput {
                id: "f1".into(),
                name: "F1".into(),
                bays: 1,
                bay_counts_by_quarter: vec![],
            }],
            products: vec![ProductInput {
                id: "p1".into(),
                name: "Widget".into(),
                lead_times: vec![LeadTimeInput {
                    year: 2026,
                    quarter: 3,
                    lead_time_days: 5,
                }],
            }],
            demand: vec![DemandInput {
                id: "d1".into(),
                product_id: "p1".into(),
                period_type: "quarter".into(),
                year: 2026,
                period_index: 3,
                quantity: 1,
                spread_mode: "even".into(),
            }],
        }
    }

    #[test]
    fn empty_input_is_ok() {
        let out = run_schedule(&ScheduleInput {
            factories: vec![],
            products: vec![],
            demand: vec![],
        });
        assert_eq!(out.total_demand, 0);
        assert_eq!(out.shipped_on_time, 0);
        assert_eq!(out.unshippable, 0);
    }

    #[test]
    fn single_unit_fits() {
        let out = run_schedule(&sample_scenario_one_unit_fits());
        assert_eq!(out.total_demand, 1);
        assert_eq!(out.shipped_on_time, 1);
        assert_eq!(out.unshippable, 0);
        let u = &out.units[0];
        assert_eq!(u.due_date, ymd(2026, 9, 30));
        assert_eq!(u.required_start, ymd(2026, 9, 26)); // 5-day LT inclusive: 26..30 = 5 days
        assert_eq!(u.factory_id.as_deref(), Some("f1"));
        assert_eq!(u.bay_index, Some(0));
    }

    #[test]
    fn demand_spreads_across_factories() {
        // 2 factories, 2 bays each. 10 units evenly spread over Q3.
        // With load-balanced placement, both factories should get units.
        let s = ScheduleInput {
            factories: vec![
                FactoryInput { id: "fA".into(), name: "A".into(), bays: 2, bay_counts_by_quarter: vec![] },
                FactoryInput { id: "fB".into(), name: "B".into(), bays: 2, bay_counts_by_quarter: vec![] },
            ],
            products: vec![ProductInput {
                id: "p1".into(),
                name: "P".into(),
                lead_times: vec![LeadTimeInput { year: 2026, quarter: 3, lead_time_days: 5 }],
            }],
            demand: vec![DemandInput {
                id: "d1".into(),
                product_id: "p1".into(),
                period_type: "quarter".into(),
                year: 2026,
                period_index: 3,
                quantity: 10,
                spread_mode: "even".into(),
            }],
        };
        let out = run_schedule(&s);
        assert_eq!(out.shipped_on_time, 10);
        let mut count_a: i64 = 0;
        let mut count_b: i64 = 0;
        for u in &out.units {
            match u.factory_id.as_deref() {
                Some("fA") => count_a += 1,
                Some("fB") => count_b += 1,
                _ => {}
            }
        }
        assert!(count_a > 0, "Factory A should receive units, got {count_a}");
        assert!(count_b > 0, "Factory B should receive units, got {count_b}");
        // Should be roughly balanced (each factory carries ~half)
        let diff = (count_a - count_b).abs();
        assert!(
            diff <= 2,
            "factories should be balanced; got A={count_a}, B={count_b}"
        );
    }

    #[test]
    fn demand_spreads_across_bays_within_factory() {
        // Single factory, 4 bays. 8 units evenly spread. Every bay should be used.
        let s = ScheduleInput {
            factories: vec![FactoryInput { id: "f1".into(), name: "F1".into(), bays: 4, bay_counts_by_quarter: vec![] }],
            products: vec![ProductInput {
                id: "p1".into(),
                name: "P".into(),
                lead_times: vec![LeadTimeInput { year: 2026, quarter: 3, lead_time_days: 5 }],
            }],
            demand: vec![DemandInput {
                id: "d1".into(),
                product_id: "p1".into(),
                period_type: "quarter".into(),
                year: 2026,
                period_index: 3,
                quantity: 8,
                spread_mode: "even".into(),
            }],
        };
        let out = run_schedule(&s);
        assert_eq!(out.shipped_on_time, 8);
        let mut bay_counts = [0i64; 4];
        for u in &out.units {
            if let Some(b) = u.bay_index {
                bay_counts[b as usize] += 1;
            }
        }
        // All 4 bays should have at least one unit
        for (i, &c) in bay_counts.iter().enumerate() {
            assert!(c > 0, "bay {i} unused; counts = {bay_counts:?}");
        }
    }

    #[test]
    fn variable_bays_shrinking_quarter_limits_capacity() {
        // Factory has baseline of 4 bays, but Q3 2026 is overridden to 1 bay.
        // 4 units due in Q3 (5-day LT, non-overlapping windows) — with 1 bay
        // available, all 4 still fit sequentially. Add a 5th unit competing for
        // the same exact window and that one becomes unshippable.
        let s = ScheduleInput {
            factories: vec![FactoryInput {
                id: "f1".into(),
                name: "F1".into(),
                bays: 4,
                bay_counts_by_quarter: vec![BayCountInput {
                    year: 2026,
                    quarter: 3,
                    bays: 1,
                }],
            }],
            products: vec![ProductInput {
                id: "p1".into(),
                name: "P".into(),
                lead_times: vec![LeadTimeInput { year: 2026, quarter: 3, lead_time_days: 5 }],
            }],
            demand: vec![DemandInput {
                id: "d1".into(),
                product_id: "p1".into(),
                period_type: "quarter".into(),
                year: 2026,
                period_index: 3,
                quantity: 6,
                spread_mode: "end".into(), // all due same day -> all need same window
            }],
        };
        let out = run_schedule(&s);
        // With all 6 needing the same window and only 1 bay during Q3, only 1 fits
        assert_eq!(out.shipped_on_time, 1, "should be capped by Q3 override of 1 bay");
        assert_eq!(out.unshippable, 5);
    }

    #[test]
    fn variable_bays_increasing_quarter_expands_capacity() {
        // Baseline 1 bay; Q3 2026 overridden to 4 bays. Six units all due Sep 30
        // should now fit (was capped at 1 in the previous test).
        let s = ScheduleInput {
            factories: vec![FactoryInput {
                id: "f1".into(),
                name: "F1".into(),
                bays: 1,
                bay_counts_by_quarter: vec![BayCountInput {
                    year: 2026,
                    quarter: 3,
                    bays: 4,
                }],
            }],
            products: vec![ProductInput {
                id: "p1".into(),
                name: "P".into(),
                lead_times: vec![LeadTimeInput { year: 2026, quarter: 3, lead_time_days: 5 }],
            }],
            demand: vec![DemandInput {
                id: "d1".into(),
                product_id: "p1".into(),
                period_type: "quarter".into(),
                year: 2026,
                period_index: 3,
                quantity: 4,
                spread_mode: "end".into(),
            }],
        };
        let out = run_schedule(&s);
        assert_eq!(out.shipped_on_time, 4);
        assert_eq!(out.unshippable, 0);
    }

    #[test]
    fn variable_bays_window_crossing_quarter_uses_min() {
        // Window crosses Q1->Q2 boundary. Q1 has 1 bay, Q2 has 4 bays.
        // Bays beyond index 0 are unusable in any cross-quarter window because
        // min(1, 4) = 1.
        let s = ScheduleInput {
            factories: vec![FactoryInput {
                id: "f1".into(),
                name: "F1".into(),
                bays: 4,
                bay_counts_by_quarter: vec![
                    BayCountInput { year: 2026, quarter: 1, bays: 1 },
                    BayCountInput { year: 2026, quarter: 2, bays: 4 },
                ],
            }],
            products: vec![ProductInput {
                id: "p1".into(),
                name: "P".into(),
                // 40-day LT for a unit due Apr 10 -> window [Mar 2, Apr 10] crosses Q1/Q2
                lead_times: vec![LeadTimeInput { year: 2026, quarter: 2, lead_time_days: 40 }],
            }],
            demand: vec![DemandInput {
                id: "d1".into(),
                product_id: "p1".into(),
                period_type: "month".into(),
                year: 2026,
                period_index: 4, // April
                quantity: 4,
                spread_mode: "start".into(), // all due Apr 1
            }],
        };
        let out = run_schedule(&s);
        // Cross-quarter windows are gated by min(Q1=1, Q2=4) = 1, so only 1 fits
        assert_eq!(out.shipped_on_time, 1);
        assert_eq!(out.unshippable, 3);
    }

    #[test]
    fn capacity_limited_shortfall() {
        // 1 bay, 5-day LT, but 3 units due same day -> only 1 fits, 2 unshippable
        let mut s = sample_scenario_one_unit_fits();
        s.demand[0].quantity = 3;
        s.demand[0].spread_mode = "end".into();
        let out = run_schedule(&s);
        assert_eq!(out.total_demand, 3);
        // Even with greedy, only one fits because all three have the same exact required window
        assert!(out.shipped_on_time <= 1);
        assert!(out.unshippable >= 2);
    }

    #[test]
    fn cross_quarter_lead_time_uses_due_quarter() {
        // LT differs Q1 vs Q2; due date is April 5 (Q2) -> should use Q2 LT
        let s = ScheduleInput {
            factories: vec![FactoryInput {
                id: "f1".into(),
                name: "F1".into(),
                bays: 1,
                bay_counts_by_quarter: vec![],
            }],
            products: vec![ProductInput {
                id: "p1".into(),
                name: "P".into(),
                lead_times: vec![
                    LeadTimeInput {
                        year: 2026,
                        quarter: 1,
                        lead_time_days: 100,
                    },
                    LeadTimeInput {
                        year: 2026,
                        quarter: 2,
                        lead_time_days: 5,
                    },
                ],
            }],
            demand: vec![DemandInput {
                id: "d1".into(),
                product_id: "p1".into(),
                period_type: "month".into(),
                year: 2026,
                period_index: 4, // April
                quantity: 1,
                spread_mode: "end".into(),
            }],
        };
        let out = run_schedule(&s);
        let u = &out.units[0];
        assert_eq!(u.due_date, ymd(2026, 4, 30));
        // 5-day LT means required_start = Apr 26
        assert_eq!(u.required_start, ymd(2026, 4, 26));
    }

    #[test]
    fn multiple_bays_load_balance() {
        // 2 bays, 4 units of 5-day LT all due Sep 30 -> first 2 placed; remaining 2 unshippable
        let s = ScheduleInput {
            factories: vec![FactoryInput {
                id: "f1".into(),
                name: "F1".into(),
                bays: 2,
                bay_counts_by_quarter: vec![],
            }],
            products: vec![ProductInput {
                id: "p1".into(),
                name: "P".into(),
                lead_times: vec![LeadTimeInput {
                    year: 2026,
                    quarter: 3,
                    lead_time_days: 5,
                }],
            }],
            demand: vec![DemandInput {
                id: "d1".into(),
                product_id: "p1".into(),
                period_type: "quarter".into(),
                year: 2026,
                period_index: 3,
                quantity: 4,
                spread_mode: "end".into(),
            }],
        };
        let out = run_schedule(&s);
        assert_eq!(out.shipped_on_time, 2);
        assert_eq!(out.unshippable, 2);
    }
}
