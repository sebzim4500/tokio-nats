use std::{
    fs::File,
    io::{self, BufReader, Error},
    path::Path,
    sync::Arc,
};

use rustls_pemfile::{certs, read_one};
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;
use tokio_rustls::{
    client::TlsStream,
    rustls::{Certificate, ClientConfig, OwnedTrustAnchor, PrivateKey, RootCertStore, ServerName},
};

fn load_certs(path: &Path) -> io::Result<Vec<Certificate>> {
    certs(&mut BufReader::new(File::open(path)?))
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid cert"))
        .map(|mut certs| certs.drain(..).map(Certificate).collect())
}

fn load_key(path: &Path) -> PrivateKey {
    log::debug!("Path: {path:?}");
    let cert_file = File::open(path).expect("cannot open private key file");
    let mut reader = BufReader::new(cert_file);
    loop {
        match read_one(&mut reader).expect("cannot parse private key file") {
            Some(rustls_pemfile::Item::ECKey(key)) => {
                log::trace!("ECKey");
                return PrivateKey(key);
            }
            Some(rustls_pemfile::Item::RSAKey(key)) => {
                log::trace!("RSAKey");
                return PrivateKey(key);
            }
            Some(rustls_pemfile::Item::PKCS8Key(key)) => {
                log::trace!("PKCS8Key");
                return PrivateKey(key);
            }
            None => {
                log::debug!("No type found");
                break;
            }
            _ => {}
        }
    }
    panic!("No keys found in {path:?}");
}

pub(crate) async fn tls_connection(
    stream: TcpStream,
    domain: &str,
    ca_location: &str,
    cert_location: &str,
    key_location: &str,
) -> Result<TlsStream<TcpStream>, Error> {
    let mut root_cert_store = RootCertStore::empty();
    let mut pem = BufReader::new(File::open(ca_location)?);

    let certs = rustls_pemfile::certs(&mut pem)?;
    let trust_anchors = certs.iter().map(|cert| {
        let ta = webpki::TrustAnchor::try_from_cert_der(&cert[..]).unwrap();
        OwnedTrustAnchor::from_subject_spki_name_constraints(
            ta.subject,
            ta.spki,
            ta.name_constraints,
        )
    });
    root_cert_store.add_server_trust_anchors(trust_anchors);

    let config_builder = ClientConfig::builder()
        .with_safe_defaults()
        .with_root_certificates(root_cert_store);

    let config = {
        let certs = load_certs(Path::new(cert_location))?;
        let key = load_key(Path::new(key_location));
        config_builder
            .with_single_cert(certs, key)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, err))
            .unwrap()
    };

    let domain = ServerName::try_from(domain)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid dns name"))?;

    log::trace!("Connecting TLS using domain: {domain:?}");
    let connector = TlsConnector::from(Arc::new(config));

    connector.connect(domain, stream).await
}
