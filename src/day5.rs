use axum::{
    async_trait,
    body::Bytes,
    extract::{FromRequest, Request},
    http::{header::CONTENT_TYPE, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
    Router,
};
use cargo_manifest::Manifest;
use std::fmt::Write;

pub fn router() -> Router {
    Router::new().route("/5/manifest", post(task1))
}

#[axum::debug_handler]
async fn task1(ValidPayload(manifest): ValidPayload<Manifest>) -> Result<String, Response> {
    // dbg!(&toml);
    let Some(package) = manifest.package else {
        return Err(StatusCode::NO_CONTENT.into_response());
    };
    let keywords = package
        .keywords
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "Magic keyword not provided").into_response())?
        .as_local()
        .unwrap();
    if !keywords.contains(&"Christmas 2024".to_string()) {
        return Err((StatusCode::BAD_REQUEST, "Magic keyword not provided").into_response());
    }
    let Some(metadata) = package.metadata else {
        return Err(StatusCode::NO_CONTENT.into_response());
    };
    let Some(orders) = metadata.get("orders") else {
        return Err(StatusCode::NO_CONTENT.into_response());
    };
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
        return Err(StatusCode::NO_CONTENT.into_response());
    }
    Ok(output.trim().to_string())
}

#[derive(Debug, thiserror::Error)]
pub enum TomlRejection {
    #[error("Failed to deserialize the request body")]
    ManifestError(#[from] cargo_manifest::Error),
    #[error("Failed to deserialize the request body")]
    SerdeYamlError(#[from] serde_yaml::Error),
    #[error("Request body didn't contain valid bytes")]
    StringRejection(#[from] axum::extract::rejection::BytesRejection),
    #[error("Media type not supported")]
    UnsupportedMedia,
}

impl IntoResponse for TomlRejection {
    fn into_response(self) -> axum::response::Response {
        match self {
            Self::ManifestError(e) => {
                eprintln!("ManifestError: {e}");
                (StatusCode::BAD_REQUEST, "Invalid manifest").into_response()
            }
            Self::SerdeYamlError(e) => {
                eprintln!("SerdeYamlError: {e}");
                (StatusCode::BAD_REQUEST, "Invalid manifest").into_response()
            }
            Self::StringRejection(error) => error.into_response(),
            Self::UnsupportedMedia => (StatusCode::UNSUPPORTED_MEDIA_TYPE).into_response(),
        }
    }
}

pub struct ValidPayload<T>(pub T);

#[async_trait]
impl<S> FromRequest<S> for ValidPayload<Manifest>
where
    S: Send + Sync,
{
    type Rejection = TomlRejection;

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
