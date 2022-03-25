use crate::domain::NewSubscriber;
use crate::email_client::EmailClient;
use crate::startup::AppBaseUrl;
use actix_http::StatusCode;
use actix_web::{web, HttpResponse, ResponseError};
use chrono::Utc;
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use serde::Deserialize;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

#[derive(Deserialize)]
pub struct FormData {
    email: String,
    name: String,
}

impl TryFrom<FormData> for NewSubscriber {
    type Error = String;

    fn try_from(value: FormData) -> Result<Self, Self::Error> {
        NewSubscriber::new(value.name, value.email)
    }
}

fn error_chain_fmt(
    e: &impl std::error::Error,
    f: &mut std::fmt::Formatter<'_>,
) -> std::fmt::Result {
    writeln!(f, "\n{}", e)?;
    let mut current = e.source();
    while let Some(cause) = current {
        writeln!(f, "caused by:\t{}", cause)?;
        current = cause.source();
    }
    Ok(())
}

#[derive(thiserror::Error)]
pub enum SubscribeError {
    #[error("{0}")]
    ValidationError(String),
    #[error("Faild to acquire a Postgres connection from the pool")]
    PoolError(#[source] sqlx::Error),
    #[error("Faild to insert new sub")]
    InsertSubError(#[source] sqlx::Error),
    #[error("Faild to store sub token")]
    StoreTokenError(#[source] sqlx::Error),
    #[error("Faild to commit sql transaction")]
    TransactionCommitError(#[source] sqlx::Error),
    #[error("Faild to send a confirmation email")]
    SendEmailError(#[source] reqwest::Error),
}

impl std::fmt::Debug for SubscribeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for SubscribeError {
    fn status_code(&self) -> actix_http::StatusCode {
        match self {
            SubscribeError::ValidationError(_) => StatusCode::BAD_REQUEST,
            SubscribeError::PoolError(_)
            | SubscribeError::InsertSubError(_)
            | SubscribeError::StoreTokenError(_)
            | SubscribeError::TransactionCommitError(_)
            | SubscribeError::SendEmailError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
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
) -> Result<HttpResponse, SubscribeError> {
    let new_sub = NewSubscriber::try_from(form.0).map_err(SubscribeError::ValidationError)?;
    let mut transaction = connection_pool
        .begin()
        .await
        .map_err(SubscribeError::PoolError)?;

    let sub_id = insert_subscriber(&mut transaction, &new_sub).await?;
    let sub_token = generate_sub_token();

    store_token(&mut transaction, sub_id, &sub_token).await?;

    transaction
        .commit()
        .await
        .map_err(SubscribeError::TransactionCommitError)?;

    send_confirm_email(&email_client, new_sub, &base_url, &sub_token).await?;

    Ok(HttpResponse::Ok().finish())
}

#[tracing::instrument(
    name = "Inserting subscriber data in database",
    skip(connection, new_sub)
)]
pub async fn insert_subscriber(
    connection: &mut Transaction<'_, Postgres>,
    new_sub: &NewSubscriber,
) -> Result<Uuid, SubscribeError> {
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
        SubscribeError::InsertSubError(e)
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
) -> Result<(), SubscribeError> {
    sqlx::query!(
        "insert into subscription_tokens (subscription_token, subscriber_id) values ($1, $2)",
        sub_token,
        sub_id
    )
    .execute(connection)
    .await
    .map_err(|e| {
        tracing::error!("Failed to execute query: {:?}", e);
        SubscribeError::StoreTokenError(e)
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
) -> Result<(), SubscribeError> {
    let confirm_link = format!("{}/subscriptions/confirm?sub_token={}", base_url.0, token);
    let text_body = format!("visit {} to confirm", confirm_link);
    let htmp_body = format!("visit <a href=\"{}\">this</a> to confirm", confirm_link);
    email_client
        .send_email(new_sub.email, "subject", &htmp_body, &text_body)
        .await
        .map_err(SubscribeError::SendEmailError)?;
    Ok(())
}

fn generate_sub_token() -> String {
    let mut rng = thread_rng();
    std::iter::repeat_with(|| rng.sample(Alphanumeric))
        .map(char::from)
        .take(25)
        .collect()
}
