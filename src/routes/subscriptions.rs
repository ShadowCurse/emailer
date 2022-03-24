use crate::domain::NewSubscriber;
use crate::email_client::EmailClient;
use crate::startup::AppBaseUrl;
use actix_web::{web, HttpResponse};
use chrono::Utc;
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use serde::Deserialize;
use sqlx::{PgPool, Acquire, Executor, Postgres, Transaction};
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
    base_url: web::Data<AppBaseUrl>,
) -> HttpResponse {
    let new_sub = match NewSubscriber::new(form.0.name, form.0.email) {
        Ok(new_sub) => new_sub,
        Err(_) => return HttpResponse::BadRequest().finish(),
    };
    let mut transaction = match connection_pool.begin().await {
        Ok(transaction) => transaction,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };
    let sub_id = match insert_subscriber(&mut transaction, &new_sub).await {
        Ok(sub_id) => sub_id,
        Err(_) => return HttpResponse::InternalServerError().finish(),
    };
    let sub_token = generate_sub_token();

    if store_token(&mut transaction, sub_id, &sub_token)
        .await
        .is_err()
    {
        return HttpResponse::InternalServerError().finish();
    }

    if transaction.commit().await.is_err() {
        return HttpResponse::InternalServerError().finish();
    }

    if send_confirm_email(&email_client, new_sub, &base_url, &sub_token)
        .await
        .is_err()
    {
        return HttpResponse::InternalServerError().finish();
    }

    HttpResponse::Ok().finish()
}

#[tracing::instrument(
    name = "Inserting subscriber data in database",
    skip(connection, new_sub)
)]
pub async fn insert_subscriber(
    connection: &mut Transaction<'_, Postgres>,
    new_sub: &NewSubscriber,
) -> Result<Uuid, sqlx::Error> {
    let sub_id = Uuid::new_v4();
    sqlx::query!(
        r#"
        insert into subscriptions (id, email, name, subscribed_at, status)
        values ($1, $2, $3, $4, 'pending')
        "#,
        sub_id,
        new_sub.email.as_ref(),
        new_sub.name.as_ref(),
        Utc::now()
    )
    .execute(connection)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;
    Ok(sub_id)
}

#[tracing::instrument(
    name = "Stores new token of a new sub",
    skip(connection, sub_id, sub_token)
)]
pub async fn store_token(
    connection: &mut Transaction<'_, Postgres>,
    sub_id: Uuid,
    sub_token: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "insert into subscription_tokens (subscription_token, subscriber_id) values ($1, $2)",
        sub_token,
        sub_id
    )
    .execute(connection)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        e
    })?;
    Ok(())
}

#[tracing::instrument(
    name = "Send a confirmation email to a new sub",
    skip(email_client, new_sub, base_url)
)]
pub async fn send_confirm_email(
    email_client: &EmailClient,
    new_sub: NewSubscriber,
    base_url: &AppBaseUrl,
    token: &str,
) -> Result<(), reqwest::Error> {
    let confirm_link = format!("{}/subscriptions/confirm?sub_token={}", base_url.0, token);
    let text_body = format!("visit {} to confirm", confirm_link);
    let htmp_body = format!("visit <a href=\"{}\">this</a> to confirm", confirm_link);
    email_client
        .send_email(new_sub.email, "subject", &htmp_body, &text_body)
        .await
}

fn generate_sub_token() -> String {
    let mut rng = thread_rng();
    std::iter::repeat_with(|| rng.sample(Alphanumeric))
        .map(char::from)
        .take(25)
        .collect()
}
