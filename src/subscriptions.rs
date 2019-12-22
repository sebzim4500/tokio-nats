use crate::connection::NatsClientInner;
use crate::protocol::ClientOp;

use crate::NatsMessage;
use futures_util::stream::Stream;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::sync::mpsc::{Receiver, Sender};

struct Subscription {
    topic: String,
    sender: Sender<NatsMessage>,
}

pub(crate) struct SubscriptionManager {
    subscriptions: HashMap<usize, Subscription>,
    next_id: usize,
}

impl SubscriptionManager {
    pub(crate) fn new() -> Self {
        SubscriptionManager {
            subscriptions: HashMap::new(),
            next_id: 0,
        }
    }

    pub(crate) fn allocate_sid(&mut self, topic: String, sender: Sender<NatsMessage>) -> usize {
        let sid = self.next_id;
        self.next_id += 1;

        self.subscriptions
            .insert(sid, Subscription { topic, sender });
        sid
    }

    pub(crate) fn sender_with_sid(&mut self, sid: usize) -> Option<&mut Sender<NatsMessage>> {
        self.subscriptions.get_mut(&sid).map(|sub| &mut sub.sender)
    }

    pub(crate) fn forget_sid(&mut self, sid: usize) -> bool {
        self.subscriptions.remove(&sid).is_some()
    }

    pub(crate) fn all_subscriptions(&self) -> Vec<(usize, String)> {
        self.subscriptions
            .iter()
            .map(|(&sid, sub)| (sid, sub.topic.clone()))
            .collect()
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
    pub(crate) receiver: Receiver<NatsMessage>,
    pub(crate) connection: Arc<NatsClientInner>,
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
    type Item = NatsMessage;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.receiver.poll_recv(cx)
    }
}
