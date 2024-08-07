use std::sync::Arc;

use anyhow::Result;
use rustls_pemfile::Item;
use tokio_rustls::rustls::{self, server, RootCertStore, ServerConfig};
use tracing::info;

///build a mtls server config
pub async fn build_rustls_server_config(
    cert: &str,
    key: &str,
    ca: &str,
) -> Result<Arc<ServerConfig>> {
    let cert = tokio::fs::read(cert).await?;
    let key = tokio::fs::read(key).await?;

    // get pem from file
    let cert = rustls_pemfile::certs(&mut cert.as_ref())?;
    let cert = cert.into_iter().map(rustls::Certificate).collect();

    let Some(Item::RSAKey(key) | Item::PKCS8Key(key)) =
        rustls_pemfile::read_one(&mut key.as_ref())?
    else {
        panic!("private key invalid or not supported")
    };

    let key = rustls::PrivateKey(key);

    let config_builder = ServerConfig::builder().with_safe_defaults();

    info!("mTLS ca cert path={}", ca);
    let ca = tokio::fs::read(ca).await.unwrap();
    let mut server_config =
        if let Some(Item::X509Certificate(ca)) = rustls_pemfile::read_one(&mut ca.as_ref())? {
            let mut root_cert_store = RootCertStore::empty();
            root_cert_store
                .add(&rustls::Certificate(ca))
                .expect("bad ca cert");
            config_builder
                .with_client_cert_verifier(
                    server::AllowAnyAuthenticatedClient::new(root_cert_store).boxed(),
                )
                .with_single_cert(cert, key)
                .expect("bad certificate/key")
        } else {
            panic!("invalid root ca cert")
        };
    server_config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
    Ok(Arc::new(server_config))
}

#[cfg(test)]
mod test_tls {
    use super::*;

    #[tokio::test]
    async fn test_valid() {
        build_rustls_server_config(
            "certificate/server_certs/server.cert.pem",
            "certificate/server_certs/server.key.pem",
            "certificate/certs/cacert.pem",
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    #[should_panic]
    async fn test_invalid_path() {
        build_rustls_server_config(
            "certificate/server_certs/server.cert.pem",
            "certificate/server_certs/servr.key.pem",
            "certificate/certs/cacert.pem",
        )
        .await
        .unwrap();
    }
}
