use crate::network::Network;
use crate::protocol::Envelope;
use rand::Rng;
use std::sync::Mutex;
use std::time::Duration;

pub struct FaultyNetwork<N: Network> {
    pub inner: N,
    pub drop_rate: f64,              // 0.0 to 1.0
    pub delay_range_ms: (u64, u64),  // (min, max) delay
    pub duplicate_rate: f64,         // 0.0 to 1.0
    pub reorder_buffer: Mutex<Vec<Envelope>>,
}

impl<N: Network> FaultyNetwork<N> {
    pub fn new(
        inner: N,
        drop_rate: f64,
        delay_range_ms: (u64, u64),
        duplicate_rate: f64,
        _reorder_buffer_cap: usize,
    ) -> Self {
        Self {
            inner,
            drop_rate,
            delay_range_ms,
            duplicate_rate,
            reorder_buffer: Mutex::new(Vec::new()),
        }
    }
}

#[async_trait::async_trait]
impl<N: Network> Network for FaultyNetwork<N> {
    async fn broadcast(&self, envelope: Envelope) {
        // Pre-compute decisions so ThreadRng is dropped before any .await
        let (should_drop, delay_ms, should_duplicate) = {
            let mut rng = rand::rng();
            let should_drop = rng.random_bool(self.drop_rate.clamp(0.0, 1.0));
            let (min_delay, max_delay) = self.delay_range_ms;
            let delay_ms = if max_delay > 0 {
                if min_delay >= max_delay {
                    min_delay
                } else {
                    rng.random_range(min_delay..=max_delay)
                }
            } else {
                0
            };
            let should_duplicate = rng.random_bool(self.duplicate_rate.clamp(0.0, 1.0));
            (should_drop, delay_ms, should_duplicate)
        };

        // 1. Packet drop check
        if should_drop {
            return;
        }

        // 2. Artificial latency
        if delay_ms > 0 {
            tokio::time::sleep(Duration::from_millis(delay_ms)).await;
        }

        // 3. Normal transmission
        self.inner.broadcast(envelope.clone()).await;

        // 4. Duplicate packet check
        if should_duplicate {
            self.inner.broadcast(envelope).await;
        }
    }

    async fn recv(&self) -> Option<Envelope> {
        self.inner.recv().await
    }

    async fn drain(&self) -> Vec<Envelope> {
        self.inner.drain().await
    }
}
