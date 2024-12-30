use std::fmt::Write;

use axum::{
    extract::Path,
    http::StatusCode,
    response::{IntoResponse, Result},
    routing::{get, post},
    Router,
};
use axum_extra::extract::Multipart;
use html_escape::encode_safe;
use serde::Deserialize;

pub fn router() -> Router {
    Router::new()
        .route("/23/star", get(star))
        .route("/23/present/:color", get(present))
        .route("/23/ornament/:state/:n", get(ornament))
        .route("/23/lockfile", post(lockfile))
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

#[derive(Debug, Deserialize)]
struct Lockfile {
    package: Vec<Package>,
}

#[derive(Debug, Deserialize)]
struct Package {
    checksum: Option<String>,
}

#[axum::debug_handler]
async fn lockfile(mut multipart: Multipart) -> Result<String, StatusCode> {
    let mut res = String::new();
    while let Some(field) = multipart.next_field().await.map_err(|e| {
        eprintln!("Problem with multipart: {e:?}");
        StatusCode::BAD_REQUEST
    })? {
        let name = field.name().ok_or(StatusCode::BAD_REQUEST)?.to_string();
        if name != "lockfile" {
            return Err(StatusCode::BAD_REQUEST);
        }
        let data = field.text().await.map_err(|_| StatusCode::BAD_REQUEST)?;
        eprintln!("Length of `{}` is {} bytes", name, data.len());
        // dbg!(&data);

        let lockfile: Lockfile = toml::from_str(&data).map_err(|e| {
            eprintln!("Problem with lockfile: {e:?}");
            StatusCode::BAD_REQUEST
        })?;
        // dbg!(&lockfile);
        if lockfile.package.is_empty() {
            eprintln!("ERROR: Empty lockfile package");
            return Err(StatusCode::BAD_REQUEST);
        }
        for package in lockfile.package {
            // skip missing package checksum
            let checksum = match package.checksum {
                Some(c) => c,
                None => continue,
            };
            // dbg!(&checksum);
            // first 6 bytes for the color
            let color = checksum.get(0..6).ok_or(StatusCode::UNPROCESSABLE_ENTITY)?;
            let color =
                u32::from_str_radix(color, 16).map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
            // next 2 bytes for top
            let top = checksum.get(6..8).ok_or(StatusCode::UNPROCESSABLE_ENTITY)?;
            let top = u8::from_str_radix(top, 16).map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
            // next 2 bytes for left
            let left = checksum
                .get(8..10)
                .ok_or(StatusCode::UNPROCESSABLE_ENTITY)?;
            let left =
                u8::from_str_radix(left, 16).map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
            // dbg!(&color);
            writeln!(
                res,
                r#"<div style="background-color:#{color:06x};top:{top}px;left:{left}px;"></div>"#
            )
            .unwrap();
        }
    }
    // dbg!(&res);
    Ok(res)
}
