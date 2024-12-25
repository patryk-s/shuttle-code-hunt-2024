use axum::{extract::Path, http::StatusCode, response::IntoResponse, routing::get, Router};
use html_escape::{encode_safe, encode_text};

pub fn router() -> Router {
    Router::new()
        .route("/23/star", get(star))
        .route("/23/present/:color", get(present))
        .route("/23/ornament/:state/:n", get(ornament))
}

#[axum::debug_handler]
async fn star() -> impl IntoResponse {
    r#"<div id="star" class="lit"></div>"#
}

#[axum::debug_handler]
async fn present(Path(color): Path<String>) -> impl IntoResponse {
    let next_color = match color.as_str() {
        "red" => "blue",
        "blue" => "purple",
        "purple" => "red",
        _ => return StatusCode::IM_A_TEAPOT.into_response(),
    };
    format!(
        r#"<div class="present {color}" hx-get="/23/present/{next_color}" hx-swap="outerHTML">
           <div class="ribbon"></div>
           <div class="ribbon"></div>
           <div class="ribbon"></div>
           <div class="ribbon"></div>
    </div>"#
    )
    .into_response()
}

#[axum::debug_handler]
async fn ornament(Path((state, n)): Path<(String, String)>) -> impl IntoResponse {
    eprintln!("{state} {n}");
    let n = encode_safe(&n);
    let (class, next_state) = match state.as_str() {
        "on" => ("ornament on", "off"),
        "off" => ("ornament", "on"),
        _ => return StatusCode::IM_A_TEAPOT.into_response(),
    };
    format!(
        r#"<div class="{class}" id="ornament{n}" hx-trigger="load delay:2s once" hx-get="/23/ornament/{next_state}/{n}" hx-swap="outerHTML"></div>"#
    )
    .into_response()
}
