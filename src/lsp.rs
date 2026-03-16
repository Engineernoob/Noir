use std::{
    io::{BufRead, BufReader, Write},
    process::{Child, ChildStdin, Command, Stdio},
    sync::mpsc::{self, Receiver},
    thread,
};

use anyhow::Result;
use serde_json::{Value, json};

pub struct LspClient {
    stdin: ChildStdin,
    pub rx: Receiver<Value>,
    _child: Child,
}

impl LspClient {
    pub fn start() -> Result<Self> {
        let mut child = Command::new("rust-analyzer")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;

        let stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();

        let reader = BufReader::new(stdout);
        let (tx, rx) = mpsc::channel();

        thread::spawn(move || {
            for line in reader.lines().flatten() {
                if line.starts_with("Content-Length") {
                    continue;
                }

                if let Ok(json) = serde_json::from_str::<Value>(&line) {
                    let _ = tx.send(json);
                }
            }
        });

        Ok(Self {
            stdin,
            rx,
            _child: child,
        })
    }

    fn send(&mut self, value: Value) -> Result<()> {
        let body = value.to_string();
        let header = format!("Content-Length: {}\r\n\r\n", body.len());

        self.stdin.write_all(header.as_bytes())?;
        self.stdin.write_all(body.as_bytes())?;
        self.stdin.flush()?;

        Ok(())
    }

    pub fn initialize(&mut self) -> Result<()> {
        self.send(json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "capabilities": {}
            }
        }))
    }

    pub fn open_file(&mut self, uri: String, text: String) -> Result<()> {
        self.send(json!({
            "jsonrpc":"2.0",
            "method":"textDocument/didOpen",
            "params":{
                "textDocument":{
                    "uri": uri,
                    "languageId":"rust",
                    "version":1,
                    "text":text
                }
            }
        }))
    }
}
