use axum::{
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use axum_extra::extract::{cookie::Cookie, CookieJar};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use serde_json::Value;

const KEY: &[u8] = b"super_secret";

pub fn router() -> Router {
    Router::new()
        .route("/16/wrap", post(wrap))
        .route("/16/unwrap", get(unwrap))
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

    // let data: Value = serde_json::from_str(&claim.payload).unwrap();
    Json(claim.payload).into_response()
}

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    payload: Value,
    // exp is required for JWT validation
    exp: u64,
}
