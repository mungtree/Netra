//! [`BroadcastEventBus`] — a fan-out implementation of [`EventBus`].

use futures::StreamExt;
use futures::stream::BoxStream;
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;

use netra_core::traits::{DomainEvent, EventBus};

/// An [`EventBus`] backed by a `tokio` broadcast channel.
///
/// Every [`subscribe`](EventBus::subscribe) gets an independent stream. A slow
/// subscriber that overruns the channel capacity silently skips the events it
/// missed rather than stalling publishers.
#[derive(Debug, Clone)]
pub struct BroadcastEventBus {
    sender: broadcast::Sender<DomainEvent>,
}

impl BroadcastEventBus {
    /// Creates a bus whose channel buffers up to `capacity` events per
    /// subscriber (clamped to at least 1).
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity.max(1));
        Self { sender }
    }

    /// The number of live subscribers.
    #[must_use]
    pub fn subscriber_count(&self) -> usize {
        self.sender.receiver_count()
    }
}

impl Default for BroadcastEventBus {
    fn default() -> Self {
        Self::new(256)
    }
}

impl EventBus for BroadcastEventBus {
    fn publish(&self, event: DomainEvent) {
        // `send` errors only when there are no subscribers — safe to drop.
        let _ = self.sender.send(event);
    }

    fn subscribe(&self) -> BoxStream<'static, DomainEvent> {
        BroadcastStream::new(self.sender.subscribe())
            // Drop `Lagged` errors — a slow subscriber simply misses events.
            .filter_map(|result| async move { result.ok() })
            .boxed()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use netra_core::ids::JobId;
    use std::time::Duration;

    #[tokio::test]
    async fn delivers_published_events_to_subscribers() {
        let bus = BroadcastEventBus::new(16);
        let mut stream = bus.subscribe();

        let job_id = JobId::new();
        bus.publish(DomainEvent::JobStarted { job_id });

        let event = tokio::time::timeout(Duration::from_millis(200), stream.next())
            .await
            .expect("an event should arrive")
            .expect("stream should not end");
        assert!(matches!(event, DomainEvent::JobStarted { job_id: id } if id == job_id));
    }

    #[tokio::test]
    async fn every_subscriber_receives_each_event() {
        let bus = BroadcastEventBus::new(16);
        let mut first = bus.subscribe();
        let mut second = bus.subscribe();
        assert_eq!(bus.subscriber_count(), 2);

        bus.publish(DomainEvent::JobCompleted {
            job_id: JobId::new(),
        });

        assert!(first.next().await.is_some());
        assert!(second.next().await.is_some());
    }

    #[tokio::test]
    async fn publishing_without_subscribers_is_harmless() {
        let bus = BroadcastEventBus::default();
        bus.publish(DomainEvent::JobCompleted {
            job_id: JobId::new(),
        });
    }
}
