use actix_web::{delete, get, post, put, web, HttpResponse};

use crate::db::{new_id, Pool};
use crate::error::{AppError, AppResult};
use crate::models::*;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(list_factories)
        .service(create_factory)
        .service(update_factory)
        .service(delete_factory);
}

#[get("/api/scenarios/{id}/factories")]
async fn list_factories(
    pool: web::Data<Pool>,
    path: web::Path<String>,
) -> AppResult<HttpResponse> {
    let scenario_id = path.into_inner();
    let rows = sqlx::query_as::<_, Factory>(
        "SELECT id, scenario_id, name, bays FROM factory WHERE scenario_id = ? ORDER BY name",
    )
    .bind(&scenario_id)
    .fetch_all(pool.get_ref())
    .await?;
    Ok(HttpResponse::Ok().json(rows))
}

#[post("/api/scenarios/{id}/factories")]
async fn create_factory(
    pool: web::Data<Pool>,
    path: web::Path<String>,
    body: web::Json<CreateFactory>,
) -> AppResult<HttpResponse> {
    let scenario_id = path.into_inner();
    let name = body.name.trim();
    if name.is_empty() {
        return Err(AppError::BadRequest("name is required".into()));
    }
    if body.bays < 0 {
        return Err(AppError::BadRequest("bays must be >= 0".into()));
    }
    let id = new_id();
    sqlx::query("INSERT INTO factory (id, scenario_id, name, bays) VALUES (?, ?, ?, ?)")
        .bind(&id)
        .bind(&scenario_id)
        .bind(name)
        .bind(body.bays)
        .execute(pool.get_ref())
        .await?;
    let row = sqlx::query_as::<_, Factory>(
        "SELECT id, scenario_id, name, bays FROM factory WHERE id = ?",
    )
    .bind(&id)
    .fetch_one(pool.get_ref())
    .await?;
    Ok(HttpResponse::Created().json(row))
}

#[put("/api/factories/{id}")]
async fn update_factory(
    pool: web::Data<Pool>,
    path: web::Path<String>,
    body: web::Json<UpdateFactory>,
) -> AppResult<HttpResponse> {
    let id = path.into_inner();
    let name = body.name.trim();
    if name.is_empty() {
        return Err(AppError::BadRequest("name is required".into()));
    }
    if body.bays < 0 {
        return Err(AppError::BadRequest("bays must be >= 0".into()));
    }
    let res = sqlx::query("UPDATE factory SET name = ?, bays = ? WHERE id = ?")
        .bind(name)
        .bind(body.bays)
        .bind(&id)
        .execute(pool.get_ref())
        .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("factory {id}")));
    }
    let row = sqlx::query_as::<_, Factory>(
        "SELECT id, scenario_id, name, bays FROM factory WHERE id = ?",
    )
    .bind(&id)
    .fetch_one(pool.get_ref())
    .await?;
    Ok(HttpResponse::Ok().json(row))
}

#[delete("/api/factories/{id}")]
async fn delete_factory(
    pool: web::Data<Pool>,
    path: web::Path<String>,
) -> AppResult<HttpResponse> {
    let id = path.into_inner();
    let res = sqlx::query("DELETE FROM factory WHERE id = ?")
        .bind(&id)
        .execute(pool.get_ref())
        .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("factory {id}")));
    }
    Ok(HttpResponse::NoContent().finish())
}
