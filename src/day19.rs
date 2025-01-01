use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{
    prelude::FromRow,
    query, query_as,
    types::chrono::{DateTime, Utc},
    PgPool,
};
use uuid::Uuid;

pub fn router(pool: PgPool) -> Router {
    Router::new()
        .route("/19/reset", post(reset))
        .route("/19/cite/:id", get(get_id))
        .route("/19/list", get(list))
        .route("/19/undo/:id", put(update))
        .route("/19/draft", post(create))
        .route("/19/remove/:id", delete(remove))
        .with_state(pool)
}

#[axum::debug_handler]
async fn reset(State(db): State<PgPool>) -> impl IntoResponse {
    query!(r#"DELETE FROM quotes"#)
        .execute(&db)
        .await
        .expect("deleting quotes");
}

#[axum::debug_handler]
async fn create(State(db): State<PgPool>, Json(draft): Json<Draft>) -> impl IntoResponse {
    let id = Uuid::new_v4();
    match query_as!(
        Quote,
        r#"INSERT INTO quotes (id, author, quote) VALUES ($1, $2, $3) RETURNING id, author, quote, created_at, version"#,
        id,
        draft.author,
        draft.quote,
    )
        .fetch_one(&db)
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
async fn remove(State(db): State<PgPool>, Path(id): Path<Uuid>) -> impl IntoResponse {
    match query_as!(
        Quote,
        r#"DELETE FROM quotes WHERE id = $1 RETURNING id, author, quote, created_at, version"#,
        id,
    )
    .fetch_one(&db)
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
    State(db): State<PgPool>,
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
        .fetch_one(&db)
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
async fn get_id(State(db): State<PgPool>, Path(id): Path<Uuid>) -> impl IntoResponse {
    match query_as!(
        Quote,
        r#"SELECT id, author, quote, created_at, version FROM quotes where id = $1"#,
        id,
    )
    .fetch_one(&db)
    .await
    {
        Ok(quote) => (StatusCode::OK, Json(quote)).into_response(),
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}

#[axum::debug_handler]
async fn list(State(db): State<PgPool>) -> impl IntoResponse {
    let quotes = match query_as!(
        Quote,
        r#"SELECT id, author, quote, created_at, version FROM quotes ORDER BY created_at"#,
    )
    .fetch_all(&db)
    .await
    {
        Ok(quotes) => quotes,
        Err(_) => return StatusCode::NOT_FOUND.into_response(),
    };

    let page = 1;
    let next_token = if quotes.len() > 3 { Some("foo") } else { None };

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
