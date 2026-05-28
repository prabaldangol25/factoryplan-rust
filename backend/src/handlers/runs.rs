use actix_web::{get, post, web, HttpResponse};
use serde::Serialize;

use crate::db::{new_id, now_iso, Pool};
use crate::error::{AppError, AppResult};
use crate::models::*;
use crate::recommendations::{compute_recommendations, RecommendationOut};
use crate::scheduling::{
    run_schedule, DemandInput, FactoryInput, LeadTimeInput, ProductInput, ScheduleInput,
};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(run_scenario).service(get_run);
}

#[derive(Serialize)]
pub struct RunResponse {
    pub run: ScheduleRun,
    pub units: Vec<ScheduledUnit>,
    pub recommendation: RecommendationOut,
}

pub(crate) async fn load_schedule_input(
    pool: &Pool,
    scenario_id: &str,
) -> AppResult<ScheduleInput> {
    // factories
    let factories = sqlx::query_as::<_, Factory>(
        "SELECT id, scenario_id, name, bays FROM factory WHERE scenario_id = ? ORDER BY name",
    )
    .bind(scenario_id)
    .fetch_all(pool)
    .await?;

    // products + lead times
    let products_rows = sqlx::query_as::<_, ProductRow>(
        "SELECT id, scenario_id, name FROM product WHERE scenario_id = ? ORDER BY name",
    )
    .bind(scenario_id)
    .fetch_all(pool)
    .await?;
    let mut products: Vec<ProductInput> = Vec::with_capacity(products_rows.len());
    for p in products_rows {
        let lts = sqlx::query_as::<_, LeadTimeRow>(
            "SELECT id, product_id, year, quarter, lead_time_days FROM product_lead_time WHERE product_id = ?",
        )
        .bind(&p.id)
        .fetch_all(pool)
        .await?;
        products.push(ProductInput {
            id: p.id,
            name: p.name,
            lead_times: lts
                .into_iter()
                .map(|l| LeadTimeInput {
                    year: l.year,
                    quarter: l.quarter,
                    lead_time_days: l.lead_time_days,
                })
                .collect(),
        });
    }

    // demand
    let demand_rows = sqlx::query_as::<_, Demand>(
        "SELECT id, scenario_id, product_id, period_type, year, period_index, quantity, spread_mode FROM demand WHERE scenario_id = ? ORDER BY year, period_index",
    )
    .bind(scenario_id)
    .fetch_all(pool)
    .await?;

    Ok(ScheduleInput {
        factories: factories
            .into_iter()
            .map(|f| FactoryInput {
                id: f.id,
                name: f.name,
                bays: f.bays,
            })
            .collect(),
        products,
        demand: demand_rows
            .into_iter()
            .map(|d| DemandInput {
                id: d.id,
                product_id: d.product_id,
                period_type: d.period_type,
                year: d.year,
                period_index: d.period_index,
                quantity: d.quantity,
                spread_mode: d.spread_mode,
            })
            .collect(),
    })
}

#[post("/api/scenarios/{id}/run")]
async fn run_scenario(
    pool: web::Data<Pool>,
    path: web::Path<String>,
) -> AppResult<HttpResponse> {
    let scenario_id = path.into_inner();

    // Verify scenario exists
    let exists: Option<(String,)> =
        sqlx::query_as("SELECT id FROM scenario WHERE id = ?")
            .bind(&scenario_id)
            .fetch_optional(pool.get_ref())
            .await?;
    if exists.is_none() {
        return Err(AppError::NotFound(format!("scenario {scenario_id}")));
    }

    let input = load_schedule_input(pool.get_ref(), &scenario_id).await?;
    let output = run_schedule(&input);

    // Compute recommendations only if there's a shortfall
    let recommendation = if output.unshippable > 0 {
        compute_recommendations(&input, &output)
    } else {
        RecommendationOut::default()
    };

    // Persist
    let run_id = new_id();
    let run_at = now_iso();
    let mut tx = pool.begin().await?;
    sqlx::query("INSERT INTO schedule_run (id, scenario_id, run_at, total_demand, shipped_on_time, unshippable) VALUES (?, ?, ?, ?, ?, ?)")
        .bind(&run_id)
        .bind(&scenario_id)
        .bind(&run_at)
        .bind(output.total_demand as i64)
        .bind(output.shipped_on_time as i64)
        .bind(output.unshippable as i64)
        .execute(&mut *tx)
        .await?;

    for u in &output.units {
        sqlx::query("INSERT INTO scheduled_unit (id, run_id, demand_id, product_id, factory_id, bay_index, required_start, due_date, status) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)")
            .bind(new_id())
            .bind(&run_id)
            .bind(&u.demand_id)
            .bind(&u.product_id)
            .bind(u.factory_id.as_deref())
            .bind(u.bay_index)
            .bind(u.required_start.to_string())
            .bind(u.due_date.to_string())
            .bind(u.status.as_str())
            .execute(&mut *tx)
            .await?;
    }

    // Persist recommendations (one row per type, payload JSON)
    persist_rec(&mut tx, &run_id, "bays_needed", &recommendation.bays_needed).await?;
    persist_rec(&mut tx, &run_id, "uniform_lt_pct", &recommendation.uniform_lt_pct).await?;
    persist_rec(&mut tx, &run_id, "per_product_lt", &recommendation.per_product_lt).await?;

    tx.commit().await?;

    let run = ScheduleRun {
        id: run_id.clone(),
        scenario_id: scenario_id.clone(),
        run_at,
        total_demand: output.total_demand as i64,
        shipped_on_time: output.shipped_on_time as i64,
        unshippable: output.unshippable as i64,
    };

    let units = load_units(pool.get_ref(), &run_id).await?;

    Ok(HttpResponse::Ok().json(RunResponse {
        run,
        units,
        recommendation,
    }))
}

#[get("/api/runs/{id}")]
async fn get_run(pool: web::Data<Pool>, path: web::Path<String>) -> AppResult<HttpResponse> {
    let run_id = path.into_inner();
    let run = sqlx::query_as::<_, ScheduleRun>(
        "SELECT id, scenario_id, run_at, total_demand, shipped_on_time, unshippable FROM schedule_run WHERE id = ?",
    )
    .bind(&run_id)
    .fetch_optional(pool.get_ref())
    .await?
    .ok_or_else(|| AppError::NotFound(format!("run {run_id}")))?;

    let units = load_units(pool.get_ref(), &run_id).await?;

    let recs = sqlx::query_as::<_, RecommendationRow>(
        "SELECT id, run_id, rec_type, payload_json FROM recommendation WHERE run_id = ?",
    )
    .bind(&run_id)
    .fetch_all(pool.get_ref())
    .await?;

    let mut recommendation = RecommendationOut::default();
    for r in recs {
        match r.rec_type.as_str() {
            "bays_needed" => {
                recommendation.bays_needed = serde_json::from_str(&r.payload_json).ok().flatten();
            }
            "uniform_lt_pct" => {
                recommendation.uniform_lt_pct =
                    serde_json::from_str(&r.payload_json).ok().flatten();
            }
            "per_product_lt" => {
                recommendation.per_product_lt =
                    serde_json::from_str(&r.payload_json).ok().unwrap_or_default();
            }
            _ => {}
        }
    }

    Ok(HttpResponse::Ok().json(RunResponse {
        run,
        units,
        recommendation,
    }))
}

async fn load_units(pool: &Pool, run_id: &str) -> AppResult<Vec<ScheduledUnit>> {
    let units = sqlx::query_as::<_, ScheduledUnit>(
        "SELECT id, run_id, demand_id, product_id, factory_id, bay_index, required_start, due_date, status FROM scheduled_unit WHERE run_id = ? ORDER BY due_date",
    )
    .bind(run_id)
    .fetch_all(pool)
    .await?;
    Ok(units)
}

async fn persist_rec<T: serde::Serialize>(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    run_id: &str,
    rec_type: &str,
    payload: &T,
) -> AppResult<()> {
    let json = serde_json::to_string(payload)
        .map_err(|e| AppError::Internal(format!("serialize {rec_type}: {e}")))?;
    sqlx::query("INSERT INTO recommendation (id, run_id, rec_type, payload_json) VALUES (?, ?, ?, ?)")
        .bind(new_id())
        .bind(run_id)
        .bind(rec_type)
        .bind(&json)
        .execute(&mut **tx)
        .await?;
    Ok(())
}


