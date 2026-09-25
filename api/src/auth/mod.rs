//! Authentication & authorization: JWT issuing/verification, the dashboard PIN,
//! and an Actix request extractor that enforces a valid `Authorization: Bearer
//! <jwt>` header.
//!
//! The dashboard signs in with a PIN (bcrypt-hashed in `bot_settings`), seeded to
//! a default on first run and changed on first sign-in. The `API_KEY` can still
//! be used as a credential for scripts/clients.

use std::future::{ready, Ready};

use actix_web::error::ErrorUnauthorized;
use actix_web::{dev::Payload, web, FromRequest, HttpRequest};
use anyhow::{Context, Result};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::state::AppState;

/// `bot_settings` keys for the dashboard PIN.
const PIN_HASH_KEY: &str = "dashboard_pin_hash";
const PIN_DEFAULT_KEY: &str = "dashboard_pin_default";

/// A PIN is 4-8 digits.
pub fn valid_pin(pin: &str) -> bool {
    (4..=8).contains(&pin.len()) && pin.chars().all(|c| c.is_ascii_digit())
}

/// Hash a PIN with bcrypt.
pub fn hash_pin(pin: &str) -> Result<String> {
    bcrypt::hash(pin, bcrypt::DEFAULT_COST).context("failed to hash PIN")
}

/// Verify a PIN against a stored bcrypt hash.
pub fn verify_pin(pin: &str, hash: &str) -> bool {
    bcrypt::verify(pin, hash).unwrap_or(false)
}

/// The stored PIN hash, if one has been set.
pub async fn stored_pin_hash(db: &PgPool) -> Result<Option<String>> {
    let row: Option<(String,)> = sqlx::query_as("SELECT value FROM bot_settings WHERE key = $1")
        .bind(PIN_HASH_KEY)
        .fetch_optional(db)
        .await?;
    Ok(row.map(|(v,)| v))
}

/// Whether the PIN is still the seeded default (i.e. must be changed).
pub async fn pin_is_default(db: &PgPool) -> bool {
    sqlx::query_as::<_, (String,)>("SELECT value FROM bot_settings WHERE key = $1")
        .bind(PIN_DEFAULT_KEY)
        .fetch_optional(db)
        .await
        .ok()
        .flatten()
        .map(|(v,)| v == "true")
        .unwrap_or(true)
}

/// Set (and persist) a new PIN.
pub async fn set_pin(db: &PgPool, pin: &str, is_default: bool) -> Result<()> {
    let hash = hash_pin(pin)?;
    set_setting(db, PIN_HASH_KEY, &hash).await?;
    set_setting(
        db,
        PIN_DEFAULT_KEY,
        if is_default { "true" } else { "false" },
    )
    .await?;
    Ok(())
}

async fn set_setting(db: &PgPool, key: &str, value: &str) -> Result<()> {
    sqlx::query(
        "INSERT INTO bot_settings (key, value, updated_at) VALUES ($1, $2, now()) \
         ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value, updated_at = now()",
    )
    .bind(key)
    .bind(value)
    .execute(db)
    .await?;
    Ok(())
}

/// Seed the default PIN on first run (no-op if a PIN already exists).
pub async fn ensure_pin_initialized(db: &PgPool, default_pin: &str) -> Result<()> {
    if stored_pin_hash(db).await?.is_none() {
        set_pin(db, default_pin, true).await?;
        tracing::info!("dashboard PIN seeded to the default — it must be changed on first sign-in");
    }
    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    /// Subject — who the token was issued to.
    pub sub: String,
    /// Role for access control.
    pub role: String,
    /// Issued-at (unix seconds).
    pub iat: usize,
    /// Expiry (unix seconds).
    pub exp: usize,
}

/// Issue a signed JWT valid for `ttl_secs` seconds.
pub fn issue_token(
    secret: &str,
    sub: &str,
    role: &str,
    ttl_secs: i64,
) -> Result<String, jsonwebtoken::errors::Error> {
    let now = chrono::Utc::now().timestamp();
    let claims = Claims {
        sub: sub.to_string(),
        role: role.to_string(),
        iat: now as usize,
        exp: (now + ttl_secs) as usize,
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
}

/// Verify a JWT and return its claims (also enforces expiry).
pub fn verify_token(secret: &str, token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    let data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::new(Algorithm::HS256),
    )?;
    Ok(data.claims)
}

/// Extractor that requires a valid bearer token. Add it as a handler argument to
/// protect an endpoint; extraction fails with 401 when the token is missing or
/// invalid.
pub struct AuthUser {
    pub claims: Claims,
}

impl FromRequest for AuthUser {
    type Error = actix_web::Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let state = req.app_data::<web::Data<AppState>>().cloned();

        let result = (|| {
            let state = state.ok_or_else(|| ErrorUnauthorized("server misconfigured"))?;
            let header = req
                .headers()
                .get("Authorization")
                .and_then(|h| h.to_str().ok())
                .ok_or_else(|| ErrorUnauthorized("missing Authorization header"))?;
            let token = header
                .strip_prefix("Bearer ")
                .ok_or_else(|| ErrorUnauthorized("expected a Bearer token"))?
                .trim();
            let claims = verify_token(&state.config.jwt_secret, token)
                .map_err(|_| ErrorUnauthorized("invalid or expired token"))?;
            Ok(AuthUser { claims })
        })();

        ready(result)
    }
}
