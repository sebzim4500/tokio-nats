use std::io::BufRead;
use std::sync::Arc;

use rustls_pemfile::{certs, read_one};
use tokio::net::TcpStream;
use tokio_rustls::client::TlsStream;
use tokio_rustls::rustls::{
    Certificate, ClientConfig, OwnedTrustAnchor, PrivateKey, RootCertStore, ServerName,
};
use tokio_rustls::TlsConnector;
use webpki::TrustAnchor;

#[derive(Debug)]
pub enum TLSConnBuildError {
    NotConfigured,
    UnableParseClientCertificate,
    UnableParseClientKey,
    UnableParseCaCertificate,
    UnableToConnect(String),
}

#[derive(Debug, Clone)]
pub struct TlsConnParams {
    pub(crate) client_key: PrivateKey,
    pub(crate) client_certs: Vec<Certificate>,
    pub(crate) root_cert: RootCertStore,
}

#[derive(Debug)]
pub struct TLSConnBuild {
    client_key: Option<PrivateKey>,
    client_certs: Vec<Certificate>,
    root_cert: RootCertStore,
}

impl Default for TLSConnBuild {
    fn default() -> Self {
        Self::new()
    }
}

fn load_key(mut reader: &mut dyn BufRead) -> Result<PrivateKey, TLSConnBuildError> {
    loop {
        match read_one(&mut reader).map_err(|_| TLSConnBuildError::UnableParseClientKey)? {
            Some(rustls_pemfile::Item::ECKey(key)) => {
                log::trace!("ECKey");
                return Ok(PrivateKey(key));
            }
            Some(rustls_pemfile::Item::RSAKey(key)) => {
                log::trace!("RSAKey");
                return Ok(PrivateKey(key));
            }
            Some(rustls_pemfile::Item::PKCS8Key(key)) => {
                log::trace!("PKCS8Key");
                return Ok(PrivateKey(key));
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
            .map(|mut certs| certs.drain(..).map(Certificate).collect())
            .map_err(|_| TLSConnBuildError::UnableParseClientCertificate)?;

        Ok(())
    }

    pub fn client_key(&mut self, reader: &mut dyn BufRead) -> Result<(), TLSConnBuildError> {
        self.client_key = Some(load_key(reader)?);
        Ok(())
    }

    pub fn root_cert(&mut self, mut reader: &mut dyn BufRead) -> Result<(), TLSConnBuildError> {
        let certs = certs(&mut reader).map_err(|_| TLSConnBuildError::UnableParseCaCertificate)?;

        let trust_anchors: Result<Vec<OwnedTrustAnchor>, TLSConnBuildError> = certs
            .iter()
            .map(|cert| {
                let ta: Result<TrustAnchor, TLSConnBuildError> =
                    webpki::TrustAnchor::try_from_cert_der(cert)
                        .map_err(|_| TLSConnBuildError::UnableParseCaCertificate);
                let ta = ta?;
                Ok(OwnedTrustAnchor::from_subject_spki_name_constraints(
                    ta.subject,
                    ta.spki,
                    ta.name_constraints,
                ))
            })
            .collect();
        let trust_anchors = trust_anchors?;
        self.root_cert
            .add_server_trust_anchors(trust_anchors.into_iter());
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
    let domain = ServerName::try_from(domain)
        .map_err(|_| TLSConnBuildError::UnableToConnect("Invalid dns name".to_owned()))?;

    log::trace!("Connecting TLS using domain: {domain:?}");

    let config_builder = ClientConfig::builder()
        .with_safe_defaults()
        .with_root_certificates(tls_params.root_cert);

    let config = config_builder
        .with_single_cert(tls_params.client_certs, tls_params.client_key)
        .map_err(|_| TLSConnBuildError::UnableToConnect("Unable to TLS config".to_owned()))?;

    let connector = TlsConnector::from(Arc::new(config));

    connector
        .connect(domain, stream)
        .await
        .map_err(|e| TLSConnBuildError::UnableToConnect(e.to_string()))
}
