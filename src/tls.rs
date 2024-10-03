use std::io;
use std::io::BufRead;
use std::sync::Arc;

use rustls_pemfile::{certs, read_one};
use tokio::net::TcpStream;
use tokio_rustls::client::TlsStream;
use tokio_rustls::rustls::pki_types::{CertificateDer, PrivateKeyDer, ServerName, TrustAnchor};
use tokio_rustls::rustls::{ClientConfig, RootCertStore};
use tokio_rustls::TlsConnector;

#[derive(Debug)]
pub enum TLSConnBuildError {
    NotConfigured,
    UnableParseClientCertificate,
    UnableParseClientKey,
    UnableParseCaCertificate,
    UnableToConnect(String),
}

#[derive(Debug)]
pub struct TlsConnParams {
    pub(crate) client_key: PrivateKeyDer<'static>,
    pub(crate) client_certs: Vec<CertificateDer<'static>>,
    pub(crate) root_cert: RootCertStore,
}

impl Clone for TlsConnParams {
    fn clone(&self) -> Self {
        TlsConnParams {
            client_key: self.client_key.clone_key(),
            client_certs: self.client_certs.clone(),
            root_cert: self.root_cert.clone(),
        }
    }
}

#[derive(Debug)]
pub struct TLSConnBuild {
    client_key: Option<PrivateKeyDer<'static>>,
    client_certs: Vec<CertificateDer<'static>>,
    root_cert: RootCertStore,
}

impl Default for TLSConnBuild {
    fn default() -> Self {
        Self::new()
    }
}

fn load_key(mut reader: &mut dyn BufRead) -> Result<PrivateKeyDer<'static>, TLSConnBuildError> {
    loop {
        match read_one(&mut reader).map_err(|_| TLSConnBuildError::UnableParseClientKey)? {
            Some(rustls_pemfile::Item::Sec1Key(key)) => {
                log::trace!("ECKey");
                return Ok(PrivateKeyDer::Sec1(key));
            }
            Some(rustls_pemfile::Item::Pkcs1Key(key)) => {
                log::trace!("RSAKey");
                return Ok(PrivateKeyDer::Pkcs1(key));
            }
            Some(rustls_pemfile::Item::Pkcs8Key(key)) => {
                log::trace!("PKCS8Key");
                return Ok(PrivateKeyDer::Pkcs8(key));
            }
            None => {
                log::debug!("No type found");
                break;
            }
            _ => {}
        }
    }
    Err(TLSConnBuildError::UnableParseClientKey)
}

impl TLSConnBuild {
    pub fn new() -> TLSConnBuild {
        TLSConnBuild {
            client_key: None,
            client_certs: Vec::new(),
            root_cert: RootCertStore::empty(),
        }
    }

    pub fn client_certs(&mut self, mut reader: &mut dyn BufRead) -> Result<(), TLSConnBuildError> {
        self.client_certs = certs(&mut reader)
            .collect::<Result<Vec<_>, io::Error>>()
            .map_err(|_| TLSConnBuildError::UnableParseClientCertificate)?;

        Ok(())
    }

    pub fn client_key(&mut self, reader: &mut dyn BufRead) -> Result<(), TLSConnBuildError> {
        self.client_key = Some(load_key(reader)?);
        Ok(())
    }

    pub fn root_cert(&mut self, mut reader: &mut dyn BufRead) -> Result<(), TLSConnBuildError> {
        let certs = certs(&mut reader)
            .collect::<Result<Vec<_>, io::Error>>()
            .map_err(|_| TLSConnBuildError::UnableParseCaCertificate)?;

        let trust_anchors: Result<Vec<TrustAnchor>, TLSConnBuildError> = certs
            .iter()
            .map(|cert| {
                webpki::anchor_from_trusted_cert(cert)
                    .map_err(|_| TLSConnBuildError::UnableParseCaCertificate)
                    .map(|ta| ta.to_owned())
            })
            .collect();
        self.root_cert.extend(trust_anchors?);
        Ok(())
    }

    pub(crate) fn is_ready(&self) -> bool {
        !self.root_cert.is_empty() && !self.client_certs.is_empty() && self.client_key.is_some()
    }

    pub fn build(self) -> Result<TlsConnParams, TLSConnBuildError> {
        if !self.is_ready() {
            return Err(TLSConnBuildError::NotConfigured);
        }

        if let (Some(client_key), client_certs, root_cert) =
            (self.client_key, self.client_certs, self.root_cert)
        {
            Ok(TlsConnParams {
                client_key,
                client_certs,
                root_cert,
            })
        } else {
            Err(TLSConnBuildError::NotConfigured)
        }
    }
}

pub(crate) async fn connect(
    stream: TcpStream,
    domain: &str,
    tls_params: TlsConnParams,
) -> Result<TlsStream<TcpStream>, TLSConnBuildError> {
    let domain = ServerName::try_from(domain.to_owned())
        .map_err(|_| TLSConnBuildError::UnableToConnect("Invalid dns name".to_owned()))?;

    log::trace!("Connecting TLS using domain: {domain:?}");

    let config_builder = ClientConfig::builder().with_root_certificates(tls_params.root_cert);

    let config = config_builder
        .with_client_auth_cert(tls_params.client_certs, tls_params.client_key)
        .map_err(|_| TLSConnBuildError::UnableToConnect("Unable to TLS config".to_owned()))?;

    let connector = TlsConnector::from(Arc::new(config));

    connector
        .connect(domain, stream)
        .await
        .map_err(|e| TLSConnBuildError::UnableToConnect(e.to_string()))
}
