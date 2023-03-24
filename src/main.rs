use std::{sync::Arc, time::Duration};

use anyhow::Result;
use axum::{
    routing::{get, post},
    Router, Server,
};
use axum_server::{bind_rustls, tls_rustls::RustlsConfig, Handle};
use tokio::{sync::RwLock, task::JoinHandle};
use tonic::transport::Server as GrpcServer;
use tower_http::request_id::{MakeRequestUuid, SetRequestIdLayer};
use tracing::{info, warn};
use tracing_subscriber::{fmt, EnvFilter};

use rs_utils::config::{init_watcher, Config};

pub mod permission {
    tonic::include_proto!("permission");
}
use permission::iam_server::IamServer;
mod mtls;
use mtls::build_rustls_server_config;
mod handelers;
use handelers::{shutdown_signal, fallback};
mod grpc;
use grpc::router::MyIam;
mod http;
use http::router::{add, alive, ready, remove, replace};
mod config;
use config::{PermissionsConfig, Tls, CONFIG_FALLBACK};

type ConfigState = Arc<RwLock<PermissionsConfig>>;

///lauch the grpc router
async fn make_grpc(
    shared_state: ConfigState,
    addr: String,
) -> Result<JoinHandle<Result<(), tonic::transport::Error>>> {
    let service = IamServer::new(MyIam {
        config: shared_state,
    });
    info!("lauching grpc server on: {addr}");
    let socket = addr.parse()?;
    let handle = tokio::spawn(
        GrpcServer::builder()
            .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
            .add_service(service)
            .serve_with_shutdown(socket, shutdown_signal()),
    );
    Ok(handle)
}

///main router config
pub fn app(shared_state: ConfigState) -> Router {
    info!("configuring main router");
    Router::new()
        .route("/api/iam/policy", post(add).delete(remove).put(replace))
        .with_state(shared_state)
        .fallback(fallback)
        .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
}

///heatlh router config
pub fn health(shared_state: ConfigState) -> Router {
    info!("configuring health router");
    Router::new()
        .route("/api/iam/alive", get(alive))
        .route("/api/iam/ready", get(ready))
        .fallback(fallback)
        .with_state(shared_state)
}

///this function send the shutdown signal to the router
async fn shutdown(handle: axum_server::Handle) {
    shutdown_signal().await;
    handle.graceful_shutdown(Some(Duration::from_secs(30)))
}

///launch http router with mtls
async fn make_http_mtls(
    shared_state: ConfigState,
    f: fn(ConfigState) -> Router,
    addr: String,
    handle: &Handle,
    tls: &Tls,
) -> Result<JoinHandle<Result<(), std::io::Error>>> {
    //todo: add path for tlscertificate
    let tls_config =
        build_rustls_server_config(&tls.certificate, &tls.key, &tls.cert_autority).await?;
    let rustls_config = RustlsConfig::from_config(tls_config);
    let handle = tokio::spawn(
        bind_rustls(addr.parse().unwrap(), rustls_config)
            .handle(handle.to_owned())
            .serve(f(shared_state).into_make_service()),
    );
    info!("lauching http server on: {addr}");
    Ok(handle)
}

///launch simple http router
async fn make_http(
    shared_state: ConfigState,
    f: fn(ConfigState) -> Router,
    addr: String,
) -> Result<JoinHandle<Result<(), hyper::Error>>> {
    //todo: add path for tlscertificate
    let handle = tokio::spawn(
        Server::bind(&addr.parse().unwrap())
            .serve(f(shared_state).into_make_service())
            .with_graceful_shutdown(shutdown_signal()),
    );
    info!("lauching http server on: {addr}");
    Ok(handle)
}

#[tokio::main]
async fn main() -> Result<()> {
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

    info!("statrting http router");
    let http_addr = service.addr.clone() + ":" + &service.ports.http as &str;
    let http = make_http_mtls(shared_state.clone(), app, http_addr, &handle, &tls).await?;

    let health_addr = service.addr.clone() + ":" + &service.ports.http_health as &str;
    let health = make_http(shared_state.clone(), health, health_addr).await?;

    info!("statrting grpc router");
    let grpc_addr = service.addr.clone() + ":" + &service.ports.grpc as &str;
    let grpc = make_grpc(shared_state, grpc_addr).await?;
    let (grpc_critical, http_critical, health_critical) = tokio::try_join!(grpc, http, health)?;
    grpc_critical?;
    http_critical?;
    health_critical?;
    Ok(())
}
