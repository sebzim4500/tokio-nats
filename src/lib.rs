#![feature(async_closure)]

// TODO docs

#[macro_use]
extern crate serde;
#[macro_use]
extern crate derive_builder;

mod connection;
mod errors;
mod protocol;
mod subscriptions;

use bytes::Bytes;
pub use connection::{connect, NatsClient, NatsConfig, NatsConfigBuilder};
pub use errors::Error;
pub use subscriptions::NatsSubscription;

#[derive(Debug, Clone)]
pub struct NatsMessage {
    pub subject: String,
    pub payload: Bytes,
}
