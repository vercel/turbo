use std::{
    borrow::Cow,
    io::{self, Write},
    sync::{Arc, Mutex, RwLock},
};

/// OutputSink represent a sink for outputs that can be written to from multiple
/// threads through the use of Loggers.
pub struct OutputSink<W> {
    writers: Arc<Mutex<SinkWriters<W>>>,
}

struct SinkWriters<W> {
    out: W,
    err: W,
}

/// OutputClient allows for multiple threads to write to the same OutputSink
pub struct OutputClient<W> {
    behavior: OutputClientBehavior,
    // We could use a RefCell if we didn't use this with async code.
    // Any locals held across an await must implement Sync and RwLock lets us achieve this
    buffer: Option<RwLock<Vec<SinkBytes<'static>>>>,
    writers: Arc<Mutex<SinkWriters<W>>>,
}

pub struct OutputWriter<'a, W> {
    logger: &'a OutputClient<W>,
    destination: Destination,
}

/// Enum for controlling the behavior of the client
#[derive(Debug, Clone, Copy)]
pub enum OutputClientBehavior {
    /// Every line sent to the client will get immediately sent to the sink
    Passthrough,
    /// Every line sent to the client will get immediately sent to the sink,
    /// but a buffer will be built up as well and returned when finish is called
    InMemoryBuffer,
    // Every line sent to the client will get tracked in the buffer only being
    // sent to the sink once finish is called.
    Grouped,
}

#[derive(Debug, Clone, Copy)]
enum Destination {
    Stdout,
    Stderr,
}

#[derive(Debug, Clone)]
struct SinkBytes<'a> {
    buffer: Cow<'a, [u8]>,
    destination: Destination,
}

impl<W: Write> OutputSink<W> {
    /// Produces a new sink with the corresponding out and err writers
    pub fn new(out: W, err: W) -> Self {
        Self {
            writers: Arc::new(Mutex::new(SinkWriters { out, err })),
        }
    }

    /// Produces a new client that will send all bytes that it receives to the
    /// underlying sink. Behavior of how these bytes are sent is controlled
    /// by the behavior parameter. Note that OutputClient intentionally doesn't
    /// implement Sync as if you want to write to the same sink
    /// from multiple threads, then you should create a logger for each thread.
    pub fn logger(&self, behavior: OutputClientBehavior) -> OutputClient<W> {
        let buffer = match behavior {
            OutputClientBehavior::Passthrough => None,
            OutputClientBehavior::InMemoryBuffer | OutputClientBehavior::Grouped => {
                Some(Default::default())
            }
        };
        let writers = self.writers.clone();
        OutputClient {
            behavior,
            buffer,
            writers,
        }
    }
}

impl<W: Write> OutputClient<W> {
    /// A writer that will write to the underlying sink's out writer according
    /// to this client's behavior.
    pub fn stdout(&self) -> OutputWriter<W> {
        OutputWriter {
            logger: self,
            destination: Destination::Stdout,
        }
    }

    /// A writer that will write to the underlying sink's err writer according
    /// to this client's behavior.
    pub fn stderr(&self) -> OutputWriter<W> {
        OutputWriter {
            logger: self,
            destination: Destination::Stderr,
        }
    }

    /// Consume the client and flush any bytes to the underlying sink if
    /// necessary
    pub fn finish(self) -> io::Result<Option<Vec<u8>>> {
        let Self {
            behavior,
            buffer,
            writers,
        } = self;
        let buffers = buffer.map(|cell| cell.into_inner().expect("lock poisoned"));

        if matches!(behavior, OutputClientBehavior::Grouped) {
            let buffers = buffers
                .as_ref()
                .expect("grouped logging requires buffer to be present");
            // We hold the mutex until we write all of the bytes associated for the client
            // to ensure that the bytes aren't interspersed.
            let mut writers = writers.lock().expect("lock poisoned");
            for SinkBytes {
                buffer,
                destination,
            } in buffers
            {
                let writer = match destination {
                    Destination::Stdout => &mut writers.out,
                    Destination::Stderr => &mut writers.err,
                };
                writer.write_all(buffer)?;
            }
        }

        Ok(buffers.map(|buffers| {
            // TODO: it might be worth the list traversal to calculate length so we do a
            // single allocation
            let mut bytes = Vec::new();
            for SinkBytes { buffer, .. } in buffers {
                bytes.extend_from_slice(&buffer[..]);
            }
            bytes
        }))
    }

    fn handle_bytes(&self, bytes: SinkBytes) -> io::Result<usize> {
        if matches!(
            self.behavior,
            OutputClientBehavior::InMemoryBuffer | OutputClientBehavior::Grouped
        ) {
            // This reconstruction is necessary to change the type of bytes from
            // SinkBytes<'a> to SinkBytes<'static>
            let bytes = SinkBytes {
                destination: bytes.destination,
                buffer: bytes.buffer.to_vec().into(),
            };
            self.add_bytes_to_buffer(bytes);
        }
        if matches!(
            self.behavior,
            OutputClientBehavior::Passthrough | OutputClientBehavior::InMemoryBuffer
        ) {
            self.write_bytes(bytes)
        } else {
            // If we only wrote to the buffer, then we consider it a successful write
            Ok(bytes.buffer.len())
        }
    }

    fn write_bytes(&self, bytes: SinkBytes) -> io::Result<usize> {
        let SinkBytes {
            buffer: line,
            destination,
        } = bytes;
        let mut writers = self.writers.lock().expect("writer lock poisoned");
        let writer = match destination {
            Destination::Stdout => &mut writers.out,
            Destination::Stderr => &mut writers.err,
        };
        writer.write(&line)
    }

    fn add_bytes_to_buffer(&self, bytes: SinkBytes<'static>) {
        let buffer = self
            .buffer
            .as_ref()
            .expect("attempted to add line to nil buffer");
        buffer.write().expect("lock poisoned").push(bytes);
    }
}

impl<'a, W: Write> Write for OutputWriter<'a, W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.logger.handle_bytes(SinkBytes {
            buffer: buf.into(),
            destination: self.destination,
        })
    }

    fn flush(&mut self) -> io::Result<()> {
        // No buffer held by the logger writer so flush is a noop
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_loggers_from_multiple_threads() {
        let sink = OutputSink::new(Vec::new(), Vec::new());
        let pass_thru_logger = sink.logger(OutputClientBehavior::Passthrough);
        let buffer_logger = sink.logger(OutputClientBehavior::InMemoryBuffer);
        std::thread::scope(|s| {
            s.spawn(move || {
                let mut out = pass_thru_logger.stdout();
                let mut err = pass_thru_logger.stderr();
                writeln!(&mut out, "task 1: out").unwrap();
                writeln!(&mut err, "task 1: err").unwrap();
                assert!(pass_thru_logger.finish().unwrap().is_none());
            });
            s.spawn(move || {
                let mut out = buffer_logger.stdout();
                let mut err = buffer_logger.stderr();
                writeln!(&mut out, "task 2: out").unwrap();
                writeln!(&mut err, "task 2: err").unwrap();
                assert_eq!(
                    buffer_logger.finish().unwrap().unwrap(),
                    b"task 2: out\ntask 2: err\n"
                );
            });
        });
        let SinkWriters { out, err } = Arc::into_inner(sink.writers).unwrap().into_inner().unwrap();
        let out = String::from_utf8(out).unwrap();
        let err = String::from_utf8(err).unwrap();
        for line in out.lines() {
            assert!(line.ends_with(": out"));
        }
        for line in err.lines() {
            assert!(line.ends_with(": err"));
        }
    }

    #[test]
    fn test_pass_thru() -> io::Result<()> {
        let sink = OutputSink::new(Vec::new(), Vec::new());
        let logger = sink.logger(OutputClientBehavior::Passthrough);

        let mut out = logger.stdout();

        writeln!(&mut out, "output for 1")?;
        assert_eq!(
            sink.writers.lock().unwrap().out.as_slice(),
            b"output for 1\n",
            "pass thru should end up in sink immediately"
        );
        assert!(
            logger.finish()?.is_none(),
            "pass through logs shouldn't keep a buffer"
        );
        assert_eq!(
            sink.writers.lock().unwrap().out.as_slice(),
            b"output for 1\n",
            "pass thru shouldn't alter sink on finish"
        );

        Ok(())
    }

    #[test]
    fn test_buffer() -> io::Result<()> {
        let sink = OutputSink::new(Vec::new(), Vec::new());
        let logger = sink.logger(OutputClientBehavior::InMemoryBuffer);

        let mut out = logger.stdout();

        writeln!(&mut out, "output for 1")?;
        assert_eq!(
            sink.writers.lock().unwrap().out.as_slice(),
            b"output for 1\n",
            "buffer should end up in sink immediately"
        );
        assert_eq!(
            logger.finish()?.unwrap(),
            b"output for 1\n",
            "buffer should return buffer"
        );
        assert_eq!(
            sink.writers.lock().unwrap().out.as_slice(),
            b"output for 1\n",
            "buffer shouldn't alter sink on finish"
        );

        Ok(())
    }

    #[test]
    fn test_grouped_logs() -> io::Result<()> {
        let sink = OutputSink::new(Vec::new(), Vec::new());
        let group1_logger = sink.logger(OutputClientBehavior::Grouped);
        let group2_logger = sink.logger(OutputClientBehavior::Grouped);

        let mut group1_out = group1_logger.stdout();
        let mut group2_out = group2_logger.stdout();
        let mut group2_err = group2_logger.stderr();

        writeln!(&mut group2_out, "output for 2")?;
        writeln!(&mut group1_out, "output for 1")?;
        let group1_logs = group1_logger
            .finish()?
            .expect("grouped logs should have buffer");
        writeln!(&mut group2_err, "warning for 2")?;
        let group2_logs = group2_logger
            .finish()?
            .expect("grouped logs should have buffer");

        assert_eq!(group1_logs, b"output for 1\n");
        assert_eq!(group2_logs, b"output for 2\nwarning for 2\n");

        let SinkWriters { out, err } = Arc::into_inner(sink.writers).unwrap().into_inner().unwrap();
        assert_eq!(out, b"output for 1\noutput for 2\n");
        assert_eq!(err, b"warning for 2\n");

        Ok(())
    }

    #[test]
    fn assert_output_writer_sync() {
        // This is the bound required for a value to be held across an await
        fn hold_across_await<T: Send>() {}
        hold_across_await::<&mut OutputWriter<'static, Vec<u8>>>();
    }
}
