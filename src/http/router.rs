use std::sync::Arc;

use anyhow::anyhow;
use axum::{
    extract::{Json, State},
    response::Result,
    Extension,
};
use tracing::info;
use ory_kratos_client::apis::metadata_api::is_ready;
use tokio::sync::RwLock;
use tower_http::request_id::RequestId;

use crate::{
    config::PermissionsConfig,
    http::{controler::kratos_controler, error::RouterError},
    permission::Input,
};

///http route to add an identity field
pub async fn add(
    State(config): State<Arc<RwLock<PermissionsConfig>>>,
    request_id: Extension<RequestId>,
    Json(payload): Json<Input>,
) -> Result<(), RouterError> {
    let uuid = request_id.header_value().to_str()?;

    info!("{uuid}: adding data to identity");
    let config = config.read().await;
    let client = match &config.kratos.client {
        Some(client) => client,
        None => Err(anyhow!("{uuid}: Kratos client not initialized"))?,
    };
    kratos_controler(client, uuid, payload, "add").await?;
    info!("{uuid}: done");
    Ok(())
}

///http route to remove an identity field
pub async fn remove(
    State(config): State<Arc<RwLock<PermissionsConfig>>>,
    request_id: Extension<RequestId>,
    Json(payload): Json<Input>,
) -> Result<(), RouterError> {
    let uuid = request_id.header_value().to_str()?;
    info!("{uuid}: removing data to identity");
    let config = config.read().await;
    let client = match &config.kratos.client {
        Some(client) => client,
        None => Err(anyhow!("{uuid}: Kratos client not initialized"))?,
    };
    kratos_controler(client, uuid, payload, "remove").await?;
    info!("{uuid}: done");
    Ok(())
}

///http route to replace an identity field
pub async fn replace(
    State(config): State<Arc<RwLock<PermissionsConfig>>>,
    request_id: Extension<RequestId>,
    Json(payload): Json<Input>,
) -> Result<(), RouterError> {
    let uuid = request_id.header_value().to_str()?;
    info!("{uuid}: replacing data in identity");
    let config = config.read().await;
    let client = match &config.kratos.client {
        Some(client) => client,
        None => Err(anyhow!("{uuid}: Kratos client not initialized"))?,
    };
    kratos_controler(client, uuid, payload, "replace").await?;

    info!("{uuid}: done");
    Ok(())
}

pub async fn alive() -> Result<(), RouterError> {
    Ok(())
}

pub async fn ready(
    State(config): State<Arc<RwLock<PermissionsConfig>>>,
) -> Result<(), RouterError> {
    let config = config.read().await;
    let client = match &config.kratos.client {
        Some(client) => client,
        None => Err(anyhow!("Kratos client not initialized"))?,
    };
    is_ready(client).await?;
    Ok(())
}

#[cfg(test)]
mod http_router_test {
    use std::sync::Arc;

    use crate::{
        app,
        config::{PermissionsConfig, CONFIG_FALLBACK},
    };
    use axum::{
        body::Body,
        http::{self, Request, StatusCode},
    };
    use rs_utils::config::Config;
    use serde_json::json;
    use tokio::sync::RwLock;
    use tower::ServiceExt;

    async fn create_config() -> Arc<RwLock<PermissionsConfig>> {
        Arc::new(RwLock::new(PermissionsConfig::new(CONFIG_FALLBACK).await))
    }

    #[tokio::test]
    async fn test_add() {
        let config = create_config().await;
        let app = app(config);

        let response = app
            .oneshot(
                Request::builder()
                    .method(http::Method::POST)
                    .uri("/api/permissions")
                    .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                    .body(Body::from(
                        serde_json::to_string(&json!({
                          "id": "1",
                          "perm_type": "project",
                          "resource": "222",
                          "role": "contributor"
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
                    .uri("/api/permissions")
                    .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                    .body(Body::from(
                        serde_json::to_string(&json!({
                          "id": "1",
                          "perm_type": "project",
                          "resource": "222",
                          "role": "contributor"
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
                    .uri("/api/permissions")
                    .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                    .body(Body::from(
                        serde_json::to_string(&json!({
                          "id": "1",
                          "perm_type": "project",
                          "resource": "222",
                          "role": "contributor"
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
