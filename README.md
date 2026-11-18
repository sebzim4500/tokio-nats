# `tokio-nats`

<!-- prettier-ignore-start -->

[![crates.io](https://img.shields.io/crates/v/tokio-nats?label=latest)](https://crates.io/crates/tokio-nats)
[![Documentation](https://docs.rs/tokio-nats/badge.svg?version=0.5.0)](https://docs.rs/tokio-nats/0.5.0)
![LGPL-3.0 licensed](https://img.shields.io/crates/l/tokio-nats.svg)
<br />
[![dependency status](https://deps.rs/crate/tokio-nats/0.5.0/status.svg)](https://deps.rs/crate/tokio-nats/0.5.0)
[![Download](https://img.shields.io/crates/d/tokio-nats.svg)](https://crates.io/crates/tokio-nats)
[![CI](https://github.com/x52dev/tokio-nats/actions/workflows/ci.yml/badge.svg)](https://github.com/x52dev/tokio-nats/actions/workflows/ci.yml)

<!-- prettier-ignore-end -->

<!-- cargo-rdme start -->

A client for NATS using `tokio` and async-await.

There are still features missing, but it should be ready for use in simple situations.

## Installation

```toml
[dependencies]
tokio-nats = "0.4.2"
```

## Usage

```rust
use tokio_nats::{NatsConfigBuilder, connect};
use futures_util::StreamExt as _;

async fn demo() {
    let config = NatsConfigBuilder::default()
        .server("127.0.0.1:4222")
        .build()
        .unwrap();

     let client = connect(config).await.unwrap();

    client.publish("MySubject", "hello world".as_bytes()).await.unwrap();

    client.subscribe("MyOtherSubject").await.unwrap().for_each(|message| async move {
        println!("Received message {:?}", message);
    }).await;
}
```

<!-- cargo-rdme end -->
