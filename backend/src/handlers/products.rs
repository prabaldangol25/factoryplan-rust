use actix_web::{delete, get, post, put, web, HttpResponse};

use crate::db::{new_id, Pool};
use crate::error::{AppError, AppResult};
use crate::models::*;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(list_products)
        .service(create_product)
        .service(update_product)
        .service(delete_product);
}

async fn lead_times_for(pool: &Pool, product_id: &str) -> Result<Vec<LeadTimeRow>, sqlx::Error> {
    sqlx::query_as::<_, LeadTimeRow>(
        "SELECT id, product_id, year, quarter, lead_time_days FROM product_lead_time WHERE product_id = ? ORDER BY year, quarter",
    )
    .bind(product_id)
    .fetch_all(pool)
    .await
}

#[get("/api/scenarios/{id}/products")]
async fn list_products(
    pool: web::Data<Pool>,
    path: web::Path<String>,
) -> AppResult<HttpResponse> {
    let scenario_id = path.into_inner();
    let products = sqlx::query_as::<_, ProductRow>(
        "SELECT id, scenario_id, name FROM product WHERE scenario_id = ? ORDER BY name",
    )
    .bind(&scenario_id)
    .fetch_all(pool.get_ref())
    .await?;

    let mut out: Vec<Product> = Vec::with_capacity(products.len());
    for p in products {
        let lts = lead_times_for(pool.get_ref(), &p.id).await?;
        out.push(Product {
            id: p.id,
            scenario_id: p.scenario_id,
            name: p.name,
            lead_times: lts,
        });
    }
    Ok(HttpResponse::Ok().json(out))
}

fn validate_lead_times(lts: &[LeadTimeInput]) -> Result<(), AppError> {
    for lt in lts {
        if !(1..=4).contains(&lt.quarter) {
            return Err(AppError::BadRequest(format!(
                "quarter must be 1..=4 (got {})",
                lt.quarter
            )));
        }
        if lt.lead_time_days <= 0 {
            return Err(AppError::BadRequest(format!(
                "lead_time_days must be > 0 (got {})",
                lt.lead_time_days
            )));
        }
    }
    Ok(())
}

#[post("/api/scenarios/{id}/products")]
async fn create_product(
    pool: web::Data<Pool>,
    path: web::Path<String>,
    body: web::Json<CreateProduct>,
) -> AppResult<HttpResponse> {
    let scenario_id = path.into_inner();
    let name = body.name.trim();
    if name.is_empty() {
        return Err(AppError::BadRequest("name is required".into()));
    }
    validate_lead_times(&body.lead_times)?;
    let id = new_id();

    let mut tx = pool.begin().await?;
    sqlx::query("INSERT INTO product (id, scenario_id, name) VALUES (?, ?, ?)")
        .bind(&id)
        .bind(&scenario_id)
        .bind(name)
        .execute(&mut *tx)
        .await?;
    for lt in &body.lead_times {
        sqlx::query("INSERT INTO product_lead_time (id, product_id, year, quarter, lead_time_days) VALUES (?, ?, ?, ?, ?)")
            .bind(new_id())
            .bind(&id)
            .bind(lt.year)
            .bind(lt.quarter)
            .bind(lt.lead_time_days)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;

    let p = sqlx::query_as::<_, ProductRow>(
        "SELECT id, scenario_id, name FROM product WHERE id = ?",
    )
    .bind(&id)
    .fetch_one(pool.get_ref())
    .await?;
    let lts = lead_times_for(pool.get_ref(), &p.id).await?;
    Ok(HttpResponse::Created().json(Product {
        id: p.id,
        scenario_id: p.scenario_id,
        name: p.name,
        lead_times: lts,
    }))
}

#[put("/api/products/{id}")]
async fn update_product(
    pool: web::Data<Pool>,
    path: web::Path<String>,
    body: web::Json<UpdateProduct>,
) -> AppResult<HttpResponse> {
    let id = path.into_inner();
    let name = body.name.trim();
    if name.is_empty() {
        return Err(AppError::BadRequest("name is required".into()));
    }
    validate_lead_times(&body.lead_times)?;

    let mut tx = pool.begin().await?;
    let res = sqlx::query("UPDATE product SET name = ? WHERE id = ?")
        .bind(name)
        .bind(&id)
        .execute(&mut *tx)
        .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("product {id}")));
    }

    // Replace lead-time matrix wholesale
    sqlx::query("DELETE FROM product_lead_time WHERE product_id = ?")
        .bind(&id)
        .execute(&mut *tx)
        .await?;
    for lt in &body.lead_times {
        sqlx::query("INSERT INTO product_lead_time (id, product_id, year, quarter, lead_time_days) VALUES (?, ?, ?, ?, ?)")
            .bind(new_id())
            .bind(&id)
            .bind(lt.year)
            .bind(lt.quarter)
            .bind(lt.lead_time_days)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;

    let p = sqlx::query_as::<_, ProductRow>(
        "SELECT id, scenario_id, name FROM product WHERE id = ?",
    )
    .bind(&id)
    .fetch_one(pool.get_ref())
    .await?;
    let lts = lead_times_for(pool.get_ref(), &p.id).await?;
    Ok(HttpResponse::Ok().json(Product {
        id: p.id,
        scenario_id: p.scenario_id,
        name: p.name,
        lead_times: lts,
    }))
}

#[delete("/api/products/{id}")]
async fn delete_product(
    pool: web::Data<Pool>,
    path: web::Path<String>,
) -> AppResult<HttpResponse> {
    let id = path.into_inner();
    let res = sqlx::query("DELETE FROM product WHERE id = ?")
        .bind(&id)
        .execute(pool.get_ref())
        .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::NotFound(format!("product {id}")));
    }
    Ok(HttpResponse::NoContent().finish())
}
