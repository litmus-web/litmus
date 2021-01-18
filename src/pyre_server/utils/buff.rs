use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use bytes::BytesMut;

use crossbeam::queue::ArrayQueue;
use std::sync::atomic::Ordering::Relaxed;


type Queue = Arc<ArrayQueue<BytesMut>>;
type MoreBody = Arc<AtomicBool>;


const MIN_BUFFER_SEND: usize = 64 * 1024;
const MAX_BUFFER_SEND: usize = 256 * 2024;


pub enum BufferStatus {
    /// There is data in the buffer and unwrapping this gives you the
    /// data inside.
    MoreBody(BytesMut),

    /// The buffer is empty.
    Empty,

    /// This is the last of the body
    LastBody(BytesMut),
}


pub fn make_buffer() -> (InMemoryWriter, InMemoryReader){
    let queue = ArrayQueue::<BytesMut>::new(8);
    let sharable_queue: Queue = Arc::new(queue);
    let shareable_more_body = Arc::new(AtomicBool::new(false));

    let reader = InMemoryReader {
        queue: sharable_queue.clone(),
        more_body: shareable_more_body.clone(),
    };

    let writer = InMemoryWriter {
        queue: sharable_queue,
        more_body: shareable_more_body,
        buffer: BytesMut::with_capacity(MAX_BUFFER_SEND)
    };

    (writer, reader)
}


pub struct InMemoryReader {
    queue: Queue,
    more_body: MoreBody,
}

impl InMemoryReader {
    pub fn read_chunk(&self) -> BufferStatus {
        if let Some(data) = self.queue.pop() {
            let is_done = self.more_body.load(Relaxed);

            return if is_done {
                BufferStatus::LastBody(data)
            } else {
                BufferStatus::MoreBody(data)
            }
        }

        BufferStatus::Empty
    }
}

pub struct InMemoryWriter {
    queue: Queue,
    more_body: MoreBody,
    buffer: BytesMut,
}

impl InMemoryWriter {
    pub fn write(&mut self, data: BytesMut, more_body: bool) -> Result<(), BytesMut> {
        self.buffer += data;

        let len = self.buffer.len();

        if len > MAX_BUFFER_SEND {
            self.queue.push(self.buffer.split_to(MAX_BUFFER_SEND))?;
        } else if len > MIN_BUFFER_SEND {
            self.queue.push(self.buffer.split_off(0))?;
        } else {
            return Ok(())
        }

        if !more_body {
            self.more_body.store(more_body, Relaxed);
        }

        Ok(())
    }
}