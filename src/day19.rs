use std::{collections::HashMap, ops::Not, sync::Arc};

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post, put},
    Json, Router,
};
use rand::distributions::{Alphanumeric, DistString};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{
    prelude::FromRow,
    query, query_as, query_scalar,
    types::chrono::{DateTime, Utc},
    PgPool,
};
use tokio::sync::Mutex;
use uuid::Uuid;

#[derive(Clone)]
struct Data {
    db: PgPool,
    pagination: Arc<Mutex<HashMap<String, i64>>>,
}

pub fn router(pool: PgPool) -> Router {
    let data = Data {
        db: pool,
        pagination: Arc::new(Mutex::new(HashMap::new())),
    };

    Router::new()
        .route("/19/reset", post(reset))
        .route("/19/cite/:id", get(get_id))
        .route("/19/list", get(list))
        .route("/19/undo/:id", put(update))
        .route("/19/draft", post(create))
        .route("/19/remove/:id", delete(remove))
        .with_state(data)
}

#[axum::debug_handler]
async fn reset(State(data): State<Data>) -> impl IntoResponse {
    query!(r#"DELETE FROM quotes"#)
        .execute(&data.db)
        .await
        .expect("deleting quotes");
}

#[axum::debug_handler]
async fn create(State(data): State<Data>, Json(draft): Json<Draft>) -> impl IntoResponse {
    let id = Uuid::new_v4();
    match query_as!(
        Quote,
        r#"INSERT INTO quotes (id, author, quote) VALUES ($1, $2, $3) RETURNING id, author, quote, created_at, version"#,
        id,
        draft.author,
        draft.quote,
    )
        .fetch_one(&data.db)
        .await
    {
        Ok(quote) => (StatusCode::CREATED, Json(quote)).into_response(),
        Err(e) => {
         eprintln!("Problem creating draft: {e}");
         StatusCode::NOT_FOUND.into_response()
        },
    }
}

#[axum::debug_handler]
async fn remove(State(data): State<Data>, Path(id): Path<Uuid>) -> impl IntoResponse {
    match query_as!(
        Quote,
        r#"DELETE FROM quotes WHERE id = $1 RETURNING id, author, quote, created_at, version"#,
        id,
    )
    .fetch_one(&data.db)
    .await
    {
        Ok(quote) => (StatusCode::OK, Json(quote)).into_response(),
        Err(e) => {
            eprintln!("Problem creating draft: {e}");
            StatusCode::NOT_FOUND.into_response()
        }
    }
}

#[axum::debug_handler]
async fn update(
    State(data): State<Data>,
    Path(id): Path<Uuid>,
    Json(draft): Json<Draft>,
) -> impl IntoResponse {
    match query_as!(
        Quote,
        r#"UPDATE quotes SET author = $1, quote = $2, version = (SELECT version FROM quotes WHERE id = $3) + 1 WHERE id = $3 RETURNING id, author, quote, created_at, version"#,
        draft.author,
        draft.quote,
        id,
    )
        .fetch_one(&data.db)
        .await
    {
        Ok(quote) => (StatusCode::OK, Json(quote)).into_response(),
        Err(e) => {
         eprintln!("Problem updating draft ({id}): {e}");
         StatusCode::NOT_FOUND.into_response()
        },
    }
}

#[axum::debug_handler]
async fn get_id(State(data): State<Data>, Path(id): Path<Uuid>) -> impl IntoResponse {
    match query_as!(
        Quote,
        r#"SELECT id, author, quote, created_at, version FROM quotes where id = $1"#,
        id,
    )
    .fetch_one(&data.db)
    .await
    {
        Ok(quote) => (StatusCode::OK, Json(quote)).into_response(),
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}

#[axum::debug_handler]
async fn list(State(data): State<Data>, Query(params): Query<ListParams>) -> impl IntoResponse {
    let limit = 3;
    let num_quotes = match query_scalar!(r#"SELECT count(*) FROM quotes"#,)
        .fetch_one(&data.db)
        .await
    {
        Ok(quotes) => quotes.unwrap(),
        Err(_) => return StatusCode::NOT_FOUND.into_response(),
    };

    let mut next_token = params.token.clone();

    let page = if num_quotes > limit {
        match params.token {
            Some(token) => {
                if data.pagination.lock().await.contains_key(&token).not() {
                    return StatusCode::BAD_REQUEST.into_response();
                }
                data.pagination
                    .lock()
                    .await
                    .entry(token)
                    .and_modify(|e| *e += 1)
                    .or_default()
                    .to_owned()
            }
            None => {
                let new_token = Alphanumeric.sample_string(&mut rand::thread_rng(), 16);
                next_token = Some(new_token.clone());
                data.pagination.lock().await.insert(new_token, 1);
                1
            }
        }
    } else {
        1
    };

    let offset = (page - 1) * limit;
    let quotes = match query_as!(
        Quote,
        r#"SELECT id, author, quote, created_at, version FROM quotes ORDER BY created_at LIMIT $1 OFFSET $2"#,
        limit,
        offset,
    )
    .fetch_all(&data.db)
    .await
    {
        Ok(quotes) => quotes,
        Err(_) => return StatusCode::NOT_FOUND.into_response(),
    };

    // at last page
    if page * limit >= num_quotes {
        next_token = None;
    }

    (
        StatusCode::OK,
        Json(json!({"quotes": quotes, "page": page, "next_token": next_token})),
    )
        .into_response()
}

#[derive(Debug, FromRow, Serialize)]
struct Quote {
    id: Uuid,
    author: String,
    quote: String,
    created_at: DateTime<Utc>,
    version: i32,
}

#[derive(Debug, Deserialize)]
struct Draft {
    author: String,
    quote: String,
}

#[derive(Debug, Deserialize)]
struct ListParams {
    token: Option<String>,
}
