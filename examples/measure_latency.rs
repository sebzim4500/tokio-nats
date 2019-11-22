#![feature(async_closure)]

use bytes::Bytes;
use futures_util::StreamExt;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use tokio_nats::{connect, NatsConfigBuilder};

#[tokio::main()]
async fn main() -> Result<(), tokio_nats::Error> {
    let config = NatsConfigBuilder::default()
        .server("127.0.0.1:4222")
        .build()
        .unwrap();
    let mut pub_client = connect(config.clone()).await?;
    let counter = Arc::new(AtomicUsize::new(0));
    let mut sub_client = connect(config).await?;
    let subscription = sub_client.subscribe("TIMES".to_string()).await?;

    tokio::spawn(
        subscription
            .map(move |message| {
                let nanos = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos();
                let time_sent = std::str::from_utf8(&message[..])
                    .unwrap()
                    .parse::<u128>()
                    .unwrap();

                if counter.fetch_add(1, Ordering::SeqCst) % 100_000 == 0 {
                    println!("{:?}: Latency = {}", Instant::now(), nanos - time_sent);
                }
            })
            .for_each(async move |()| {}),
    );

    loop {
        //        std::thread::sleep(Duration::from_millis(1));
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let bytes = Bytes::from(format!("{}", nanos).as_bytes());
        pub_client.publish("TIMES".to_string(), bytes).await?;
    }
}
