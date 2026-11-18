use super::*;
use rcgen::{BasicConstraints, Certificate, CertificateParams, IsCa, KeyPair};
use std::io::{self, Read};

struct BrokenReader;

#[test]
fn keys_preserve_formats_first_match_and_reader_position() {
    for (label, expected) in [
        ("RSA PRIVATE KEY", 1),
        ("EC PRIVATE KEY", 2),
        ("PRIVATE KEY", 3),
    ] {
        let pem = format!(
            "ignored\n-----BEGIN CERTIFICATE-----\nAQID\n-----END CERTIFICATE-----\n-----BEGIN {label}-----\nBAUG\n-----END {label}-----\nremaining\n"
        );
        let mut reader = pem.as_bytes();
        let mut builder = TLSConnBuild::new();
        builder.client_key(&mut reader).unwrap();

        let key = builder.client_key.unwrap();
        assert_eq!(key.secret_der(), &[4, 5, 6]);
        assert!(matches!(
            (expected, key),
            (1, PrivateKeyDer::Pkcs1(_))
                | (2, PrivateKeyDer::Sec1(_))
                | (3, PrivateKeyDer::Pkcs8(_))
        ));
        assert_eq!(reader, b"remaining\n");
    }
    let mut reader = b"-----BEGIN PRIVATE KEY-----\nAQID\n-----END PRIVATE KEY-----\n-----BEGIN RSA PRIVATE KEY-----\nBAUG\n-----END RSA PRIVATE KEY-----\n".as_slice();
    let mut builder = TLSConnBuild::new();
    builder.client_key(&mut reader).unwrap();
    assert_eq!(
        builder.client_key.as_ref().unwrap().secret_der(),
        &[1, 2, 3]
    );
    builder.client_key(&mut reader).unwrap();
    assert_eq!(
        builder.client_key.as_ref().unwrap().secret_der(),
        &[4, 5, 6]
    );
}

#[test]
fn certificate_bundles_preserve_order_replacement_and_root_accumulation() {
    let (ca, ca_key) = authority();
    let (cert, key) = identity(&ca, &ca_key, false);
    let (other_ca, _) = authority();
    let mut builder = configured(&ca, &cert, &key);
    let bundle = format!("{}{}{}", cert.pem(), key.serialize_pem(), ca.pem());
    builder.client_certs(&mut bundle.as_bytes()).unwrap();
    assert_eq!(
        builder.client_certs,
        vec![cert.der().clone(), ca.der().clone()]
    );
    builder.root_cert(&mut other_ca.pem().as_bytes()).unwrap();
    assert_eq!(builder.root_cert.len(), 2);
    builder.client_certs(&mut ca.pem().as_bytes()).unwrap();
    assert_eq!(builder.client_certs, vec![ca.der().clone()]);

    let params = builder.build().unwrap();
    let cloned = params.clone();
    assert_eq!(
        params.client_key.secret_der(),
        cloned.client_key.secret_der()
    );
    assert_eq!(params.client_certs, cloned.client_certs);
    assert_eq!(params.root_cert.roots, cloned.root_cert.roots);
}

#[test]
fn parse_errors_leave_previous_configuration_intact() {
    let (ca, ca_key) = authority();
    let (cert, key) = identity(&ca, &ca_key, false);
    let mut builder = configured(&ca, &cert, &key);
    for invalid in [
        "-----BEGIN CERTIFICATE-----\n!\n-----END CERTIFICATE-----\n",
        "-----BEGIN CERTIFICATE-----\nAQID\n",
    ] {
        assert!(matches!(
            builder.client_certs(&mut invalid.as_bytes()),
            Err(TLSConnBuildError::UnableParseClientCertificate)
        ));
        assert!(matches!(
            builder.root_cert(&mut invalid.as_bytes()),
            Err(TLSConnBuildError::UnableParseCaCertificate)
        ));
    }
    let bad_bundle = format!(
        "{}-----BEGIN CERTIFICATE-----\nAQID\n-----END CERTIFICATE-----\n",
        ca.pem()
    );
    assert!(matches!(
        builder.root_cert(&mut bad_bundle.as_bytes()),
        Err(TLSConnBuildError::UnableParseCaCertificate)
    ));
    for invalid in [
        "",
        "unrelated text",
        "-----BEGIN PRIVATE KEY-----\n!\n-----END PRIVATE KEY-----\n",
        "-----BEGIN PRIVATE KEY-----\nAQID\n",
    ] {
        assert!(matches!(
            builder.client_key(&mut invalid.as_bytes()),
            Err(TLSConnBuildError::UnableParseClientKey)
        ));
    }
    assert_eq!(builder.client_certs, vec![cert.der().clone()]);
    assert_eq!(builder.root_cert.len(), 1);
    assert_eq!(
        builder.client_key.as_ref().unwrap().secret_der(),
        key.serialize_der()
    );
    assert!(builder.is_ready());
}

#[test]
fn io_errors_and_empty_inputs_keep_existing_error_categories() {
    let mut builder = TLSConnBuild::new();
    assert!(matches!(
        builder.client_key(&mut BrokenReader),
        Err(TLSConnBuildError::UnableParseClientKey)
    ));
    assert!(matches!(
        builder.client_certs(&mut BrokenReader),
        Err(TLSConnBuildError::UnableParseClientCertificate)
    ));
    assert!(matches!(
        builder.root_cert(&mut BrokenReader),
        Err(TLSConnBuildError::UnableParseCaCertificate)
    ));
    builder.client_certs(&mut b"".as_slice()).unwrap();
    builder.root_cert(&mut b"".as_slice()).unwrap();
    assert!(matches!(
        builder.build(),
        Err(TLSConnBuildError::NotConfigured)
    ));
}

#[cfg(any(feature = "aws_lc_rs", feature = "ring"))]
mod handshake {
    use super::*;
    use rustls::{server::WebPkiClientVerifier, ServerConfig};
    use std::time::Duration;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;
    use tokio_rustls::TlsAcceptor;

    #[tokio::test]
    async fn mutual_tls_authenticates_both_peers_and_exchanges_data() {
        for tls12 in [false, true] {
            assert_eq!(
                exchange("localhost", true, true, false, false, tls12).await,
                (true, true)
            );
        }
    }

    #[tokio::test]
    async fn tls_rejects_wrong_names_untrusted_peers_expiry_and_mismatched_keys() {
        for tls12 in [false, true] {
            for (domain, server, client, expired, mismatch) in [
                ("wrong.example", true, true, false, false),
                ("invalid name", true, true, false, false),
                ("localhost", false, true, false, false),
                ("localhost", true, false, false, false),
                ("localhost", true, true, true, false),
                ("localhost", true, true, false, true),
            ] {
                assert_eq!(
                    exchange(domain, server, client, expired, mismatch, tls12).await,
                    (false, false)
                );
            }
        }
    }
    async fn exchange(
        domain: &str,
        trusted_server: bool,
        trusted_client: bool,
        expired: bool,
        mismatched_key: bool,
        tls12: bool,
    ) -> (bool, bool) {
        #[cfg(feature = "aws_lc_rs")]
        let provider = rustls::crypto::aws_lc_rs::default_provider();
        #[cfg(all(feature = "ring", not(feature = "aws_lc_rs")))]
        let provider = rustls::crypto::ring::default_provider();
        let _ = provider.install_default();

        let (ca, ca_key) = authority();
        let (other_ca, _) = authority();
        let intermediate_key = KeyPair::generate().unwrap();
        let mut intermediate_params = CertificateParams::default();
        intermediate_params.is_ca = IsCa::Ca(BasicConstraints::Constrained(0));
        intermediate_params
            .distinguished_name
            .push(rcgen::DnType::CommonName, "intermediate");
        let intermediate = intermediate_params
            .signed_by(&intermediate_key, &ca, &ca_key)
            .unwrap();
        let (server_cert, server_key) = identity(&intermediate, &intermediate_key, expired);
        let (client_cert, client_key) = identity(&intermediate, &intermediate_key, false);

        let mut roots = RootCertStore::empty();
        roots
            .add(if trusted_client {
                ca.der().clone()
            } else {
                other_ca.der().clone()
            })
            .unwrap();
        let verifier = WebPkiClientVerifier::builder(Arc::new(roots))
            .build()
            .unwrap();
        let version = if tls12 {
            &rustls::version::TLS12
        } else {
            &rustls::version::TLS13
        };
        let config = ServerConfig::builder_with_protocol_versions(&[version])
            .with_client_cert_verifier(verifier)
            .with_single_cert(
                vec![server_cert.der().clone(), intermediate.der().clone()],
                PrivateKeyDer::Pkcs8(server_key.serialize_der().into()),
            )
            .unwrap();

        let mut builder = configured(
            if trusted_server { &ca } else { &other_ca },
            &client_cert,
            &client_key,
        );
        builder
            .client_certs(&mut format!("{}{}", client_cert.pem(), intermediate.pem()).as_bytes())
            .unwrap();
        if mismatched_key {
            builder
                .client_key(&mut KeyPair::generate().unwrap().serialize_pem().as_bytes())
                .unwrap();
        }
        let params = builder.build().unwrap().clone();

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let expected_client = vec![client_cert.der().clone(), intermediate.der().clone()];

        let server = async move {
            let (socket, _) = listener.accept().await.unwrap();
            let Ok(mut stream) = TlsAcceptor::from(Arc::new(config)).accept(socket).await else {
                return false;
            };
            assert_eq!(
                stream.get_ref().1.peer_certificates().unwrap(),
                expected_client
            );
            let mut data = [0; 6];
            if stream.read_exact(&mut data).await.is_err() {
                return false;
            }
            assert_eq!(&data, b"PING\r\n");
            stream.write_all(b"PONG\r\n").await.unwrap();
            true
        };

        let client = async move {
            let socket = TcpStream::connect(addr).await.unwrap();
            let Ok(mut stream) = connect(socket, domain, params).await else {
                return false;
            };
            if stream.write_all(b"PING\r\n").await.is_err() {
                return false;
            }
            let mut data = [0; 6];
            if stream.read_exact(&mut data).await.is_err() {
                return false;
            }
            assert_eq!(&data, b"PONG\r\n");
            true
        };

        tokio::time::timeout(Duration::from_secs(10), async {
            tokio::join!(client, server)
        })
        .await
        .unwrap()
    }
}

fn authority() -> (Certificate, KeyPair) {
    let key = KeyPair::generate().unwrap();
    let mut params = CertificateParams::default();
    params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    (params.self_signed(&key).unwrap(), key)
}

fn identity(ca: &Certificate, ca_key: &KeyPair, expired: bool) -> (Certificate, KeyPair) {
    let key = KeyPair::generate().unwrap();
    let mut params = CertificateParams::new(vec!["localhost".into()]).unwrap();
    if expired {
        params.not_before = rcgen::date_time_ymd(2000, 1, 1);
        params.not_after = rcgen::date_time_ymd(2001, 1, 1);
    }
    (params.signed_by(&key, ca, ca_key).unwrap(), key)
}

fn configured(ca: &Certificate, cert: &Certificate, key: &KeyPair) -> TLSConnBuild {
    let mut builder = TLSConnBuild::new();
    builder.root_cert(&mut ca.pem().as_bytes()).unwrap();
    builder.client_certs(&mut cert.pem().as_bytes()).unwrap();
    builder
        .client_key(&mut key.serialize_pem().as_bytes())
        .unwrap();
    builder
}

impl Read for BrokenReader {
    fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
        Err(io::Error::other("read failed"))
    }
}

impl BufRead for BrokenReader {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        Err(io::Error::other("read failed"))
    }

    fn consume(&mut self, _: usize) {}
}
