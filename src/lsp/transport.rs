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

pub struct StdioTransport {
    stdin: ChildStdin,
    events: Receiver<TransportEvent>,
    child: Child,
}

impl StdioTransport {
    pub fn start(server: &str) -> Result<Self> {
        let mut child = Command::new(server)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .with_context(|| format!("failed to spawn {server}"))?;

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
                match read_value(&mut reader) {
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

    pub fn send<T: Serialize>(&mut self, message: &T) -> Result<()> {
        write_message(&mut self.stdin, message).context("failed to write JSON-RPC message")
    }

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

fn read_value(reader: &mut BufReader<ChildStdout>) -> io::Result<Option<Value>> {
    let mut content_length = None;

    loop {
        let mut header = String::new();
        let bytes_read = reader.read_line(&mut header)?;

        if bytes_read == 0 {
            return Ok(None);
        }

        if header == "\r\n" || header == "\n" {
            break;
        }

        if let Some((name, value)) = header.split_once(':') {
            if name.eq_ignore_ascii_case("Content-Length") {
                let parsed_length = value.trim().parse::<usize>().map_err(|err| {
                    io::Error::new(io::ErrorKind::InvalidData, format!("invalid length: {err}"))
                })?;

                content_length = Some(parsed_length);
            }
        }
    }

    let Some(content_length) = content_length else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "missing Content-Length header",
        ));
    };

    let mut body = vec![0; content_length];
    std::io::Read::read_exact(reader, &mut body)?;

    let message = serde_json::from_slice(&body).map_err(|err| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("invalid JSON-RPC body: {err}"),
        )
    })?;

    Ok(Some(message))
}

fn write_message(writer: &mut impl Write, message: &impl Serialize) -> io::Result<()> {
    let body = serde_json::to_vec(message).map_err(|err| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("failed to serialize JSON-RPC message: {err}"),
        )
    })?;

    let header = format!("Content-Length: {}\r\n\r\n", body.len());
    writer.write_all(header.as_bytes())?;
    writer.write_all(&body)?;
    writer.flush()
}
