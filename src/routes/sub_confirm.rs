use actix_web::{web, HttpResponse};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct Params {
    sub_token: String,
}

#[tracing::instrument(name = "Confirm a pending sub", skip(connection_pool, params))]
pub async fn confirm(
    connection_pool: web::Data<PgPool>,
    params: web::Query<Params>,
) -> HttpResponse {
    let id = match get_sub_id_from_token(&connection_pool, &params.sub_token).await {
        Ok(id) => id,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };
    match id {
        None => HttpResponse::Unauthorized().finish(),
        Some(sub_id) => {
            if confirm_sub(&connection_pool, sub_id).await.is_err() {
                return HttpResponse::InternalServerError().finish();
            }
            HttpResponse::Ok().finish()
        }
    }
}

#[tracing::instrument(name = "Get sub id from token", skip(connection_pool, sub_token))]
pub async fn get_sub_id_from_token(
    connection_pool: &PgPool,
    sub_token: &str,
) -> Result<Option<Uuid>, sqlx::Error> {
    let result = sqlx::query!(
        "select subscriber_id from subscription_tokens where subscription_token = $1",
        sub_token,
    )
    .fetch_optional(connection_pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;
    Ok(result.map(|r| r.subscriber_id))
}

#[tracing::instrument(name = "Confirms a sub", skip(connection_pool, sub_id))]
pub async fn confirm_sub(connection_pool: &PgPool, sub_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "update subscriptions set status = 'confirmed' where id = $1",
        sub_id
    )
    .execute(connection_pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;
    Ok(())
}
