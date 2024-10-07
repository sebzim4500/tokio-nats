//! [![Crates.io](https://img.shields.io/crates/v/tokio-nats.svg)](https://crates.io/crates/tokio-nats)
//! [![docs.rs](https://docs.rs/tokio-nats/badge.svg)](https://docs.rs/tokio-nats)
//! A client for NATS using `tokio` and async-await.
//!
//! There are still features missing, but it should be ready for use in simple situations.
//!
//! ## Installation
//!
//! ```toml
//! [dependencies]
//! tokio-nats = "0.4.1"
//! ```
//! ## Usage
//! ```rust
//!
//! use tokio_nats::{NatsConfigBuilder, connect};
//! use futures_util::StreamExt;
//!
//! async fn demo() {
//!     let config = NatsConfigBuilder::default()
//!         .server("127.0.0.1:4222")
//!         .build()
//!         .unwrap();
//!     let mut client = connect(config).await.unwrap();
//!
//!     client.publish("MySubject", "hello world".as_bytes()).await.unwrap();
//!
//!     client.subscribe("MyOtherSubject").await.unwrap().for_each(|message| async move {
//!         println!("Received message {:?}", message);
//!     }).await;
//! }
//! ```

#[macro_use]
extern crate serde;
#[macro_use]
extern crate derive_builder;

mod connection;
mod errors;
mod protocol;
mod subscriptions;
mod tls;

use bytes::Bytes;
pub use connection::{connect, NatsClient, NatsConfig, NatsConfigBuilder};
pub use errors::Error;
pub use subscriptions::NatsSubscription;
pub use tls::{TLSConnBuild, TLSConnBuildError, TlsConnParams};

/// A message that has been received by the NATS client.
#[derive(Debug, Clone)]
pub struct NatsMessage {
    pub subject: String,
    pub payload: Bytes,
}
