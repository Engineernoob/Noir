use std::{
    io::{self, BufRead, BufReader, Write},
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
    sync::mpsc::{self, Receiver},
    thread,
};

use anyhow::{Context, Result};
use serde::Serialize;
use serde_json::Value;

use crate::lsp::protocol::{IncomingMessage, parse_incoming_message};

pub enum TransportEvent {
    Message(IncomingMessage),
    ReadError(String),
    Closed,
}

/// Owns the child process and its stdin/stdout pipes.
///
/// A background thread reads Content-Length–framed JSON-RPC messages from the
/// server's stdout and forwards them through an mpsc channel. The main thread
/// writes to stdin via [`send`](Self::send) and drains the channel via
/// [`try_recv`](Self::try_recv).
pub struct StdioTransport {
    stdin: ChildStdin,
    events: Receiver<TransportEvent>,
    child: Child,
}

impl StdioTransport {
    /// Spawn `command` with `args` and connect its stdin/stdout for JSON-RPC.
    pub fn start(command: &str, args: &[String]) -> Result<Self> {
        let mut child = Command::new(command)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .with_context(|| format!("failed to spawn LSP server '{command}'"))?;

        let stdin = child
            .stdin
            .take()
            .context("failed to capture LSP server stdin")?;
        let stdout = child
            .stdout
            .take()
            .context("failed to capture LSP server stdout")?;

        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let mut reader = BufReader::new(stdout);

            loop {
                match read_message(&mut reader) {
                    Ok(Some(value)) => match parse_incoming_message(value) {
                        Ok(message) => {
                            if tx.send(TransportEvent::Message(message)).is_err() {
                                break;
                            }
                        }
                        Err(err) => {
                            let _ = tx.send(TransportEvent::ReadError(err.to_string()));
                            break;
                        }
                    },
                    Ok(None) => {
                        let _ = tx.send(TransportEvent::Closed);
                        break;
                    }
                    Err(err) => {
                        let _ = tx.send(TransportEvent::ReadError(err.to_string()));
                        break;
                    }
                }
            }
        });

        Ok(Self {
            stdin,
            events: rx,
            child,
        })
    }

    /// Serialize `message` as JSON and write it with a `Content-Length` header.
    pub fn send<T: Serialize>(&mut self, message: &T) -> Result<()> {
        write_message(&mut self.stdin, message).context("failed to write JSON-RPC message")
    }

    /// Returns the next pending event without blocking, or `None`.
    pub fn try_recv(&self) -> Option<TransportEvent> {
        self.events.try_recv().ok()
    }
}

impl Drop for StdioTransport {
    fn drop(&mut self) {
        if matches!(self.child.try_wait(), Ok(None)) {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

/// Read one Content-Length–framed JSON value from `reader`.
/// Returns `Ok(None)` when the stream is cleanly closed.
fn read_message(reader: &mut BufReader<ChildStdout>) -> io::Result<Option<Value>> {
    let mut content_length: Option<usize> = None;

    loop {
        let mut header = String::new();
        if reader.read_line(&mut header)? == 0 {
            return Ok(None); // EOF
        }

        if header == "\r\n" || header == "\n" {
            break;
        }

        if let Some((name, value)) = header.split_once(':') {
            if name.eq_ignore_ascii_case("Content-Length") {
                let len = value.trim().parse::<usize>().map_err(|e| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("invalid Content-Length: {e}"),
                    )
                })?;
                content_length = Some(len);
            }
        }
    }

    let length = content_length.ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidData, "missing Content-Length header")
    })?;

    let mut body = vec![0u8; length];
    io::Read::read_exact(reader, &mut body)?;

    serde_json::from_slice(&body).map(Some).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("invalid JSON-RPC body: {e}"),
        )
    })
}

fn write_message(writer: &mut impl Write, message: &impl Serialize) -> io::Result<()> {
    let body = serde_json::to_vec(message).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("failed to serialize JSON-RPC message: {e}"),
        )
    })?;
    write!(writer, "Content-Length: {}\r\n\r\n", body.len())?;
    writer.write_all(&body)?;
    writer.flush()
}
