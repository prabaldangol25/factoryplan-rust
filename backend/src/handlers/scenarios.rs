use actix_web::{delete, get, post, put, web, HttpResponse};

use crate::db::{new_id, now_iso, Pool};
use crate::error::{AppError, AppResult};
use crate::models::*;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(list_scenarios)
        .service(create_scenario)
        .service(get_scenario)
        .service(update_scenario)
        .service(delete_scenario)
        .service(activate_scenario);
}

#[get("/api/scenarios")]
async fn list_scenarios(pool: web::Data<Pool>) -> AppResult<HttpResponse> {
    let rows = sqlx::query_as::<_, Scenario>(
        "SELECT id, name, created_at, updated_at, is_active FROM scenario ORDER BY created_at DESC",
    )
    .fetch_all(pool.get_ref())
    .await?;
    Ok(HttpResponse::Ok().json(rows))
}

#[post("/api/scenarios")]
async fn create_scenario(
    pool: web::Data<Pool>,
    body: web::Json<CreateScenario>,
) -> AppResult<HttpResponse> {
    let id = new_id();
    let now = now_iso();
    let name = body.name.trim();
    if name.is_empty() {
        return Err(AppError::BadRequest("name is required".into()));
    }

    let mut tx = pool.begin().await?;

    sqlx::query("INSERT INTO scenario (id, name, created_at, updated_at, is_active) VALUES (?, ?, ?, ?, 0)")
        .bind(&id)
        .bind(name)
        .bind(&now)
        .bind(&now)
        .execute(&mut *tx)
        .await?;

    // Clone children if requested
    if let Some(src_id) = &body.clone_from {
        // factories + per-quarter bay-count overrides
        let factories = sqlx::query_as::<_, Factory>(
            "SELECT id, scenario_id, name, bays FROM factory WHERE scenario_id = ?",
        )
        .bind(src_id)
        .fetch_all(&mut *tx)
        .await?;
        for f in factories {
            let new_fid = new_id();
            sqlx::query("INSERT INTO factory (id, scenario_id, name, bays) VALUES (?, ?, ?, ?)")
                .bind(&new_fid)
                .bind(&id)
                .bind(&f.name)
                .bind(f.bays)
                .execute(&mut *tx)
                .await?;
            let bcs = sqlx::query_as::<_, BayCountRow>(
                "SELECT id, factory_id, year, quarter, bays FROM factory_bay_count WHERE factory_id = ?",
            )
            .bind(&f.id)
            .fetch_all(&mut *tx)
            .await?;
            for bc in bcs {
                sqlx::query("INSERT INTO factory_bay_count (id, factory_id, year, quarter, bays) VALUES (?, ?, ?, ?, ?)")
                    .bind(new_id())
                    .bind(&new_fid)
                    .bind(bc.year)
                    .bind(bc.quarter)
                    .bind(bc.bays)
                    .execute(&mut *tx)
                    .await?;
            }
        }

        // products + lead times (need id remap)
        let products = sqlx::query_as::<_, ProductRow>(
            "SELECT id, scenario_id, name FROM product WHERE scenario_id = ?",
        )
        .bind(src_id)
        .fetch_all(&mut *tx)
        .await?;
        let mut product_id_map: std::collections::HashMap<String, String> = Default::default();
        for p in products {
            let new_pid = new_id();
            sqlx::query("INSERT INTO product (id, scenario_id, name) VALUES (?, ?, ?)")
                .bind(&new_pid)
                .bind(&id)
                .bind(&p.name)
                .execute(&mut *tx)
                .await?;
            let lts = sqlx::query_as::<_, LeadTimeRow>(
                "SELECT id, product_id, year, quarter, lead_time_days FROM product_lead_time WHERE product_id = ?",
            )
            .bind(&p.id)
            .fetch_all(&mut *tx)
            .await?;
            for lt in lts {
                sqlx::query("INSERT INTO product_lead_time (id, product_id, year, quarter, lead_time_days) VALUES (?, ?, ?, ?, ?)")
                    .bind(new_id())
                    .bind(&new_pid)
                    .bind(lt.year)
                    .bind(lt.quarter)
                    .bind(lt.lead_time_days)
                    .execute(&mut *tx)
                    .await?;
            }
            product_id_map.insert(p.id, new_pid);
        }

        // demand (remap product_id)
        let demands = sqlx::query_as::<_, Demand>(
            "SELECT id, scenario_id, product_id, period_type, year, period_index, quantity, spread_mode FROM demand WHERE scenario_id = ?",
        )
        .bind(src_id)
        .fetch_all(&mut *tx)
        .await?;
        for d in demands {
            let Some(new_pid) = product_id_map.get(&d.product_id) else { continue };
            sqlx::query("INSERT INTO demand (id, scenario_id, product_id, period_type, year, period_index, quantity, spread_mode) VALUES (?, ?, ?, ?, ?, ?, ?, ?)")
                .bind(new_id())
                .bind(&id)
                .bind(new_pid)
                .bind(&d.period_type)
                .bind(d.year)
                .bind(d.period_index)
                .bind(d.quantity)
                .bind(&d.spread_mode)
                .execute(&mut *tx)
                .await?;
        }
    }

    tx.commit().await?;

    let created = sqlx::query_as::<_, Scenario>(
        "SELECT id, name, created_at, updated_at, is_active FROM scenario WHERE id = ?",
    )
    .bind(&id)
    .fetch_one(pool.get_ref())
    .await?;

    Ok(HttpResponse::Created().json(created))
}

#[get("/api/scenarios/{id}")]
async fn get_scenario(pool: web::Data<Pool>, path: web::Path<String>) -> AppResult<HttpResponse> {
    let id = path.into_inner();
    let row = sqlx::query_as::<_, Scenario>(
        "SELECT id, name, created_at, updated_at, is_active FROM scenario WHERE id = ?",
    )
    .bind(&id)
    .fetch_optional(pool.get_ref())
    .await?;
    match row {
        Some(s) => Ok(HttpResponse::Ok().json(s)),
        None => Err(AppError::NotFound(format!("scenario {id}"))),
    }
}

#[put("/api/scenarios/{id}")]
async fn update_scenario(
    pool: web::Data<Pool>,
    path: web::Path<String>,
    body: web::Json<UpdateScenario>,
) -> AppResult<HttpResponse> {
    let id = path.into_inner();
    let name = body.name.trim();
    if name.is_empty() {
        return Err(AppError::BadRequest("name is required".into()));
    }
    let res = sqlx::query("UPDATE scenario SET name = ?, updated_at = ? WHERE id = ?")
        .bind(name)
        .bind(now_iso())
        .bind(&id)
        .execute(pool.get_ref())
        .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("scenario {id}")));
    }
    let row = sqlx::query_as::<_, Scenario>(
        "SELECT id, name, created_at, updated_at, is_active FROM scenario WHERE id = ?",
    )
    .bind(&id)
    .fetch_one(pool.get_ref())
    .await?;
    Ok(HttpResponse::Ok().json(row))
}

#[delete("/api/scenarios/{id}")]
async fn delete_scenario(pool: web::Data<Pool>, path: web::Path<String>) -> AppResult<HttpResponse> {
    let id = path.into_inner();
    let res = sqlx::query("DELETE FROM scenario WHERE id = ?")
        .bind(&id)
        .execute(pool.get_ref())
        .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("scenario {id}")));
    }
    Ok(HttpResponse::NoContent().finish())
}

#[post("/api/scenarios/{id}/activate")]
async fn activate_scenario(
    pool: web::Data<Pool>,
    path: web::Path<String>,
) -> AppResult<HttpResponse> {
    let id = path.into_inner();
    let mut tx = pool.begin().await?;
    sqlx::query("UPDATE scenario SET is_active = 0")
        .execute(&mut *tx)
        .await?;
    let res = sqlx::query("UPDATE scenario SET is_active = 1, updated_at = ? WHERE id = ?")
        .bind(now_iso())
        .bind(&id)
        .execute(&mut *tx)
        .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("scenario {id}")));
    }
    tx.commit().await?;
    Ok(HttpResponse::NoContent().finish())
}
