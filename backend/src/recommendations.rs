//! Recommendation engine — see docs/PLAN.md §4 for the three flavors:
//!   1. bays_needed  – min added bays that clears shortfall
//!   2. uniform_lt_pct – min uniform % LT reduction that clears shortfall
//!   3. per_product_lt – per product, min LT that lets all its units fit
//!
//! Phase 2 ships a stub returning empty recs; Phase 3 implements the binary searches.

use serde::{Deserialize, Serialize};

use crate::scheduling::{ScheduleInput, ScheduleOutput};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct RecommendationOut {
    pub bays_needed: Option<BaysNeededRec>,
    pub uniform_lt_pct: Option<UniformLtPctRec>,
    #[serde(default)]
    pub per_product_lt: Vec<PerProductLtTarget>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaysNeededRec {
    pub bays_to_add: i64,
    pub suggested_factory_id: Option<String>,
    pub suggested_factory_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniformLtPctRec {
    /// Reduction in percent, e.g. 12.5 means "reduce all LTs by 12.5%".
    pub reduction_pct: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerProductLtTarget {
    pub product_id: String,
    pub product_name: String,
    pub current_lead_time_days: i64,
    pub target_lead_time_days: i64,
}

/// Phase 2 stub. Phase 3 will replace with real binary-search algorithms.
pub fn compute_recommendations(
    _input: &ScheduleInput,
    _output: &ScheduleOutput,
) -> RecommendationOut {
    RecommendationOut::default()
}
