use crate::errors::Error;
use crate::protocol::{ClientInfo, ClientOp, NatsCodec, Op, ServerInfo, ServerOp};
use crate::subscriptions::SubscriptionManager;
use crate::NatsSubscription;
use bytes::Bytes;
use futures_util::{stream::iter, stream::select, SinkExt, StreamExt, TryFutureExt, TryStreamExt};
use parking_lot::Mutex;
use std::future::Future;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;
use tokio::codec::Framed;
use tokio::net::TcpStream;
use tokio::sync::mpsc::{channel, Sender};

/// A handle to a NATS connection, which allows subscribing and publishing messages.
///
/// Can be cloned, so that multiple `NatsClient`s can share a single connection.
#[derive(Clone)]
pub struct NatsClient {
    connection: Arc<NatsConnection>,
    send_queue: Sender<ClientOp>,
}

pub(crate) struct NatsConnection {
    pub(crate) config: NatsConfig,
    pub(crate) server_info: ServerInfo,
    pub(crate) subscription_manager: Mutex<SubscriptionManager>,
    pub(crate) control_sender: Mutex<Sender<ClientOp>>,
}

impl NatsClient {
    /// Publish a message over the associated NATS connection.
    ///
    /// The future will resolve as soon as the message has been successfully queued into the buffer,
    /// there is no guarantee that messages will be delivered in the case of connection failures.
    pub fn publish(
        &mut self,
        subject: String,
        message: Bytes,
    ) -> impl Future<Output = Result<(), Error>> + '_ {
        self.send_queue
            .send(ClientOp::Pub(subject, message))
            .map_err(|_| Error::ClientClosed)
    }

    /// Subscribe to a particular subject or pattern.
    ///
    /// Since NATS does not send acknowledgements for subscriptions, this function returns
    /// immediately and it is possible to miss messages sent soon after `subscribe` returns.
    pub async fn subscribe(&mut self, subject: String) -> Result<NatsSubscription, Error> {
        let (sender, receiver) = channel(self.connection.config.buffer_size);
        let sid = self
            .connection
            .subscription_manager
            .lock()
            .allocate_sid(sender);
        self.send_queue
            .send(ClientOp::Sub(subject, sid))
            .await
            .map_err(|_| Error::SendBufferFull)?;
        Ok(NatsSubscription {
            connection: self.connection.clone(),
            receiver,
            sid,
        })
    }
}

/// Configuration used in creating a NATS connection
#[derive(Builder, Debug, Clone)]
#[builder(setter(into))]
pub struct NatsConfig {
    /// The size of the queues used to both send and receive messages. Using a buffer too small will
    /// make `publish` block until there is capacity to add a new message to the send queue. It will
    /// also make subscriptions miss messages in the event of a slow consumer.
    #[builder(default = "5000")]
    buffer_size: usize,
    /// The host and port of the NATS server to connect to. E.g. `127.0.0.1:4222`
    server: String,
}

/// Make a new NATS connection. Return a `NatsClient` which can be cloned to obtain multiple handles
/// to the same connection.
pub async fn connect(config: NatsConfig) -> Result<NatsClient, Error> {
    let connection = TcpStream::connect(&SocketAddr::from_str(&config.server).unwrap()).await?;
    let mut framed = Framed::new(connection, NatsCodec::new());
    let first_op = framed.next().await.ok_or(Error::ProtocolError)??;
    let info = if let ServerOp::Info(info) = first_op {
        info
    } else {
        return Err(Error::ProtocolError);
    };
    framed
        .send(ClientOp::Connect(ClientInfo {
            verbose: false,
            pedantic: false,
            name: None,
            lang: "tokio-nats".to_string(),
            version: "0.1".to_string(),
        }))
        .await?;
    let (op_sender, op_receiver) = channel(config.buffer_size);

    let connection = Arc::new(NatsConnection {
        config,
        server_info: info,
        subscription_manager: Mutex::new(SubscriptionManager::new()),
        control_sender: Mutex::new(op_sender.clone()),
    });

    let (framed_write, framed_read) = framed.split();
    tokio::spawn(op_receiver.map(Result::Ok).forward(framed_write).unwrap_or_else(|err| println!("Error writing {:?}", err)));

    let mut server_response_sender = op_sender.clone();
    let connection_for_task = connection.clone();
    tokio::spawn(framed_read.map_err(Error::from).map(move |x| match x? {
            ServerOp::Ping => {
                let _ = server_response_sender.try_send(ClientOp::Pong);
                Ok(())
            }
            ServerOp::Msg(sid, message) => {
                if let Some(sender) = connection_for_task
                    .subscription_manager
                    .lock()
                    .sender_with_sid(sid)
                {
                    if let Err(err) = sender.try_send(message) {
                        if !err.is_closed() {
                            println!("Slow consumer :(") // TODO something better here
                        }
                    }
                }
                Ok(())
            }
            _ => Ok(())
        }).for_each(async move |c: Result<(), Error>| {
            if let Err(err) = c {
                println!("Error {:?}", err);
            }
        }));

    Ok(NatsClient {
        connection,
        send_queue: op_sender,
    })
}
