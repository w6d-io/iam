use std::{sync::Arc, time::Duration};

use anyhow::Result;
use axum::{
    routing::{get, post},
    Router,
};

use axum_server::{bind_rustls, tls_rustls::RustlsConfig, Handle};
use tokio::{sync::RwLock, task::JoinHandle};
use tonic::transport::Server as GrpcServer;
use tower_http::request_id::{MakeRequestUuid, SetRequestIdLayer};
use tracing::warn;
use tracing_subscriber::{fmt, EnvFilter};

pub mod permission {
    tonic::include_proto!("permission");
}
use permission::permission_srv_server::PermissionSrvServer;

use rs_utils::config::{init_watcher, Config};

mod mtls;
use mtls::build_rustls_server_config;
mod handelers;
use handelers::shutdown_signal;
mod grpc;
use grpc::router::MyPermissionSrv;
mod http;
use http::router::{add, alive, ready, remove, replace};
mod config;
use config::{PermissionsConfig, Service, CONFIG_FALLBACK};

type ConfigState = Arc<RwLock<PermissionsConfig>>;

async fn make_grpc(
    shared_state: ConfigState,
    config: Service,
) -> Result<JoinHandle<Result<(), tonic::transport::Error>>> {
    let service = PermissionSrvServer::new(MyPermissionSrv {
        config: shared_state,
    });
    let socket = (config.addr.clone() + ":" + &config.ports.grpc as &str).parse()?;
    let handle = tokio::spawn(
        GrpcServer::builder()
            .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
            .add_service(service)
            .serve_with_shutdown(socket, shutdown_signal()),
    );
    Ok(handle)
}

pub fn app(shared_state: ConfigState) -> Router {
    Router::new()
        .route("/api/permissions", post(add).delete(remove).put(replace))
        .with_state(shared_state)
        .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
}

pub fn health(shared_state: ConfigState) -> Router {
    Router::new()
        .route("/api/permissions/alive", get(alive))
        .route("/api/permissions/ready", get(ready))
        .with_state(shared_state)
}

async fn shutdown(handle: axum_server::Handle) {
    shutdown_signal().await;
    handle.graceful_shutdown(Some(Duration::from_secs(30)))
}

async fn make_http(
    shared_state: ConfigState,
    f: fn(ConfigState) -> Router,
    addr: String,
    handle: Handle,
    tls: &config::Tls
) -> Result<JoinHandle<Result<(), std::io::Error>>> {
    //todo: add path for tlscertificate
    let tls_config = build_rustls_server_config(&tls.certificate, &tls.key, &tls.cert_autority).await?;
    let rustls_config = RustlsConfig::from_config(tls_config);
    let handle = tokio::spawn(
        bind_rustls(addr.parse().unwrap(), rustls_config)
            .handle(handle)
            .serve(f(shared_state).into_make_service()), // .with_graceful_shutdown(shutdown_signal())
    );
    Ok(handle)
}

#[tokio::main]
async fn main() -> Result<()> {
    std::env::set_var("RUST_LOG", "INFO");
    fmt()
        .with_target(false)
        .with_level(true)
        .with_env_filter(EnvFilter::from_default_env())
        .init();
    let config_path = std::env::var("CONFIG").unwrap_or_else(|_| {
        warn!("Config variable not found switching to fallback");
        CONFIG_FALLBACK.to_owned()
    });
    let config = PermissionsConfig::new(&config_path).await;
    let service = config.service.clone();
    let tls = config.tls.clone();
    let shared_state = Arc::new(RwLock::new(config));
    tokio::spawn(init_watcher(config_path, shared_state.clone(), None));

    let handle = Handle::new();
    tokio::spawn(shutdown(handle.clone()));

    let http_addr = service.addr.clone() + ":" + &service.ports.http as &str;
    let http = make_http(shared_state.clone(), app, http_addr, handle.clone(), &tls).await?;

    let health_addr = service.addr.clone() + ":" + &service.ports.http_health as &str;
    let health = make_http(shared_state.clone(), health, health_addr, handle, &tls).await?;

    let grpc = make_grpc(shared_state, service).await?;
    let (grpc_critical, http_critical, health_critical) = tokio::try_join!(grpc, http, health)?;
    grpc_critical?;
    http_critical?;
    health_critical?;
    Ok(())
}
