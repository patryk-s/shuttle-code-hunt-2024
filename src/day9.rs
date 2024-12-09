use std::{sync::Arc, time::Duration};

use axum::{
    extract::{Request, State},
    http::{header::CONTENT_TYPE, StatusCode},
    response::IntoResponse,
    routing::post,
    Json, RequestExt, Router,
};
use leaky_bucket::RateLimiter;
use serde::Deserialize;
use serde_json::json;

pub fn router() -> Router {
    let limiter = Arc::new(
        RateLimiter::builder()
            .initial(5)
            .max(5)
            .interval(Duration::from_secs(1))
            .build(),
    );
    Router::new()
        .route("/9/milk", post(task1))
        .with_state(limiter.clone())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Unit {
    Liters(f32),
    Gallons(f32),
}

#[axum::debug_handler]
async fn task1(State(bucket): State<Arc<RateLimiter>>, req: Request) -> impl IntoResponse {
    let got_milk = bucket.try_acquire(1);
    if !got_milk {
        return (StatusCode::TOO_MANY_REQUESTS, "No milk available\n").into_response();
    }

    let content_type_header = req.headers().get(CONTENT_TYPE);
    let content_type = content_type_header.and_then(|v| v.to_str().ok());
    if let Some(content_type) = content_type {
        if content_type.starts_with("application/json") {
            let Json(payload) = match req.extract::<Json<Unit>, _>().await {
                Ok(v) => v,
                Err(e) => return (StatusCode::BAD_REQUEST, e).into_response(),
            };
            // let payload = Bytes::from_request(req, state).await;
            match payload {
                Unit::Liters(v) => {
                    let gallons = v * 0.264172;
                    Json(json!({"gallons": gallons})).into_response()
                }
                Unit::Gallons(v) => {
                    let liters = v * 3.785412;
                    Json(json!({"liters": liters})).into_response()
                }
            }
        } else {
            StatusCode::BAD_REQUEST.into_response()
        }
    } else {
        (StatusCode::OK, "Milk withdrawn\n").into_response()
    }
}
