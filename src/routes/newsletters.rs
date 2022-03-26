use crate::domain::SubscriberEmail;
use crate::email_client::EmailClient;
use actix_web::http::StatusCode;
use actix_web::{web, HttpResponse, ResponseError};
use serde::Deserialize;
use sqlx::PgPool;

#[derive(Debug, Deserialize)]
pub struct Content {
    html: String,
    text: String,
}

#[derive(Debug, Deserialize)]
pub struct BodyData {
    title: String,
    content: Content,
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
pub enum NewsletterError {
    #[error("Failed to get confirmed subs")]
    ConfirmedSubsError(#[source] sqlx::Error),
    #[error("Failed to send a newsletter")]
    SendLetterError(#[source] reqwest::Error),
}

impl std::fmt::Debug for NewsletterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for NewsletterError {
    fn status_code(&self) -> actix_http::StatusCode {
        match self {
            NewsletterError::ConfirmedSubsError(_) | NewsletterError::SendLetterError(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        }
    }
}

pub async fn publish_newsletter(
    body: web::Json<BodyData>,
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
) -> Result<HttpResponse, NewsletterError> {
    let subs = get_confirmed_subs(&pool).await?;
    for sub in subs {
        match sub {
            Ok(sub) => email_client
                .send_email(sub, &body.title, &body.content.html, &body.content.text)
                .await
                .map_err(NewsletterError::SendLetterError)?,
            Err(e) => tracing::warn!(error.cause_chain = ?e,
                    "Skipping a confirmed sub bacause of invalid email"),
        }
    }
    Ok(HttpResponse::Ok().finish())
}

async fn get_confirmed_subs(
    pool: &PgPool,
) -> Result<Vec<Result<SubscriberEmail, String>>, NewsletterError> {

    let rows = sqlx::query!(
        "select email from subscriptions where status = 'confirmed'"
    )
    .fetch_all(pool)
    .await
    .map_err(NewsletterError::ConfirmedSubsError)?;

    Ok(rows
        .into_iter()
        .map(|r| SubscriberEmail::try_from(r.email))
        .collect())
}
