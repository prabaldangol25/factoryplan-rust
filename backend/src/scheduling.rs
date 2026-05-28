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

// ---------- Bay pool ----------

/// A single bay tracks its reserved intervals (sorted, non-overlapping).
#[derive(Debug, Default, Clone)]
pub struct Bay {
    /// Sorted by start. Each interval is [start, end] inclusive.
    intervals: Vec<(NaiveDate, NaiveDate)>,
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
        // insert sorted by start
        let pos = self
            .intervals
            .binary_search_by(|(s, _)| s.cmp(&start))
            .unwrap_or_else(|p| p);
        self.intervals.insert(pos, (start, end));
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
}

impl BayPool {
    pub fn from_factories(factories: &[FactoryInput]) -> Self {
        let mut bays = Vec::new();
        for f in factories {
            for i in 0..f.bays.max(0) {
                bays.push(PoolBay {
                    factory_id: f.id.clone(),
                    factory_name: f.name.clone(),
                    bay_index: i,
                    bay: Bay::default(),
                });
            }
        }
        BayPool { bays }
    }

    /// Find first bay (by pool order) free in [start, end]. Returns index into `bays`.
    pub fn find_free(&self, start: NaiveDate, end: NaiveDate) -> Option<usize> {
        for (i, b) in self.bays.iter().enumerate() {
            if b.bay.is_free(start, end) {
                return Some(i);
            }
        }
        None
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
