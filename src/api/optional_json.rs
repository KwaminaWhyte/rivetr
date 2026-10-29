//! A JSON body extractor for handlers where the body is optional.
//!
//! axum 0.8 changed `Option<Json<T>>`: an empty body that still carries a
//! `Content-Type: application/json` header (which the dashboard's fetch wrapper
//! sends on every call) is now a 400 instead of `None`. This extractor restores
//! the lenient behaviour: an empty or whitespace-only body is `None`, a valid
//! JSON body is `Some`, and a malformed body is still a clear 400.

use axum::{
    body::Bytes,
    extract::{FromRequest, Request},
    response::{IntoResponse, Response},
};
use serde::de::DeserializeOwned;

use super::error::ApiError;

pub struct OptionalJson<T>(pub Option<T>);

impl<S, T> FromRequest<S> for OptionalJson<T>
where
    S: Send + Sync,
    T: DeserializeOwned,
{
    type Rejection = Response;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let bytes = Bytes::from_request(req, state)
            .await
            .map_err(IntoResponse::into_response)?;
        if bytes.iter().all(u8::is_ascii_whitespace) {
            return Ok(Self(None));
        }
        serde_json::from_slice(&bytes)
            .map(|value| Self(Some(value)))
            .map_err(|e| {
                ApiError::bad_request(format!("Invalid JSON request body: {e}")).into_response()
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::header::CONTENT_TYPE};
    use serde::Deserialize;

    #[derive(Debug, Deserialize, PartialEq)]
    struct Payload {
        name: String,
    }

    async fn extract(
        content_type: Option<&str>,
        body: &str,
    ) -> Result<Option<Payload>, axum::http::StatusCode> {
        let mut builder = Request::builder().method("POST").uri("/");
        if let Some(ct) = content_type {
            builder = builder.header(CONTENT_TYPE, ct);
        }
        let req = builder.body(Body::from(body.to_string())).unwrap();
        OptionalJson::<Payload>::from_request(req, &())
            .await
            .map(|OptionalJson(v)| v)
            .map_err(|r| r.status())
    }

    #[tokio::test]
    async fn empty_body_without_content_type_is_none() {
        assert_eq!(extract(None, "").await.unwrap(), None);
    }

    #[tokio::test]
    async fn empty_body_with_json_content_type_is_none() {
        assert_eq!(extract(Some("application/json"), "").await.unwrap(), None);
        assert_eq!(
            extract(Some("application/json"), " \n").await.unwrap(),
            None
        );
    }

    #[tokio::test]
    async fn valid_body_is_some_with_or_without_content_type() {
        let want = Some(Payload { name: "a".into() });
        assert_eq!(
            extract(Some("application/json"), r#"{"name":"a"}"#)
                .await
                .unwrap(),
            want
        );
        assert_eq!(extract(None, r#"{"name":"a"}"#).await.unwrap(), want);
    }

    #[tokio::test]
    async fn malformed_body_is_a_bad_request() {
        let err = extract(Some("application/json"), "not json")
            .await
            .unwrap_err();
        assert_eq!(err, axum::http::StatusCode::BAD_REQUEST);
    }
}
