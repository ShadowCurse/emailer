use crate::domain::SubscriberEmail;
use crate::email_client::EmailClient;
use actix_http::header::{self, HeaderMap, HeaderValue};
use actix_web::http::StatusCode;
use actix_web::{web, HttpRequest, HttpResponse, ResponseError};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;
use sha3::Digest;

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
    #[error("Failed to send a newsletter")]
    AuthError(String),
}

impl std::fmt::Debug for NewsletterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NewsletterError::AuthError(e) => writeln!(f, "\n{}", e),
            _ => error_chain_fmt(self, f),
        }
    }
}

impl ResponseError for NewsletterError {
    fn error_response(&self) -> HttpResponse<actix_http::body::BoxBody> {
        match self {
            NewsletterError::ConfirmedSubsError(_) | NewsletterError::SendLetterError(_) => {
                HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR)
            }
            NewsletterError::AuthError(_) => {
                let mut response = HttpResponse::new(StatusCode::UNAUTHORIZED);
                let header = HeaderValue::from_str("Basic realm=\"publish\"").unwrap();
                response
                    .headers_mut()
                    .insert(header::WWW_AUTHENTICATE, header);
                response
            }
        }
    }
}

#[tracing::instrument(
    name = "Publish a newsletter issue",
    skip(body, pool, email_client, request),
    fields(username=tracing::field::Empty, user_id=tracing::field::Empty)
)]
pub async fn publish_newsletter(
    body: web::Json<BodyData>,
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
    request: HttpRequest,
) -> Result<HttpResponse, NewsletterError> {
    let credentials = basic_auth(request.headers())?;
    validate_credentials(credentials, &pool).await?;
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

struct Credentials {
    username: String,
    password: String,
}

fn basic_auth(headers: &HeaderMap) -> Result<Credentials, NewsletterError> {
    let header = match headers.get("Authorization") {
        Some(header) => header.to_str().map_err(|e| {
            NewsletterError::AuthError(format!(
                "Authorization header is not a valid UTF-8 string: {}",
                e
            ))
        })?,
        None => {
            return Err(NewsletterError::AuthError(
                "Authorization header is missing".to_string(),
            ))
        }
    };
    let encoded = match header.strip_prefix("Basic ") {
        Some(encoded) => encoded,
        None => {
            return Err(NewsletterError::AuthError(
                "Authorization scheme was not 'Basic'".to_string(),
            ))
        }
    };
    let decoded = base64::decode_config(encoded, base64::STANDARD)
        .map_err(|e| NewsletterError::AuthError(format!("Faild to decode base64: {}", e)))?;
    let decoded_credentials = String::from_utf8(decoded).map_err(|e| {
        NewsletterError::AuthError(format!("Decoded credentials are not valid UTF-8: {}", e))
    })?;

    let mut credentials = decoded_credentials.splitn(2, ':');
    let username = credentials
        .next()
        .ok_or_else(|| NewsletterError::AuthError("Username must be provided".to_string()))?
        .to_string();
    let password = credentials
        .next()
        .ok_or_else(|| NewsletterError::AuthError("Password must be provided".to_string()))?
        .to_string();
    Ok(Credentials { username, password })
}

async fn validate_credentials(credentials: Credentials, pool: &PgPool) -> Result<Uuid, NewsletterError> {
    let password_hash = sha3::Sha3_256::digest(credentials.password.as_bytes());
    let password_hash = format!("{:x}", password_hash);
    let user_id: Option<_> = sqlx::query!(
        "select user_id from users where username = $1 and password_hash = $2",
        credentials.username,
        password_hash
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| NewsletterError::AuthError(format!("Faild to perform query to validate credentials: {}", e)))?;
    user_id
        .map(|row| row.user_id)
        .ok_or_else(|| NewsletterError::AuthError("Invalid username or password".to_string()))
}

async fn get_confirmed_subs(
    pool: &PgPool,
) -> Result<Vec<Result<SubscriberEmail, String>>, NewsletterError> {
    let rows = sqlx::query!("select email from subscriptions where status = 'confirmed'")
        .fetch_all(pool)
        .await
        .map_err(NewsletterError::ConfirmedSubsError)?;

    Ok(rows
        .into_iter()
        .map(|r| SubscriberEmail::try_from(r.email))
        .collect())
}
