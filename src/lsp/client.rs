use std::{
    collections::{HashMap, HashSet},
    io::BufReader,
    path::{Path, PathBuf},
    process::{Child, ChildStdin, Command, Stdio},
    sync::mpsc::{self, Receiver},
    thread,
};

use anyhow::{Result, anyhow};
use lsp_types::Url;
use serde_json::{Value, json};

use crate::lsp::transport;

#[derive(Debug, Clone)]
pub struct LspDiagnostic {
    pub uri: String,
    pub path: PathBuf,
    pub line: u32,
    pub column: u32,
    pub severity: Option<String>,
    pub message: String,
}

impl LspDiagnostic {
    pub fn summary(&self) -> String {
        let location = format!(
            "{}:{}:{}",
            self.path.display(),
            self.line + 1,
            self.column + 1
        );

        match &self.severity {
            Some(severity) => format!("[{severity}] {location} {}", self.message),
            None => format!("{location} {}", self.message),
        }
    }
}

#[derive(Debug, Clone)]
pub struct LspLocation {
    pub path: PathBuf,
    pub line: u32,
    pub character: u32,
}

#[derive(Debug, Clone)]
pub enum LspEvent {
    Diagnostics {
        uri: String,
        diagnostics: Vec<LspDiagnostic>,
    },
    Hover {
        contents: Option<String>,
    },
    Definition {
        location: Option<LspLocation>,
    },
    Status(String),
}

enum PendingRequest {
    Initialize,
    Hover,
    Definition,
}

pub struct LspClient {
    stdin: ChildStdin,
    incoming: Receiver<Value>,
    _child: Child,
    next_request_id: u64,
    pending_requests: HashMap<u64, PendingRequest>,
    queued_messages: Vec<Value>,
    open_documents: HashSet<String>,
    root_uri: String,
    ready: bool,
}

impl LspClient {
    pub fn start(root: &Path) -> Result<Self> {
        let mut child = Command::new("rust-analyzer")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow!("failed to capture rust-analyzer stdin"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow!("failed to capture rust-analyzer stdout"))?;

        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let mut reader = BufReader::new(stdout);

            loop {
                match transport::read_message(&mut reader) {
                    Ok(Some(message)) => {
                        if tx.send(message).is_err() {
                            break;
                        }
                    }
                    Ok(None) => break,
                    Err(_) => break,
                }
            }
        });

        Ok(Self {
            stdin,
            incoming: rx,
            _child: child,
            next_request_id: 1,
            pending_requests: HashMap::new(),
            queued_messages: Vec::new(),
            open_documents: HashSet::new(),
            root_uri: path_to_uri(root)?,
            ready: false,
        })
    }

    pub fn initialize(&mut self) -> Result<()> {
        let request_id = self.next_id();
        self.pending_requests
            .insert(request_id, PendingRequest::Initialize);

        self.send(&json!({
            "jsonrpc": "2.0",
            "id": request_id,
            "method": "initialize",
            "params": {
                "processId": std::process::id(),
                "rootUri": self.root_uri,
                "workspaceFolders": [{
                    "uri": self.root_uri,
                    "name": "Noir",
                }],
                "clientInfo": {
                    "name": "Noir",
                    "version": env!("CARGO_PKG_VERSION"),
                },
                "capabilities": {
                    "general": {
                        "positionEncodings": ["utf-8"],
                    },
                    "textDocument": {
                        "hover": {
                            "contentFormat": ["markdown", "plaintext"],
                        },
                        "definition": {
                            "linkSupport": true,
                        },
                        "publishDiagnostics": {},
                        "synchronization": {
                            "didSave": false,
                            "dynamicRegistration": false,
                            "willSave": false,
                            "willSaveWaitUntil": false,
                        },
                    },
                    "workspace": {
                        "workspaceFolders": true,
                    },
                },
            }
        }))
    }

    pub fn open_document(&mut self, path: &Path, text: &str, version: i32) -> Result<()> {
        let uri = path_to_uri(path)?;

        if self.open_documents.contains(&uri) {
            return Ok(());
        }

        self.open_documents.insert(uri.clone());

        self.send_or_queue(json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": uri,
                    "languageId": language_id(path),
                    "version": version,
                    "text": text,
                }
            }
        }))
    }

    pub fn change_document(&mut self, path: &Path, text: &str, version: i32) -> Result<()> {
        let uri = path_to_uri(path)?;

        if !self.open_documents.contains(&uri) {
            return self.open_document(path, text, version);
        }

        self.send_or_queue(json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didChange",
            "params": {
                "textDocument": {
                    "uri": uri,
                    "version": version,
                },
                "contentChanges": [{
                    "text": text,
                }]
            }
        }))
    }

    pub fn request_hover(&mut self, path: &Path, line: u32, character: u32) -> Result<()> {
        let uri = path_to_uri(path)?;
        let request_id = self.next_id();
        self.pending_requests
            .insert(request_id, PendingRequest::Hover);

        self.send_or_queue(json!({
            "jsonrpc": "2.0",
            "id": request_id,
            "method": "textDocument/hover",
            "params": {
                "textDocument": {
                    "uri": uri,
                },
                "position": {
                    "line": line,
                    "character": character,
                }
            }
        }))
    }

    pub fn request_definition(&mut self, path: &Path, line: u32, character: u32) -> Result<()> {
        let uri = path_to_uri(path)?;
        let request_id = self.next_id();
        self.pending_requests
            .insert(request_id, PendingRequest::Definition);

        self.send_or_queue(json!({
            "jsonrpc": "2.0",
            "id": request_id,
            "method": "textDocument/definition",
            "params": {
                "textDocument": {
                    "uri": uri,
                },
                "position": {
                    "line": line,
                    "character": character,
                }
            }
        }))
    }

    pub fn drain_events(&mut self) -> Vec<LspEvent> {
        let mut events = Vec::new();

        while let Ok(message) = self.incoming.try_recv() {
            self.handle_message(message, &mut events);
        }

        events
    }

    fn handle_message(&mut self, message: Value, events: &mut Vec<LspEvent>) {
        if let Some(method) = message.get("method").and_then(Value::as_str) {
            if let Some(id) = message.get("id") {
                let _ = self.send(&json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": Value::Null,
                }));
                return;
            }

            match method {
                "textDocument/publishDiagnostics" => {
                    if let Some((uri, diagnostics)) =
                        parse_publish_diagnostics(message.get("params").unwrap_or(&Value::Null))
                    {
                        events.push(LspEvent::Diagnostics { uri, diagnostics });
                    }
                }
                "window/logMessage" | "window/showMessage" => {
                    if let Some(text) = message
                        .get("params")
                        .and_then(|params| params.get("message"))
                        .and_then(Value::as_str)
                    {
                        events.push(LspEvent::Status(text.trim().to_string()));
                    }
                }
                _ => {}
            }

            return;
        }

        let Some(request_id) = parse_request_id(message.get("id")) else {
            return;
        };

        let Some(pending_request) = self.pending_requests.remove(&request_id) else {
            return;
        };

        if let Some(error) = message.get("error") {
            events.push(LspEvent::Status(format!("LSP error: {error}")));
            return;
        }

        match pending_request {
            PendingRequest::Initialize => {
                self.ready = true;
                let _ = self.send(&json!({
                    "jsonrpc": "2.0",
                    "method": "initialized",
                    "params": {},
                }));

                for queued in std::mem::take(&mut self.queued_messages) {
                    let _ = self.send(&queued);
                }

                events.push(LspEvent::Status("rust-analyzer ready".to_string()));
            }
            PendingRequest::Hover => {
                events.push(LspEvent::Hover {
                    contents: parse_hover(message.get("result")),
                });
            }
            PendingRequest::Definition => {
                events.push(LspEvent::Definition {
                    location: parse_definition(message.get("result")),
                });
            }
        }
    }

    fn send_or_queue(&mut self, value: Value) -> Result<()> {
        if self.ready {
            self.send(&value)
        } else {
            self.queued_messages.push(value);
            Ok(())
        }
    }

    fn send(&mut self, value: &Value) -> Result<()> {
        transport::write_message(&mut self.stdin, value)?;
        Ok(())
    }

    fn next_id(&mut self) -> u64 {
        let id = self.next_request_id;
        self.next_request_id += 1;
        id
    }
}

fn parse_request_id(id: Option<&Value>) -> Option<u64> {
    id.and_then(Value::as_u64)
}

fn parse_publish_diagnostics(params: &Value) -> Option<(String, Vec<LspDiagnostic>)> {
    let uri = params.get("uri")?.as_str()?.to_string();
    let path = uri_to_path(&uri)?;
    let diagnostics = params
        .get("diagnostics")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| parse_diagnostic(item, &uri, &path))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    Some((uri, diagnostics))
}

fn parse_diagnostic(item: &Value, uri: &str, path: &Path) -> Option<LspDiagnostic> {
    let range = item.get("range")?;
    let start = range.get("start")?;
    let line = start.get("line")?.as_u64()? as u32;
    let column = start.get("character")?.as_u64()? as u32;
    let message = item.get("message")?.as_str()?.trim().to_string();
    let severity = item
        .get("severity")
        .and_then(Value::as_u64)
        .map(severity_name);

    Some(LspDiagnostic {
        uri: uri.to_string(),
        path: path.to_path_buf(),
        line,
        column,
        severity,
        message,
    })
}

fn severity_name(value: u64) -> String {
    match value {
        1 => "error".to_string(),
        2 => "warning".to_string(),
        3 => "info".to_string(),
        4 => "hint".to_string(),
        _ => format!("severity-{value}"),
    }
}

fn parse_hover(result: Option<&Value>) -> Option<String> {
    let contents = result?.get("contents")?;
    let rendered = render_hover_contents(contents)?;

    let compact = rendered
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ");

    if compact.is_empty() {
        None
    } else {
        Some(compact)
    }
}

fn render_hover_contents(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.to_string()),
        Value::Array(items) => {
            let rendered = items
                .iter()
                .filter_map(render_hover_contents)
                .collect::<Vec<_>>()
                .join("\n");

            if rendered.is_empty() {
                None
            } else {
                Some(rendered)
            }
        }
        Value::Object(map) => {
            if let Some(text) = map.get("value").and_then(Value::as_str) {
                return Some(text.to_string());
            }

            if let (Some(language), Some(value)) = (
                map.get("language").and_then(Value::as_str),
                map.get("value").and_then(Value::as_str),
            ) {
                return Some(format!("{language}: {value}"));
            }

            None
        }
        _ => None,
    }
}

fn parse_definition(result: Option<&Value>) -> Option<LspLocation> {
    let result = result?;

    if let Some(location) = parse_location_value(result) {
        return Some(location);
    }

    result
        .as_array()
        .and_then(|items| items.iter().find_map(parse_location_value))
}

fn parse_location_value(value: &Value) -> Option<LspLocation> {
    let uri = value
        .get("uri")
        .or_else(|| value.get("targetUri"))?
        .as_str()?
        .to_string();
    let path = uri_to_path(&uri)?;
    let range = value
        .get("range")
        .or_else(|| value.get("targetSelectionRange"))
        .or_else(|| value.get("targetRange"))?;
    let start = range.get("start")?;

    Some(LspLocation {
        path,
        line: start.get("line")?.as_u64()? as u32,
        character: start.get("character")?.as_u64()? as u32,
    })
}

fn path_to_uri(path: &Path) -> Result<String> {
    let url = Url::from_file_path(path)
        .map_err(|_| anyhow!("failed to create file URI for {}", path.display()))?;
    Ok(url.to_string())
}

fn uri_to_path(uri: &str) -> Option<PathBuf> {
    Url::parse(uri).ok()?.to_file_path().ok()
}

fn language_id(path: &Path) -> &'static str {
    match path.extension().and_then(|ext| ext.to_str()) {
        Some("rs") => "rust",
        _ => "plaintext",
    }
}
