use std::{fs::File, io::BufReader};

use futures_util::StreamExt;
use tokio_nats::{connect, NatsConfigBuilder, TLSConnBuild};

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let mut args = std::env::args().skip(1);

    if args.len() < 3 {
        println!("Arguments: <ca cert location> <client cert location> <client key location> [NATS addr]");
        std::process::exit(1);
    }

    let ca_location = args.next().expect("Expected CA cert location");
    let cert_location = args.next().expect("Expected client cert location");
    let key_location = args.next().expect("Expected client key location");
    let nats_addr = args.next().unwrap_or_else(|| "127.0.0.1:4222".to_owned());

    println!("Parameters given: Addr: {nats_addr}\n\tCA location: {ca_location}\n\tClient cert location: {cert_location}\n\tClient key location: {key_location}");

    let mut tls_build = TLSConnBuild::new();

    let cert_file = File::open(cert_location).expect("cannot open private cert file");
    let mut reader = BufReader::new(cert_file);
    tls_build
        .client_certs(&mut reader)
        .expect("Unable to handle client certs");

    let key_file = File::open(key_location).expect("Cannot open private key file");
    let mut reader = BufReader::new(key_file);
    tls_build
        .client_key(&mut reader)
        .expect("Unable to handle client key");

    let ca_file = File::open(ca_location).expect("Cannot open CA cert file");
    let mut reader = BufReader::new(ca_file);
    tls_build
        .root_cert(&mut reader)
        .expect("Unable to load CA cert");

    let config = NatsConfigBuilder::default()
        .server(nats_addr)
        .tls_params(tls_build.build().expect("Unable to build TLS Parameters"))
        .build()
        .unwrap();
    let mut client = connect(config).await.unwrap();

    client
        .publish("MySubject", "hello world".as_bytes())
        .await
        .unwrap();

    client
        .subscribe("MyOtherSubject")
        .await
        .unwrap()
        .for_each(|message| async move {
            println!("Received message {:?}", message);
        })
        .await;
}
