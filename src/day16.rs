use std::collections::HashSet;

use axum::{
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use axum_extra::extract::{cookie::Cookie, CookieJar};
use jsonwebtoken::{
    decode, decode_header, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

const KEY: &[u8] = b"super_secret";
const PUBKEY_PEM: &[u8] = include_bytes!("../assets/day16_santa_public_key.pem");

pub fn router() -> Router {
    Router::new()
        .route("/16/wrap", post(wrap))
        .route("/16/unwrap", get(unwrap))
        .route("/16/decode", post(decode_handler))
}

#[axum::debug_handler]
async fn wrap(Json(payload): Json<Value>) -> impl IntoResponse {
    let claim = Claims {
        payload,
        exp: 10_000_000_000,
    };
    let header = Header::new(Algorithm::HS512);
    let token = match encode(&header, &claim, &EncodingKey::from_secret(KEY)) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("problem with encoding JWT token: {e:?}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let cookie = Cookie::build(("gift", token)).http_only(true);
    let jar = CookieJar::new().add(cookie);
    (jar, (StatusCode::OK, claim.payload.to_string())).into_response()
}

#[axum::debug_handler]
async fn unwrap(jar: CookieJar) -> impl IntoResponse {
    let Some(cookie) = jar.get("gift") else {
        eprintln!("missing cookie 'gift'");
        return StatusCode::BAD_REQUEST.into_response();
    };
    let token = cookie.value();
    let claim = match decode::<Claims>(
        token,
        &DecodingKey::from_secret(KEY),
        &Validation::new(Algorithm::HS512),
    ) {
        Ok(t) => t.claims,
        Err(e) => {
            eprintln!("problem with decoding JWT token: {e:?}");
            return StatusCode::BAD_REQUEST.into_response();
        }
    };

    Json(claim.payload).into_response()
}

#[axum::debug_handler]
async fn decode_handler(token: String) -> impl IntoResponse {
    let header = match decode_header(&token) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("problem with decoding JWT header: {e:?}");
            return StatusCode::BAD_REQUEST.into_response();
        }
    };

    let Ok(key) = DecodingKey::from_rsa_pem(PUBKEY_PEM) else {
        eprintln!("problem with decoding RSA PEM key");
        return StatusCode::BAD_REQUEST.into_response();
    };

    let mut validation = Validation::new(header.alg);
    // `exp` claim is required by default -- disable that requirement
    validation.required_spec_claims = HashSet::new();
    validation.validate_exp = false;

    let claim = match decode::<Value>(&token, &key, &validation) {
        Ok(t) => t.claims,
        Err(e) if e.kind() == &jsonwebtoken::errors::ErrorKind::InvalidSignature => {
            eprintln!("invalid JWT signature");
            return StatusCode::UNAUTHORIZED.into_response();
        }
        Err(e) => {
            eprintln!("problem with decoding JWT token: {e:?}");
            return StatusCode::BAD_REQUEST.into_response();
        }
    };

    Json(claim).into_response()
}

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    payload: Value,
    // exp is required for JWT validation
    exp: u64,
}
