#![feature(async_closure)]

use bytes::Bytes;
use futures_util::StreamExt;
use std::net::SocketAddr;
use std::str::FromStr;
use std::time::Duration;
use tokio_nats::connect;

#[tokio::main()]
async fn main() -> std::io::Result<()> {
    //    let mut client = connect(&SocketAddr::from_str("127.0.0.1:4222").unwrap()).await?;
    let mut client = unimplemented!();
    let subscription = client.subscribe("TEST".to_string()).await?;

    let mut x = 5;
    tokio::spawn(subscription.take(5).for_each(async move |message| {
        println!(
            "Received message {}",
            std::str::from_utf8(&message[..]).unwrap()
        );
    }));

    loop {
        std::thread::sleep(Duration::from_millis(1000));
        connection
            .publish("TEST".to_string(), Bytes::from_static(b"hello world"))
            .await?;
    }
    Ok(())
}
