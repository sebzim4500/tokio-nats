#![feature(async_closure)]

use bytes::Bytes;
use futures_util::StreamExt;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH, Duration};
use tokio_nats::{connect, NatsConfigBuilder};
use env_logger;

#[tokio::main()]
async fn main() -> Result<(), tokio_nats::Error> {
    env_logger::init();
    let config = NatsConfigBuilder::default()
        .server("127.0.0.1:4222")
        .build()
        .unwrap();
    let mut pub_client = connect(config.clone()).await?;
    let counter = Arc::new(AtomicUsize::new(0));
    let mut sub_client = connect(config).await?;
    let subscription = sub_client.subscribe("TIMES".to_string()).await?;
    let start_time = Instant::now();

    tokio::spawn(
        subscription
            .map(move |message| {
                let nanos = Instant::now()
                    .duration_since(start_time)
                    .as_nanos();
                let time_sent = std::str::from_utf8(&message[..])
                    .unwrap()
                    .parse::<u128>()
                    .unwrap();

                if counter.fetch_add(1, Ordering::SeqCst) % 500 == 0 {
                    println!("{:?}: Latency = {}", Instant::now(), nanos - time_sent);
                }
            })
            .for_each(async move |()| {}),
    );

    loop {
        std::thread::sleep(Duration::from_millis(1));
        let nanos = Instant::now()
            .duration_since(start_time)
            .as_nanos();
        let bytes = Bytes::from(format!("{}", nanos).as_bytes().to_vec());
        for i in 0 .. 5 {
            pub_client.publish("TIMES".to_string(), bytes.clone()).await?;
        }
    }
}
