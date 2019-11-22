use crate::connection::NatsConnection;
use crate::protocol::ClientOp;
use bytes::Bytes;
use futures_util::stream::Stream;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::sync::mpsc::{Receiver, Sender};

pub(crate) struct SubscriptionManager {
    subscriptions: HashMap<usize, Sender<Bytes>>,
    next_id: usize,
}

impl SubscriptionManager {
    pub(crate) fn new() -> Self {
        SubscriptionManager {
            subscriptions: HashMap::new(),
            next_id: 0,
        }
    }

    pub(crate) fn allocate_sid(&mut self, sender: Sender<Bytes>) -> usize {
        let sid = self.next_id;
        self.next_id += 1;

        self.subscriptions.insert(sid, sender);
        sid
    }

    pub(crate) fn sender_with_sid(&mut self, sid: usize) -> Option<&mut Sender<Bytes>> {
        self.subscriptions.get_mut(&sid)
    }

    pub(crate) fn forget_sid(&mut self, sid: usize) -> bool {
        self.subscriptions.remove(&sid).is_some()
    }
}


/// A stream corresponding to a specific NATS subscription.
///
/// When dropped, an `UNSUB` message will be sent to the NATS server. It is possible to have
/// overlapping subscriptions through the same connection, in which case duplicate messages will be
/// sent from the server. In high throughput use cases it is therefore encouraged to subscribe at
/// most once to any subject.
///
/// Each subscription has a fixed size buffer with size specified in `NatsConfig`. If this queue
/// fills up messages will be dropped.
pub struct NatsSubscription {
    pub(crate) receiver: Receiver<Bytes>,
    pub(crate) connection: Arc<NatsConnection>,
    pub(crate) sid: usize,
}

impl Drop for NatsSubscription {
    fn drop(&mut self) {
        self.connection
            .subscription_manager
            .lock()
            .forget_sid(self.sid);
        let _ = self
            .connection
            .control_sender
            .lock()
            .try_send(ClientOp::Unsub(self.sid));
    }
}

impl Stream for NatsSubscription {
    type Item = Bytes;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.receiver.poll_recv(cx)
    }
}
