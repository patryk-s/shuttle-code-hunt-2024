use std::{ops::DerefMut, sync::Arc, time::Duration};

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
use tokio::sync::Mutex;

const BUCKET_SIZE: u8 = 5;
const BUCKET_REFILL_SECS: u64 = 1;

type MilkBucket = Arc<Mutex<RateLimiter>>;

fn filled_bucket() -> RateLimiter {
    RateLimiter::builder()
        .initial(BUCKET_SIZE as usize)
        .max(BUCKET_SIZE as usize)
        .interval(Duration::from_secs(BUCKET_REFILL_SECS))
        .build()
}

pub fn router() -> Router {
    let limiter = Arc::new(Mutex::new(filled_bucket()));

    Router::new()
        .route("/9/milk", post(task1))
        .route("/9/refill", post(task4))
        .with_state(limiter.clone())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Unit {
    Liters(f32),
    Litres(f32),
    Gallons(f32),
    Pints(f32),
}

#[axum::debug_handler]
async fn task1(State(bucket): State<MilkBucket>, req: Request) -> impl IntoResponse {
    let got_milk = bucket.lock().await.try_acquire(1);
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

            match payload {
                Unit::Liters(v) => {
                    let gallons = v * 0.264172;
                    return Json(json!({"gallons": gallons})).into_response();
                }
                Unit::Litres(v) => {
                    let pints = v * 1.759754;
                    return Json(json!({"pints": pints})).into_response();
                }
                Unit::Gallons(v) => {
                    let liters = v * 3.785412;
                    return Json(json!({"liters": liters})).into_response();
                }
                Unit::Pints(v) => {
                    let litres = v * 0.5682612;
                    return Json(json!({"litres": litres})).into_response();
                }
            }
        }
    }
    (StatusCode::OK, "Milk withdrawn\n").into_response()
}

#[axum::debug_handler]
async fn task4(State(bucket): State<MilkBucket>) -> impl IntoResponse {
    let mut lock = bucket.lock().await;
    let bucket = lock.deref_mut();
    *bucket = filled_bucket();
    StatusCode::OK
}
