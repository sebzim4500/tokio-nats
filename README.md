[![Crates.io](https://img.shields.io/crates/v/tokio-nats.svg)](https://crates.io/crates/tokio-nats)
[![docs.rs](https://docs.rs/tokio-nats/badge.svg)](https://docs.rs/tokio-nats)
A client for NATS using `tokio` and async-await.
There are still features missing, but it should be ready for use in simple situations.
## Installation
```toml
[dependencies]
tokio-nats = "0.2.0"
```
## Usage
```rust
use tokio_nats::{NatsConfigBuilder, connect};
use futures_util::StreamExt;
async fn demo() {
    let config = NatsConfigBuilder::default()
        .server("127.0.0.1:4222")
        .build()
        .unwrap();
    let mut client = connect(config).await.unwrap();
    client.publish("MySubject", "hello world".as_bytes()).await.unwrap();
    client.subscribe("MyOtherSubject").await.unwrap().for_each(async move |message| {
        println!("Received message {:?}", message);
    }).await;
}
```