use std::net::{Ipv4Addr, Ipv6Addr};

use axum::{extract::Query, response::IntoResponse, routing::get, Router};
use serde::Deserialize;

pub fn router() -> Router {
    Router::new()
        .route("/2/dest", get(task1))
        .route("/2/key", get(task2))
        .route("/2/v6/dest", get(task3))
        .route("/2/v6/key", get(task4))
}

#[derive(Deserialize)]
struct DestParams {
    from: Ipv4Addr,
    key: Ipv4Addr,
}

#[axum::debug_handler]
async fn task1(Query(params): Query<DestParams>) -> impl IntoResponse {
    let from = params.from.octets();
    let key = params.key.octets();
    let dest: Vec<u8> = from
        .iter()
        .zip(key)
        .map(|(a, b)| a.wrapping_add(b))
        .collect();
    let dest: [u8; 4] = dest.try_into().unwrap();
    let dest = Ipv4Addr::from(dest);
    format!("{dest}").into_response()
}

#[derive(Deserialize)]
struct KeyParams {
    from: Ipv4Addr,
    to: Ipv4Addr,
}

#[axum::debug_handler]
async fn task2(Query(params): Query<KeyParams>) -> impl IntoResponse {
    let from = params.from.octets();
    let to = params.to.octets();
    let key: Vec<u8> = to
        .iter()
        .zip(from)
        .map(|(a, b)| a.wrapping_sub(b))
        .collect();
    let key: [u8; 4] = key.try_into().unwrap();
    let key = Ipv4Addr::from(key);
    format!("{key}").into_response()
}

#[derive(Deserialize)]
struct DestParams6 {
    from: Ipv6Addr,
    key: Ipv6Addr,
}

#[axum::debug_handler]
async fn task3(Query(params): Query<DestParams6>) -> impl IntoResponse {
    let dest = xor_ipv6(params.from, params.key);
    format!("{dest}").into_response()
}

#[derive(Deserialize)]
struct KeyParams6 {
    from: Ipv6Addr,
    to: Ipv6Addr,
}

#[axum::debug_handler]
async fn task4(Query(params): Query<KeyParams6>) -> impl IntoResponse {
    let key = xor_ipv6(params.from, params.to);
    format!("{key}").into_response()
}

fn xor_ipv6(lhs: Ipv6Addr, rhs: Ipv6Addr) -> Ipv6Addr {
    let lhs = lhs.octets();
    let rhs = rhs.octets();
    let res: Vec<u8> = lhs.iter().zip(rhs).map(|(a, b)| a ^ b).collect();
    let res: [u8; 16] = res.try_into().unwrap();
    res.into()
}
