use axum::{
    http::{header, StatusCode},
    response::IntoResponse,
    routing::get,
    Router,
};
use tower_http::services::ServeDir;

mod day12;
mod day16;
mod day19;
mod day2;
mod day5;
mod day9;

#[shuttle_runtime::main]
async fn main(#[shuttle_shared_db::Postgres] pool: sqlx::PgPool) -> shuttle_axum::ShuttleAxum {
    sqlx::migrate!().run(&pool).await.unwrap();

    let router = Router::new()
        .route("/", get(hello_world))
        .route("/-1/seek", get(found))
        .merge(day2::router())
        .merge(day5::router())
        .merge(day9::router())
        .merge(day12::router())
        .merge(day16::router())
        .merge(day19::router(pool))
        .nest_service("/assets", ServeDir::new("assets"));
    Ok(router.into())
}

async fn hello_world() -> &'static str {
    "Hello, bird!"
}

async fn found() -> impl IntoResponse {
    (
        StatusCode::FOUND,
        [(
            header::LOCATION,
            "https://www.youtube.com/watch?v=9Gc4QTqslN4",
        )],
    )
}
