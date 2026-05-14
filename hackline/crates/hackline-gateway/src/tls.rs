//! TLS termination for the gateway (gated behind the `tls` feature).
//!
//! Three modes:
//! - **ACME**: obtains certs from Let's Encrypt via HTTP-01 challenge.
//! - **Manual**: loads PEM cert+key from disk.
//! - **Self-signed**: generates an ephemeral cert on startup.
//!
//! All modes produce an `axum_server::tls_rustls::RustlsConfig` that
//! can be shared across the REST API and tunnel TCP listeners.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use axum_server::tls_rustls::RustlsConfig;
use tokio_rustls::TlsAcceptor;
use tracing::{info, warn};

use crate::config::{TlsConfig, TlsMode};
use crate::error::GatewayError;

/// Shared TLS state: the `RustlsConfig` for axum-server's REST
/// listener and a `TlsAcceptor` for wrapping raw tunnel TCP sockets.
#[derive(Clone)]
pub struct TlsState {
    pub axum_config: RustlsConfig,
    pub acceptor: TlsAcceptor,
}

/// Build TLS state from the parsed `[tls]` config block.
pub async fn init(cfg: &TlsConfig) -> Result<TlsState, GatewayError> {
    let mode = cfg.mode()?;
    match mode {
        TlsMode::SelfSigned => init_self_signed(cfg).await,
        TlsMode::Manual => init_manual(cfg).await,
        TlsMode::Acme => init_acme(cfg).await,
    }
}

// ── Self-signed (dev only) ──────────────────────────────────────────

async fn init_self_signed(_cfg: &TlsConfig) -> Result<TlsState, GatewayError> {
    warn!("TLS: generating self-signed certificate (not for production)");

    let subject_alt_names = vec!["localhost".to_string(), "127.0.0.1".to_string()];
    let cert_params = rcgen::CertificateParams::new(subject_alt_names)
        .map_err(|e| GatewayError::Config(format!("rcgen params: {e}")))?;
    let key_pair = rcgen::KeyPair::generate()
        .map_err(|e| GatewayError::Config(format!("rcgen keygen: {e}")))?;
    let cert = cert_params
        .self_signed(&key_pair)
        .map_err(|e| GatewayError::Config(format!("rcgen self_signed: {e}")))?;

    let cert_pem = cert.pem();
    let key_pem = key_pair.serialize_pem();

    build_state_from_pem(cert_pem.as_bytes(), key_pem.as_bytes()).await
}

// ── Manual certs ────────────────────────────────────────────────────

async fn init_manual(cfg: &TlsConfig) -> Result<TlsState, GatewayError> {
    let cert_path = cfg.cert_path.as_deref().ok_or_else(|| {
        GatewayError::Config("[tls] manual mode: cert_path required".into())
    })?;
    let key_path = cfg.key_path.as_deref().ok_or_else(|| {
        GatewayError::Config("[tls] manual mode: key_path required".into())
    })?;

    info!(cert = cert_path, key = key_path, "TLS: loading manual certs");

    let cert_pem = tokio::fs::read(cert_path)
        .await
        .map_err(|e| GatewayError::Config(format!("read {cert_path}: {e}")))?;
    let key_pem = tokio::fs::read(key_path)
        .await
        .map_err(|e| GatewayError::Config(format!("read {key_path}: {e}")))?;

    build_state_from_pem(&cert_pem, &key_pem).await
}

// ── ACME (Let's Encrypt) ────────────────────────────────────────────

async fn init_acme(cfg: &TlsConfig) -> Result<TlsState, GatewayError> {
    let domain = cfg.acme_domain.as_deref().unwrap();
    let email = cfg.acme_email.as_deref().unwrap();

    let cache_dir = cfg
        .acme_cache_dir
        .as_deref()
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            dirs::state_dir()
                .or_else(dirs::data_local_dir)
                .unwrap_or_else(|| PathBuf::from("."))
                .join("hackline")
                .join("acme")
        });

    tokio::fs::create_dir_all(&cache_dir)
        .await
        .map_err(|e| GatewayError::Config(format!("create {cache_dir:?}: {e}")))?;

    let cert_file = cache_dir.join(format!("{domain}.cert.pem"));
    let key_file = cache_dir.join(format!("{domain}.key.pem"));

    // If cached certs exist and are recent, use them directly.
    if cert_file.exists() && key_file.exists() {
        info!(cert = ?cert_file, "TLS/ACME: using cached certificate");
        let cert_pem = tokio::fs::read(&cert_file)
            .await
            .map_err(|e| GatewayError::Config(format!("read {cert_file:?}: {e}")))?;
        let key_pem = tokio::fs::read(&key_file)
            .await
            .map_err(|e| GatewayError::Config(format!("read {key_file:?}: {e}")))?;
        return build_state_from_pem(&cert_pem, &key_pem).await;
    }

    info!(domain, "TLS/ACME: requesting certificate from Let's Encrypt");

    let directory_url = if cfg.acme_staging {
        instant_acme::LetsEncrypt::Staging.url()
    } else {
        instant_acme::LetsEncrypt::Production.url()
    };

    // Load or create ACME account
    let account_file = cache_dir.join("account.json");
    let account = load_or_create_account(&account_file, directory_url, email).await?;

    // Create order
    let identifiers = [instant_acme::Identifier::Dns(domain.to_string())];
    let mut order = account
        .new_order(&instant_acme::NewOrder::new(&identifiers))
        .await
        .map_err(|e| GatewayError::Config(format!("ACME new_order: {e}")))?;

    // Walk authorizations, extract HTTP-01 challenges, set them ready.
    let mut challenge_map = std::collections::HashMap::new();
    {
        let mut authz_stream = order.authorizations();
        while let Some(mut authz) = authz_stream
            .next()
            .await
            .transpose()
            .map_err(|e| GatewayError::Config(format!("ACME authorization: {e}")))?
        {
            let mut ch = authz
                .challenge(instant_acme::ChallengeType::Http01)
                .ok_or_else(|| {
                    GatewayError::Config("ACME: no HTTP-01 challenge offered".into())
                })?;
            let ka = ch.key_authorization();
            challenge_map.insert(ch.token.clone(), ka.as_str().to_string());
            ch.set_ready()
                .await
                .map_err(|e| GatewayError::Config(format!("ACME set_ready: {e}")))?;
        }
    }

    // Spawn a temporary HTTP server on port 80 for the challenge
    let challenge_server =
        spawn_challenge_server(Arc::new(challenge_map)).await?;

    // Wait until the order is ready for finalization
    let retries = instant_acme::RetryPolicy::default();
    order
        .poll_ready(&retries)
        .await
        .map_err(|e| GatewayError::Config(format!("ACME poll_ready: {e}")))?;

    // Finalize — generates a key pair + CSR internally (rcgen feature)
    let key_pem = order
        .finalize()
        .await
        .map_err(|e| GatewayError::Config(format!("ACME finalize: {e}")))?;

    // Retrieve the signed certificate chain
    let cert_chain = order
        .poll_certificate(&retries)
        .await
        .map_err(|e| GatewayError::Config(format!("ACME certificate: {e}")))?;

    // Stop challenge server
    challenge_server.abort();

    // Persist to cache
    tokio::fs::write(&cert_file, cert_chain.as_bytes())
        .await
        .map_err(|e| GatewayError::Config(format!("write {cert_file:?}: {e}")))?;
    tokio::fs::write(&key_file, key_pem.as_bytes())
        .await
        .map_err(|e| GatewayError::Config(format!("write {key_file:?}: {e}")))?;

    info!(domain, "TLS/ACME: certificate obtained and cached");

    build_state_from_pem(cert_chain.as_bytes(), key_pem.as_bytes()).await
}

async fn load_or_create_account(
    path: &Path,
    directory_url: &str,
    email: &str,
) -> Result<instant_acme::Account, GatewayError> {
    let builder = instant_acme::Account::builder()
        .map_err(|e| GatewayError::Config(format!("ACME account builder: {e}")))?;

    if path.exists() {
        let json = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| GatewayError::Config(format!("read {path:?}: {e}")))?;
        let creds: instant_acme::AccountCredentials = serde_json::from_str(&json)
            .map_err(|e| GatewayError::Config(format!("parse account: {e}")))?;
        let account = builder
            .from_credentials(creds)
            .await
            .map_err(|e| GatewayError::Config(format!("load ACME account: {e}")))?;
        info!("ACME: loaded cached account");
        return Ok(account);
    }

    let (account, creds) = builder
        .create(
            &instant_acme::NewAccount {
                contact: &[&format!("mailto:{email}")],
                terms_of_service_agreed: true,
                only_return_existing: false,
            },
            directory_url.to_string(),
            None,
        )
        .await
        .map_err(|e| GatewayError::Config(format!("create ACME account: {e}")))?;

    let json = serde_json::to_string_pretty(&creds)
        .map_err(|e| GatewayError::Config(format!("serialize account: {e}")))?;
    tokio::fs::write(path, json.as_bytes())
        .await
        .map_err(|e| GatewayError::Config(format!("write {path:?}: {e}")))?;

    info!("ACME: created new account");
    Ok(account)
}

/// Ephemeral HTTP server on port 80 that responds to
/// `GET /.well-known/acme-challenge/<token>` with the key authorization.
async fn spawn_challenge_server(
    tokens: Arc<std::collections::HashMap<String, String>>,
) -> Result<tokio::task::JoinHandle<()>, GatewayError> {
    use axum::{extract::Path as AxumPath, routing::get, Router};

    let app = Router::new().route(
        "/.well-known/acme-challenge/{token}",
        get(move |AxumPath(token): AxumPath<String>| {
            let tokens = tokens.clone();
            async move {
                match tokens.get(&token) {
                    Some(proof) => (axum::http::StatusCode::OK, proof.clone()),
                    None => (axum::http::StatusCode::NOT_FOUND, String::new()),
                }
            }
        }),
    );

    let listener = tokio::net::TcpListener::bind("0.0.0.0:80")
        .await
        .map_err(|e| GatewayError::Config(format!("bind :80 for ACME challenge: {e}")))?;
    info!("ACME: HTTP-01 challenge responder listening on :80");

    let handle = tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });

    Ok(handle)
}

// ── Shared helpers ──────────────────────────────────────────────────

async fn build_state_from_pem(
    cert_pem: &[u8],
    key_pem: &[u8],
) -> Result<TlsState, GatewayError> {
    let axum_config = RustlsConfig::from_pem(cert_pem.to_vec(), key_pem.to_vec())
        .await
        .map_err(|e| GatewayError::Config(format!("rustls config: {e}")))?;

    // Build a TlsAcceptor from the same certs for tunnel TCP sockets.
    let certs = rustls_pemfile::certs(&mut std::io::BufReader::new(cert_pem))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| GatewayError::Config(format!("parse cert PEM: {e}")))?;
    let key = rustls_pemfile::private_key(&mut std::io::BufReader::new(key_pem))
        .map_err(|e| GatewayError::Config(format!("parse key PEM: {e}")))?
        .ok_or_else(|| GatewayError::Config("no private key in PEM".into()))?;

    let mut server_config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .map_err(|e| GatewayError::Config(format!("rustls server config: {e}")))?;
    server_config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];

    let acceptor = TlsAcceptor::from(Arc::new(server_config));

    Ok(TlsState {
        axum_config,
        acceptor,
    })
}
