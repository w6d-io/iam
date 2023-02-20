use std::sync::Arc;

use anyhow::Result;
use rustls_pemfile::Item;
use tokio_rustls::rustls::{self, server, RootCertStore, ServerConfig};

pub async fn build_rustls_server_config(
    cert: &str,
    key: &str,
    ca: &str,
) -> Result<Arc<ServerConfig>> {
    let cert = tokio::fs::read(cert).await?;
    let key = tokio::fs::read(key).await?;

    // get pem from file
    let cert = rustls_pemfile::certs(&mut cert.as_ref())?;
    let key = match rustls_pemfile::read_one(&mut key.as_ref())? {
        Some(Item::RSAKey(key)) | Some(Item::PKCS8Key(key)) => key,
        // rustls only support PKCS8, does not support ECC private key
        _ => panic!("private key invalid or not supported"),
    };
    let cert = cert.into_iter().map(rustls::Certificate).collect();
    let key = rustls::PrivateKey(key);

    let config_builder = ServerConfig::builder().with_safe_defaults();

    tracing::info!("mTLS enabled, ca cert path={}", ca);
    let ca = tokio::fs::read(ca).await.unwrap();
    let mut server_config = if let Some(Item::X509Certificate(ca)) =
        rustls_pemfile::read_one(&mut ca.as_ref())?
    {
        let mut root_cert_store = RootCertStore::empty();
        root_cert_store
            .add(&rustls::Certificate(ca))
            .expect("bad ca cert");
        config_builder
            .with_client_cert_verifier(server::AllowAnyAuthenticatedClient::new(root_cert_store))
            .with_single_cert(cert, key)
            .expect("bad certificate/key")
    } else {
        panic!("invalid root ca cert")
    };
    server_config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
    Ok(Arc::new(server_config))
}
