use std::sync::Arc;

use anyhow::anyhow;
use axum::{
    extract::{Json, State},
    response::Result,
    Extension,
};
use ory_kratos_client::apis::metadata_api::is_ready;
use tokio::sync::RwLock;
use tower_http::request_id::RequestId;
use tracing::info;

use crate::{
    config::IamConfig,
    http::{controler::kratos, error::RouterError},
    permission::Input,
};

///http route to add an identity field
pub async fn add(
    State(config): State<Arc<RwLock<IamConfig>>>,
    request_id: Extension<RequestId>,
    Json(payload): Json<Input>,
) -> Result<&'static str, RouterError> {
    let uuid = request_id.header_value().to_str()?;

    info!("{uuid}: adding data to identity");
    let config = config.read().await;
    let client = match &config.kratos.client {
        Some(client) => client,
        None => Err(anyhow!("{uuid}: Kratos client not initialized"))?,
    };
    kratos(client, uuid, payload, "add").await?;
    info!("{uuid}: done");
    Ok("200")
}

///http route to remove an identity field
pub async fn remove(
    State(config): State<Arc<RwLock<IamConfig>>>,
    request_id: Extension<RequestId>,
    Json(payload): Json<Input>,
) -> Result<&'static str, RouterError> {
    let uuid = request_id.header_value().to_str()?;
    info!("{uuid}: removing data to identity");
    let config = config.read().await;
    let client = match &config.kratos.client {
        Some(client) => client,
        None => Err(anyhow!("{uuid}: Kratos client not initialized"))?,
    };
    kratos(client, uuid, payload, "remove").await?;
    info!("{uuid}: done");
    Ok("200")
}

///http route to replace an identity field
pub async fn replace(
    State(config): State<Arc<RwLock<IamConfig>>>,
    request_id: Extension<RequestId>,
    Json(payload): Json<Input>,
) -> Result<&'static str, RouterError> {
    let uuid = request_id.header_value().to_str()?;
    info!("{uuid}: replacing data in identity");
    let config = config.read().await;
    let client = match &config.kratos.client {
        Some(client) => client,
        None => Err(anyhow!("{uuid}: Kratos client not initialized"))?,
    };
    kratos(client, uuid, payload, "replace").await?;

    info!("{uuid}: done");
    Ok("200")
}

pub async fn alive() -> Result<&'static str, RouterError> {
    Ok("200")
}

pub async fn ready(
    State(config): State<Arc<RwLock<IamConfig>>>,
) -> Result<&'static str, RouterError> {
    let config = config.read().await;
    let client = match &config.kratos.client {
        Some(client) => client,
        None => Err(anyhow!("Kratos client not initialized"))?,
    };
    is_ready(client).await?;
    Ok("200")
}

#[cfg(test)]
mod http_router_test {
    use std::sync::Arc;

    use crate::{
        app,
        config::{IamConfig, CONFIG_FALLBACK},
    };
    use axum::{
        body::Body,
        http::{self, Request, StatusCode},
    };
    use rs_utils::config::Config;
    use serde_json::json;
    use tokio::sync::RwLock;
    use tower::ServiceExt;

    async fn create_config() -> Arc<RwLock<IamConfig>> {
        Arc::new(RwLock::new(IamConfig::new(CONFIG_FALLBACK).await))
    }

    #[tokio::test]
    async fn test_add() {
        let config = create_config().await;
        let app = app(config);

        let response = app
            .oneshot(
                Request::builder()
                    .method(http::Method::POST)
                    .uri("/api/iam/policy")
                    .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                    .body(Body::from(
                        serde_json::to_string(&json!({
                          "id": "1",
                          "perm_type": "project",
                          "resource": "222",
                          "value": "\"contributor\"",
                          "mode": 0,
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_remove() {
        let config = create_config().await;
        let app = app(config);

        let response = app
            .oneshot(
                Request::builder()
                    .method(http::Method::DELETE)
                    .uri("/api/iam/policy")
                    .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                    .body(Body::from(
                        serde_json::to_string(&json!({
                          "id": "1",
                          "perm_type": "project",
                          "resource": "222",
                          "value": "\"contributor\"",
                          "mode": 0,
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_replace() {
        let config = create_config().await;
        let app = app(config);

        let response = app
            .oneshot(
                Request::builder()
                    .method(http::Method::PUT)
                    .uri("/api/iam/policy")
                    .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                    .body(Body::from(
                        serde_json::to_string(&json!({
                          "id": "1",
                          "perm_type": "project",
                          "resource": "222",
                          "value": "\"contributor\"",
                          "mode": 0,
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}
