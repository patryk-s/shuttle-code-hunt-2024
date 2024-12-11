use axum::{
    async_trait,
    body::Bytes,
    extract::{FromRequest, Request},
    http::{header::CONTENT_TYPE, StatusCode},
    response::IntoResponse,
    routing::post,
    Router,
};
use cargo_manifest::Manifest;
use std::fmt::Write;

pub fn router() -> Router {
    Router::new().route("/5/manifest", post(task1))
}

#[axum::debug_handler]
async fn task1(ValidPayload(manifest): ValidPayload<Manifest>) -> Result<String, Error> {
    let package = manifest.package.ok_or(Error::NoContent)?;
    let keywords = package
        .keywords
        .ok_or(Error::NoKeyword)?
        .as_local()
        .unwrap();
    if !keywords.contains(&"Christmas 2024".to_string()) {
        return Err(Error::NoKeyword);
    }
    let metadata = package.metadata.ok_or(Error::NoContent)?;
    let orders = metadata.get("orders").ok_or(Error::NoContent)?;
    let mut output = String::new();
    for order in orders.as_array().unwrap() {
        let order = order.as_table().unwrap();
        let Some(item) = order.get("item") else {
            continue;
        };
        let Some(quantity) = order.get("quantity") else {
            continue;
        };
        let Some(quantity) = quantity.as_integer() else {
            continue;
        };
        writeln!(output, "{}: {}", item.as_str().unwrap(), quantity).unwrap();
    }

    if output.is_empty() {
        return Err(Error::NoContent);
    }
    Ok(output.trim().to_string())
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Failed to deserialize the request body")]
    Manifest(#[from] cargo_manifest::Error),
    #[error("Failed to deserialize the request body")]
    SerdeYaml(#[from] serde_yaml::Error),
    #[error("Request body didn't contain valid bytes")]
    StringRejection(#[from] axum::extract::rejection::BytesRejection),
    #[error("Media type not supported")]
    UnsupportedMedia,
    #[error("No orders in request")]
    NoContent,
    #[error("Magic keyword not provided")]
    NoKeyword,
}

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        match self {
            Self::Manifest(e) => {
                eprintln!("ManifestError: {e}");
                (StatusCode::BAD_REQUEST, "Invalid manifest").into_response()
            }
            Self::SerdeYaml(e) => {
                eprintln!("SerdeYamlError: {e}");
                (StatusCode::BAD_REQUEST, "Invalid manifest").into_response()
            }
            Self::StringRejection(error) => error.into_response(),
            Self::UnsupportedMedia => (StatusCode::UNSUPPORTED_MEDIA_TYPE).into_response(),
            Self::NoContent => (StatusCode::NO_CONTENT).into_response(),
            Self::NoKeyword => {
                (StatusCode::BAD_REQUEST, "Magic keyword not provided").into_response()
            }
        }
    }
}

pub struct ValidPayload<T>(pub T);

#[async_trait]
impl<S> FromRequest<S> for ValidPayload<Manifest>
where
    S: Send + Sync,
{
    type Rejection = Error;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let content_type_header = req.headers().get(CONTENT_TYPE);
        let content_type = content_type_header.and_then(|v| v.to_str().ok());
        if let Some(content_type) = content_type {
            if content_type.starts_with("application/toml") {
                let payload = Bytes::from_request(req, state).await?;
                return Ok(Self(cargo_manifest::Manifest::from_slice(&payload)?));
            }
            if content_type.starts_with("application/yaml")
                || content_type.starts_with("application/json")
            {
                let payload = Bytes::from_request(req, state).await?;
                return Ok(Self(serde_yaml::from_slice(&payload)?));
            }
        }
        eprintln!("Unsupported media {:?}", content_type);
        Err(Self::Rejection::UnsupportedMedia)
    }
}
