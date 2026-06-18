use actix_web::{delete, get, post, put, web, HttpResponse};

use crate::db::{new_id, Pool};
use crate::error::{AppError, AppResult};
use crate::models::*;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(list_demand)
        .service(create_demand)
        .service(update_demand)
        .service(delete_demand);
}

fn validate_demand_inputs(
    period_type: &str,
    period_index: i64,
    quantity: i64,
    spread_mode: &str,
) -> Result<(), AppError> {
    match period_type {
        "month" => {
            if !(1..=12).contains(&period_index) {
                return Err(AppError::BadRequest(format!(
                    "period_index must be 1..=12 for month (got {period_index})"
                )));
            }
        }
        "quarter" => {
            if !(1..=4).contains(&period_index) {
                return Err(AppError::BadRequest(format!(
                    "period_index must be 1..=4 for quarter (got {period_index})"
                )));
            }
        }
        other => {
            return Err(AppError::BadRequest(format!(
                "period_type must be 'month' or 'quarter' (got '{other}')"
            )));
        }
    }
    if quantity <= 0 {
        return Err(AppError::BadRequest("quantity must be > 0".into()));
    }
    if !["even", "start", "end"].contains(&spread_mode) {
        return Err(AppError::BadRequest(format!(
            "spread_mode must be even/start/end (got '{spread_mode}')"
        )));
    }
    Ok(())
}

fn validate_serial_mode(serial_mode: &str) -> Result<(), AppError> {
    if !["none", "sequence", "list"].contains(&serial_mode) {
        return Err(AppError::BadRequest(format!(
            "serial_mode must be none/sequence/list (got '{serial_mode}')"
        )));
    }
    Ok(())
}

#[get("/api/scenarios/{id}/demand")]
async fn list_demand(pool: web::Data<Pool>, path: web::Path<String>) -> AppResult<HttpResponse> {
    let scenario_id = path.into_inner();
    let rows = sqlx::query_as::<_, Demand>(
        "SELECT id, scenario_id, product_id, period_type, year, period_index, quantity, spread_mode, serial_mode, serial_start, serial_list FROM demand WHERE scenario_id = ? ORDER BY year, period_index",
    )
    .bind(&scenario_id)
    .fetch_all(pool.get_ref())
    .await?;
    Ok(HttpResponse::Ok().json(rows))
}

#[post("/api/scenarios/{id}/demand")]
async fn create_demand(
    pool: web::Data<Pool>,
    path: web::Path<String>,
    body: web::Json<CreateDemand>,
) -> AppResult<HttpResponse> {
    let scenario_id = path.into_inner();
    validate_demand_inputs(
        &body.period_type,
        body.period_index,
        body.quantity,
        &body.spread_mode,
    )?;
    validate_serial_mode(&body.serial_mode)?;
    let id = new_id();
    sqlx::query("INSERT INTO demand (id, scenario_id, product_id, period_type, year, period_index, quantity, spread_mode, serial_mode, serial_start, serial_list) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)")
        .bind(&id)
        .bind(&scenario_id)
        .bind(&body.product_id)
        .bind(&body.period_type)
        .bind(body.year)
        .bind(body.period_index)
        .bind(body.quantity)
        .bind(&body.spread_mode)
        .bind(&body.serial_mode)
        .bind(&body.serial_start)
        .bind(&body.serial_list)
        .execute(pool.get_ref())
        .await?;
    let row = sqlx::query_as::<_, Demand>(
        "SELECT id, scenario_id, product_id, period_type, year, period_index, quantity, spread_mode, serial_mode, serial_start, serial_list FROM demand WHERE id = ?",
    )
    .bind(&id)
    .fetch_one(pool.get_ref())
    .await?;
    Ok(HttpResponse::Created().json(row))
}

#[put("/api/demand/{id}")]
async fn update_demand(
    pool: web::Data<Pool>,
    path: web::Path<String>,
    body: web::Json<UpdateDemand>,
) -> AppResult<HttpResponse> {
    let id = path.into_inner();
    validate_demand_inputs(
        &body.period_type,
        body.period_index,
        body.quantity,
        &body.spread_mode,
    )?;
    validate_serial_mode(&body.serial_mode)?;
    let res = sqlx::query("UPDATE demand SET product_id = ?, period_type = ?, year = ?, period_index = ?, quantity = ?, spread_mode = ?, serial_mode = ?, serial_start = ?, serial_list = ? WHERE id = ?")
        .bind(&body.product_id)
        .bind(&body.period_type)
        .bind(body.year)
        .bind(body.period_index)
        .bind(body.quantity)
        .bind(&body.spread_mode)
        .bind(&body.serial_mode)
        .bind(&body.serial_start)
        .bind(&body.serial_list)
        .bind(&id)
        .execute(pool.get_ref())
        .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("demand {id}")));
    }
    let row = sqlx::query_as::<_, Demand>(
        "SELECT id, scenario_id, product_id, period_type, year, period_index, quantity, spread_mode, serial_mode, serial_start, serial_list FROM demand WHERE id = ?",
    )
    .bind(&id)
    .fetch_one(pool.get_ref())
    .await?;
    Ok(HttpResponse::Ok().json(row))
}

#[delete("/api/demand/{id}")]
async fn delete_demand(
    pool: web::Data<Pool>,
    path: web::Path<String>,
) -> AppResult<HttpResponse> {
    let id = path.into_inner();
    let res = sqlx::query("DELETE FROM demand WHERE id = ?")
        .bind(&id)
        .execute(pool.get_ref())
        .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("demand {id}")));
    }
    Ok(HttpResponse::NoContent().finish())
}
