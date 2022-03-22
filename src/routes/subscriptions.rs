use crate::domain::NewSubscriber;
use crate::email_client::EmailClient;
use actix_web::{web, HttpResponse};
use chrono::Utc;
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct FormData {
    email: String,
    name: String,
}

#[tracing::instrument(
    name = "Adding a new subscriber",
    skip(form, connection_pool, email_client),
    fields (
        subscriber_email = %form.email,
        subscriber_name = %form.name,
    )
)]
pub async fn subscribe(
    form: web::Form<FormData>,
    connection_pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
) -> HttpResponse {
    let new_sub = match NewSubscriber::new(form.0.name, form.0.email) {
        Ok(new_sub) => new_sub,
        Err(_) => return HttpResponse::BadRequest().finish(),
    };
    if insert_subscriber(&connection_pool, &new_sub).await.is_err() {
        return HttpResponse::InternalServerError().finish();
    }

    if send_confirm_email(&email_client, new_sub).await.is_err() {
        return HttpResponse::InternalServerError().finish();
    }

    HttpResponse::Ok().finish()
}

#[tracing::instrument(
    name = "Inserting subscriber data in database",
    skip(connection_pool, new_sub)
)]
pub async fn insert_subscriber(
    connection_pool: &PgPool,
    new_sub: &NewSubscriber,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        insert into subscriptions (id, email, name, subscribed_at, status)
        values ($1, $2, $3, $4, 'pending')
        "#,
        Uuid::new_v4(),
        new_sub.email.as_ref(),
        new_sub.name.as_ref(),
        Utc::now()
    )
    .execute(connection_pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;
    Ok(())
}

#[tracing::instrument(
    name = "Send a confirmation email to a new sub",
    skip(email_client, new_sub)
)]
pub async fn send_confirm_email(
    email_client: &EmailClient,
    new_sub: NewSubscriber,
) -> Result<(), reqwest::Error> {
    let confirm_link = "https://some-ip.com/subsrcription/confirm";
    let text_body = format!("visit {} to confirm", confirm_link);
    let htmp_body = format!("visit <a href=\"{}\">this</a> to confirm", confirm_link);
    email_client
        .send_email(new_sub.email, "subject", &htmp_body, &text_body)
        .await
}
