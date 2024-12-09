use axum::{
    http::{header, StatusCode},
    response::IntoResponse,
    routing::get,
    Router,
};

mod day2;
mod day5;
mod day9;

#[shuttle_runtime::main]
async fn main() -> shuttle_axum::ShuttleAxum {
    let router = Router::new()
        .route("/", get(hello_world))
        .route("/-1/seek", get(found))
        .merge(day2::router())
        .merge(day5::router())
        .merge(day9::router());
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
