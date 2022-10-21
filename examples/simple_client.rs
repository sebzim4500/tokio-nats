use futures_util::StreamExt;
use tokio_nats::{connect, NatsConfigBuilder};

#[tokio::main]
async fn main() {
    let config = NatsConfigBuilder::default()
        .server("127.0.0.1:4222")
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
