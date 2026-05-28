use serde::{Deserialize, Serialize};

// ---------- Scenario ----------

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Scenario {
    pub id: String,
    pub name: String,
    pub created_at: String,
    pub updated_at: String,
    pub is_active: bool,
}

#[derive(Debug, Deserialize)]
pub struct CreateScenario {
    pub name: String,
    /// Optional id of a scenario to clone (factories/products/lead-times/demand)
    pub clone_from: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateScenario {
    pub name: String,
}

// ---------- Factory ----------

/// Raw `factory` row (used internally and during scenario cloning).
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Factory {
    pub id: String,
    pub scenario_id: String,
    pub name: String,
    pub bays: i64,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct BayCountRow {
    pub id: String,
    pub factory_id: String,
    pub year: i64,
    pub quarter: i64,
    pub bays: i64,
}

/// Composite Factory returned to API clients — base bays + per-quarter overrides.
#[derive(Debug, Clone, Serialize)]
pub struct FactoryWithBayCounts {
    pub id: String,
    pub scenario_id: String,
    pub name: String,
    pub bays: i64,
    pub bay_counts: Vec<BayCountRow>,
}

#[derive(Debug, Deserialize)]
pub struct BayCountInput {
    pub year: i64,
    pub quarter: i64,
    pub bays: i64,
}

#[derive(Debug, Deserialize)]
pub struct CreateFactory {
    pub name: String,
    pub bays: i64,
    #[serde(default)]
    pub bay_counts: Vec<BayCountInput>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateFactory {
    pub name: String,
    pub bays: i64,
    #[serde(default)]
    pub bay_counts: Vec<BayCountInput>,
}

// ---------- Product + lead times ----------

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct ProductRow {
    pub id: String,
    pub scenario_id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct LeadTimeRow {
    pub id: String,
    pub product_id: String,
    pub year: i64,
    pub quarter: i64,
    pub lead_time_days: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Product {
    pub id: String,
    pub scenario_id: String,
    pub name: String,
    pub lead_times: Vec<LeadTimeRow>,
}

#[derive(Debug, Deserialize)]
pub struct LeadTimeInput {
    pub year: i64,
    pub quarter: i64,
    pub lead_time_days: i64,
}

#[derive(Debug, Deserialize)]
pub struct CreateProduct {
    pub name: String,
    #[serde(default)]
    pub lead_times: Vec<LeadTimeInput>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProduct {
    pub name: String,
    /// Full replacement of lead-time matrix
    #[serde(default)]
    pub lead_times: Vec<LeadTimeInput>,
}

// ---------- Demand ----------

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Demand {
    pub id: String,
    pub scenario_id: String,
    pub product_id: String,
    pub period_type: String,   // 'month' | 'quarter'
    pub year: i64,
    pub period_index: i64,
    pub quantity: i64,
    pub spread_mode: String,   // 'even' | 'start' | 'end'
}

#[derive(Debug, Deserialize)]
pub struct CreateDemand {
    pub product_id: String,
    pub period_type: String,
    pub year: i64,
    pub period_index: i64,
    pub quantity: i64,
    #[serde(default = "default_spread")]
    pub spread_mode: String,
}

fn default_spread() -> String {
    "even".to_string()
}

#[derive(Debug, Deserialize)]
pub struct UpdateDemand {
    pub product_id: String,
    pub period_type: String,
    pub year: i64,
    pub period_index: i64,
    pub quantity: i64,
    pub spread_mode: String,
}

// ---------- Schedule run / units / recommendations (used by Phase 2+) ----------

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct ScheduleRun {
    pub id: String,
    pub scenario_id: String,
    pub run_at: String,
    pub total_demand: i64,
    pub shipped_on_time: i64,
    pub unshippable: i64,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct ScheduledUnit {
    pub id: String,
    pub run_id: String,
    pub demand_id: String,
    pub product_id: String,
    pub factory_id: Option<String>,
    pub bay_index: Option<i64>,
    pub required_start: String,
    pub due_date: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct RecommendationRow {
    pub id: String,
    pub run_id: String,
    pub rec_type: String,
    pub payload_json: String,
}
