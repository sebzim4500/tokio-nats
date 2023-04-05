use futures_util::StreamExt;
use tokio_nats::{connect, NatsConfigBuilder};

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

    let config = NatsConfigBuilder::default()
        .server(nats_addr)
        .ca_cert(ca_location)
        .client_cert(cert_location)
        .client_key(key_location)
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
