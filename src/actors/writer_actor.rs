// Copyright 2026 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! WriterActor — dedicated WebSocket write actor.
//!
//! Receives serialized protobuf bytes via an `mpsc` channel and forwards
//! them to the WebSocket sink. This actor is intentionally minimal:
//! it owns only the write half of the channel and the WS sink.
//!
//! # Zero-lock guarantee
//!
//! Because the actor runs in a single `tokio::spawn` task and owns its
//! state exclusively, no `Mutex` or `RwLock` is needed.

use tokio::sync::mpsc;
use tracing::warn;

/// Messages that the WriterActor processes.
#[derive(Debug)]
pub enum WriteMsg {
    /// Send raw bytes to the WebSocket.
    Send(Vec<u8>),
    /// Gracefully shut down the writer.
    Shutdown,
}

/// A dedicated actor for writing bytes to a channel/sink.
///
/// In production, the `sink` would be a WebSocket `SplitSink`.
/// Here we abstract it as an `mpsc::UnboundedSender<Vec<u8>>` for testability.
pub struct WriterActor {
    /// Incoming messages from other actors/components.
    rx: mpsc::UnboundedReceiver<WriteMsg>,
    /// Outbound sink (WebSocket write channel or test mock).
    sink: mpsc::UnboundedSender<Vec<u8>>,
}

impl WriterActor {
    /// Creates a new WriterActor.
    pub fn new(
        rx: mpsc::UnboundedReceiver<WriteMsg>,
        sink: mpsc::UnboundedSender<Vec<u8>>,
    ) -> Self {
        Self { rx, sink }
    }

    /// Runs the actor event loop.
    ///
    /// Processes messages sequentially. Returns when the channel is closed
    /// or a `Shutdown` message is received.
    pub async fn run(mut self) {
        while let Some(msg) = self.rx.recv().await {
            match msg {
                WriteMsg::Send(bytes) => {
                    if self.sink.send(bytes).is_err() {
                        warn!("WriterActor: sink closed, stopping");
                        break;
                    }
                }
                WriteMsg::Shutdown => {
                    break;
                }
            }
        }
    }

    /// Returns the number of pending messages (for diagnostics).
    /// Only meaningful before `run()` is called.
    pub fn pending_count(&self) -> usize {
        // UnboundedReceiver doesn't expose len, but we can check if empty
        0 // Placeholder — tokio doesn't expose queue depth for unbounded
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_writer_actor_sends_bytes() {
        let (msg_tx, msg_rx) = mpsc::unbounded_channel();
        let (sink_tx, mut sink_rx) = mpsc::unbounded_channel();

        let actor = WriterActor::new(msg_rx, sink_tx);
        let handle = tokio::spawn(actor.run());

        // Send a message
        msg_tx.send(WriteMsg::Send(vec![1, 2, 3])).unwrap();
        msg_tx.send(WriteMsg::Send(vec![4, 5])).unwrap();
        msg_tx.send(WriteMsg::Shutdown).unwrap();

        handle.await.unwrap();

        // Verify messages arrived at sink
        let msg1 = sink_rx.recv().await.unwrap();
        assert_eq!(msg1, vec![1, 2, 3]);
        let msg2 = sink_rx.recv().await.unwrap();
        assert_eq!(msg2, vec![4, 5]);
    }

    #[tokio::test]
    async fn test_writer_actor_shutdown_stops() {
        let (msg_tx, msg_rx) = mpsc::unbounded_channel();
        let (sink_tx, _sink_rx) = mpsc::unbounded_channel();

        let actor = WriterActor::new(msg_rx, sink_tx);
        let handle = tokio::spawn(actor.run());

        msg_tx.send(WriteMsg::Shutdown).unwrap();
        // Should finish cleanly
        handle.await.unwrap();
    }

    #[tokio::test]
    async fn test_writer_actor_channel_close_stops() {
        let (msg_tx, msg_rx) = mpsc::unbounded_channel();
        let (sink_tx, _sink_rx) = mpsc::unbounded_channel();

        let actor = WriterActor::new(msg_rx, sink_tx);
        let handle = tokio::spawn(actor.run());

        // Drop sender — closes channel
        drop(msg_tx);
        handle.await.unwrap();
    }

    #[tokio::test]
    async fn test_writer_actor_sink_closed_stops() {
        let (msg_tx, msg_rx) = mpsc::unbounded_channel();
        let (sink_tx, sink_rx) = mpsc::unbounded_channel();

        let actor = WriterActor::new(msg_rx, sink_tx);
        let handle = tokio::spawn(actor.run());

        // Drop the sink receiver — simulates WS disconnect
        drop(sink_rx);
        msg_tx.send(WriteMsg::Send(vec![1])).unwrap();
        // Actor should detect closed sink and stop
        handle.await.unwrap();
    }

    #[tokio::test]
    async fn test_writer_actor_multiple_messages_order() {
        let (msg_tx, msg_rx) = mpsc::unbounded_channel();
        let (sink_tx, mut sink_rx) = mpsc::unbounded_channel();

        let actor = WriterActor::new(msg_rx, sink_tx);
        let handle = tokio::spawn(actor.run());

        for i in 0u8..10 {
            msg_tx.send(WriteMsg::Send(vec![i])).unwrap();
        }
        msg_tx.send(WriteMsg::Shutdown).unwrap();
        handle.await.unwrap();

        // Verify order preserved
        for i in 0u8..10 {
            let msg = sink_rx.recv().await.unwrap();
            assert_eq!(msg, vec![i]);
        }
    }
}
