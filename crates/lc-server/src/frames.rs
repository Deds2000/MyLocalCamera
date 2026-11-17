//! Fonte limitada de quadros H.264, independente dos eventos de UI.

use std::collections::{BTreeSet, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use bytes::Bytes;
use tokio::sync::Notify;

/// Limite de quadros pendentes por servidor (no máximo 32 MiB pelo limite
/// de payload do protocolo).
const FRAME_QUEUE_CAPACITY: usize = 4;

/// Quadro recebido do celular.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoFrame {
    /// Sessão que originou o quadro.
    pub session_id: String,
    /// Stream de vídeo do protocolo.
    pub stream: u8,
    /// Flags do cabeçalho, incluindo `KEYFRAME`.
    pub flags: u8,
    /// Timestamp de apresentação em microssegundos.
    pub pts_us: u64,
    /// Access unit H.264 Annex-B.
    pub data: Bytes,
}

/// Indicação de que a fila descartou dados e o consumidor deve pedir um
/// keyframe para essa sessão e stream.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct KeyframeRequest {
    pub session_id: String,
    pub stream: u8,
}

#[derive(Debug, Default)]
struct QueueState {
    session_id: Option<String>,
    frames: VecDeque<VideoFrame>,
    keyframe_needed: BTreeSet<KeyframeRequest>,
}

/// Canal interno de quadros. A inserção é síncrona e nunca espera pelo
/// consumidor, portanto não bloqueia a leitura do socket.
#[derive(Debug)]
pub(crate) struct VideoFrameQueue {
    state: Mutex<QueueState>,
    ready: Notify,
    dropped_frames: AtomicU64,
}

impl VideoFrameQueue {
    pub(crate) fn new() -> Arc<Self> {
        Arc::new(Self {
            state: Mutex::new(QueueState::default()),
            ready: Notify::new(),
            dropped_frames: AtomicU64::new(0),
        })
    }

    pub(crate) fn dropped_frames(&self) -> u64 {
        self.dropped_frames.load(Ordering::Relaxed)
    }

    pub(crate) fn source(self: &Arc<Self>) -> VideoFrameSource {
        VideoFrameSource {
            queue: self.clone(),
        }
    }

    pub(crate) fn push(&self, frame: VideoFrame) {
        let mut state = self.state.lock().unwrap_or_else(|p| p.into_inner());
        if state.session_id.as_deref() != Some(frame.session_id.as_str()) {
            state.frames.clear();
            state.keyframe_needed.clear();
            state.session_id = Some(frame.session_id.clone());
        }

        if state.frames.len() == FRAME_QUEUE_CAPACITY {
            let oldest_non_key = state
                .frames
                .iter()
                .position(|queued| queued.flags & lc_protocol::flags::KEYFRAME == 0);
            if let Some(index) = oldest_non_key {
                let dropped = state.frames.remove(index).expect("índice encontrado acima");
                self.dropped_frames.fetch_add(1, Ordering::Relaxed);
                state.keyframe_needed.insert(KeyframeRequest {
                    session_id: dropped.session_id,
                    stream: dropped.stream,
                });
            } else {
                self.dropped_frames.fetch_add(1, Ordering::Relaxed);
                state.keyframe_needed.insert(KeyframeRequest {
                    session_id: frame.session_id,
                    stream: frame.stream,
                });
                return;
            }
        }

        state.frames.push_back(frame);
        drop(state);
        self.ready.notify_one();
    }

    fn pop(&self) -> Option<VideoFrame> {
        self.state
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .frames
            .pop_front()
    }

    fn take_keyframe_requests(&self) -> Vec<KeyframeRequest> {
        let mut state = self.state.lock().unwrap_or_else(|p| p.into_inner());
        std::mem::take(&mut state.keyframe_needed)
            .into_iter()
            .collect()
    }
}

/// Consumidor da fonte de quadros. Use um consumidor por servidor.
#[derive(Debug, Clone)]
pub struct VideoFrameSource {
    queue: Arc<VideoFrameQueue>,
}

impl VideoFrameSource {
    /// Aguarda o próximo quadro. Quadros descartados por pressão não são
    /// devolvidos; consulte [`Self::take_keyframe_requests`] para recuperar.
    pub async fn recv(&mut self) -> VideoFrame {
        loop {
            let notified = self.queue.ready.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            if let Some(frame) = self.queue.pop() {
                return frame;
            }
            notified.await;
        }
    }

    /// Retira as indicações acumuladas de keyframe necessário.
    pub fn take_keyframe_requests(&mut self) -> Vec<KeyframeRequest> {
        self.queue.take_keyframe_requests()
    }
}

#[cfg(test)]
mod tests {
    use super::{KeyframeRequest, VideoFrame, VideoFrameQueue};
    use bytes::Bytes;

    fn frame(pts_us: u64, flags: u8) -> VideoFrame {
        VideoFrame {
            session_id: "session-a".to_owned(),
            stream: 1,
            flags,
            pts_us,
            data: Bytes::from(vec![u8::try_from(pts_us).expect("test PTS cabe")]),
        }
    }

    #[test]
    fn pressure_discards_oldest_non_key_and_marks_stream() {
        let queue = VideoFrameQueue::new();
        for pts in 0..4 {
            queue.push(frame(pts, 0));
        }
        queue.push(frame(4, 0));

        let mut source = queue.source();
        assert_eq!(
            source.take_keyframe_requests(),
            vec![KeyframeRequest {
                session_id: "session-a".to_owned(),
                stream: 1,
            }]
        );
        assert_eq!(queue.pop().expect("frame").pts_us, 1);
        assert_eq!(queue.pop().expect("frame").pts_us, 2);
        assert_eq!(queue.pop().expect("frame").pts_us, 3);
        assert_eq!(queue.pop().expect("frame").pts_us, 4);
        assert_eq!(queue.dropped_frames(), 1);
    }

    #[test]
    fn pressure_counts_incoming_frame_when_queue_contains_only_keyframes() {
        let queue = VideoFrameQueue::new();
        for pts in 0..4 {
            queue.push(frame(pts, lc_protocol::flags::KEYFRAME));
        }
        queue.push(frame(4, 0));

        assert_eq!(queue.dropped_frames(), 1);
        assert_eq!(queue.pop().expect("frame").pts_us, 0);
    }
}
