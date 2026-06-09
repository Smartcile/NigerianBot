//! Authentication & authorization: JWT issuing/verification and an Actix
//! request extractor that enforces a valid `Authorization: Bearer <jwt>` header.
//!
//! Roles (`admin` / `user` / `viewer`) ride in the token for future
//! role-based access control; today the single `API_KEY` logs in as `admin`.

use std::future::{ready, Ready};

use actix_web::error::ErrorUnauthorized;
use actix_web::{dev::Payload, web, FromRequest, HttpRequest};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

use crate::state::AppState;

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
