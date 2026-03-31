use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};

use anyhow::{Result, anyhow};
use lsp_types::{
    DiagnosticSeverity, DidChangeTextDocumentParams, DidOpenTextDocumentParams,
    GotoDefinitionResponse, Hover, HoverContents, InitializedParams, Location, MarkedString,
    Position, PublishDiagnosticsParams, TextDocumentContentChangeEvent, TextDocumentIdentifier,
    TextDocumentItem, TextDocumentPositionParams, Url, VersionedTextDocumentIdentifier,
    WorkspaceFolder,
    notification::{
        DidChangeTextDocument, DidOpenTextDocument, Exit, Initialized, LogMessage, Notification,
        PublishDiagnostics, ShowMessage,
    },
    request::{GotoDefinition, HoverRequest, Initialize, Request, Shutdown},
};
use serde_json::{Value, json, to_value};

use crate::{
    languages::ServerConfig,
    lsp::{
        protocol::{
            IncomingMessage, NotificationMessage, RequestId, RequestMessage, ResponseMessage,
            ServerNotification, ServerResponse,
        },
        transport::{StdioTransport, TransportEvent},
    },
};

// ── Public event type ─────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum LspEvent {
    Initialized,
    Shutdown,
    Diagnostics {
        path: PathBuf,
        diagnostics: Vec<LspDiagnostic>,
    },
    Hover {
        contents: Option<String>,
    },
    Definition {
        /// `None` = server returned no result.
        location: Option<(PathBuf, u32, u32)>,
    },
    LogMessage(String),
    TransportError(String),
    ServerExited,
}

#[derive(Debug, Clone)]
pub struct LspDiagnostic {
    pub line: u32,
    pub character: u32,
    pub severity: Option<DiagnosticSeverity>,
    pub message: String,
}

// ── Internal state machine ────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ClientState {
    Created,
    Initializing,
    Ready,
    ShutdownRequested,
    Exited,
}

#[derive(Debug, Clone, Copy)]
enum PendingRequest {
    Initialize,
    Shutdown,
    Hover,
    GotoDefinition,
}

// ── LspClient ─────────────────────────────────────────────────────────────────

pub struct LspClient {
    transport: StdioTransport,
    next_id: u64,
    pending: HashMap<RequestId, PendingRequest>,
    /// Messages queued while the server is still initializing.
    queued: Vec<Value>,
    open_documents: HashSet<Url>,
    root_uri: Url,
    root_name: String,
    state: ClientState,
}

impl LspClient {
    /// Spawn the server described by `config` and return a connected client.
    ///
    /// Call [`initialize`](Self::initialize) immediately after to start the
    /// LSP handshake.
    pub fn start(root: &Path, config: ServerConfig) -> Result<Self> {
        let root_uri = Url::from_file_path(root)
            .map_err(|_| anyhow!("cannot convert '{}' to a file URI", root.display()))?;
        let root_name = root
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(".")
            .to_string();

        let transport = StdioTransport::start(&config.command, &config.args)?;

        Ok(Self {
            transport,
            next_id: 1,
            pending: HashMap::new(),
            queued: Vec::new(),
            open_documents: HashSet::new(),
            root_uri,
            root_name,
            state: ClientState::Created,
        })
    }

    // ── Lifecycle ─────────────────────────────────────────────────────────────

    /// Send the `initialize` request. Must be called exactly once after [`start`](Self::start).
    pub fn initialize(&mut self) -> Result<()> {
        if self.state != ClientState::Created {
            return Ok(());
        }

        let workspace = WorkspaceFolder {
            uri: self.root_uri.clone(),
            name: self.root_name.clone(),
        };

        let id = self.alloc_id(PendingRequest::Initialize);
        let params = json!({
            "processId": std::process::id(),
            "rootUri": self.root_uri,
            "workspaceFolders": [workspace],
            "clientInfo": {
                "name": "Noir",
                "version": env!("CARGO_PKG_VERSION"),
            },
            "capabilities": {
                "textDocument": {
                    "publishDiagnostics": {},
                    "synchronization": {
                        "dynamicRegistration": false,
                        "didSave": true,
                        "willSave": false,
                        "willSaveWaitUntil": false,
                    },
                    "hover": {
                        "contentFormat": ["plaintext", "markdown"],
                    },
                    "definition": {},
                },
            },
        });

        self.transport
            .send(&RequestMessage::new(id, Initialize::METHOD, params))?;
        self.state = ClientState::Initializing;
        Ok(())
    }

    pub fn shutdown(&mut self) -> Result<()> {
        if self.state != ClientState::Ready {
            return Ok(());
        }
        let id = self.alloc_id(PendingRequest::Shutdown);
        self.transport
            .send(&RequestMessage::new(id, Shutdown::METHOD, ()))?;
        self.state = ClientState::ShutdownRequested;
        Ok(())
    }

    pub fn exit(&mut self) -> Result<()> {
        if self.state == ClientState::Exited {
            return Ok(());
        }
        self.transport
            .send(&NotificationMessage::new(Exit::METHOD, ()))?;
        self.state = ClientState::Exited;
        Ok(())
    }

    // ── Document sync ─────────────────────────────────────────────────────────

    /// Notify the server that `path` has been opened.
    ///
    /// `language_id` should come from [`LanguageRegistry::language_id_for_path`].
    /// No-op if the document is already open.
    pub fn open_document(
        &mut self,
        path: &Path,
        text: String,
        version: i32,
        language_id: &str,
    ) -> Result<()> {
        let uri = path_to_uri(path)?;
        if self.open_documents.contains(&uri) {
            return Ok(());
        }

        let params = DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: language_id.to_string(),
                version,
                text,
            },
        };

        self.open_documents.insert(uri);
        self.send_notification_or_queue(DidOpenTextDocument::METHOD, params)
    }

    /// Notify the server that `path`'s content has changed.
    ///
    /// Opens the document first if it has not been opened yet.
    pub fn change_document(
        &mut self,
        path: &Path,
        text: String,
        version: i32,
        language_id: &str,
    ) -> Result<()> {
        let uri = path_to_uri(path)?;

        if !self.open_documents.contains(&uri) {
            return self.open_document(path, text, version, language_id);
        }

        let params = DidChangeTextDocumentParams {
            text_document: VersionedTextDocumentIdentifier { uri, version },
            content_changes: vec![TextDocumentContentChangeEvent {
                range: None,
                range_length: None,
                text,
            }],
        };

        self.send_notification_or_queue(DidChangeTextDocument::METHOD, params)
    }

    // ── LSP features ──────────────────────────────────────────────────────────

    pub fn hover(&mut self, path: &Path, line: u32, character: u32) -> Result<()> {
        let uri = path_to_uri(path)?;
        let params = TextDocumentPositionParams {
            text_document: TextDocumentIdentifier { uri },
            position: Position { line, character },
        };
        let id = self.alloc_id(PendingRequest::Hover);
        self.send_request_or_queue(HoverRequest::METHOD, id, params)
    }

    pub fn definition(&mut self, path: &Path, line: u32, character: u32) -> Result<()> {
        let uri = path_to_uri(path)?;
        let params = TextDocumentPositionParams {
            text_document: TextDocumentIdentifier { uri },
            position: Position { line, character },
        };
        let id = self.alloc_id(PendingRequest::GotoDefinition);
        self.send_request_or_queue(GotoDefinition::METHOD, id, params)
    }

    // ── Event draining ────────────────────────────────────────────────────────

    /// Drain all pending messages from the server and return them as events.
    /// Call this every frame from `App::tick`.
    pub fn drain_events(&mut self) -> Vec<LspEvent> {
        let mut events = Vec::new();

        while let Some(transport_event) = self.transport.try_recv() {
            match transport_event {
                TransportEvent::Message(msg) => self.handle_message(msg, &mut events),
                TransportEvent::ReadError(e) => events.push(LspEvent::TransportError(e)),
                TransportEvent::Closed => events.push(LspEvent::ServerExited),
            }
        }

        events
    }

    // ── Internal message dispatch ─────────────────────────────────────────────

    fn handle_message(&mut self, message: IncomingMessage, events: &mut Vec<LspEvent>) {
        match message {
            // Server-initiated requests: acknowledge with a null result.
            IncomingMessage::Request(req) => {
                let reply = ResponseMessage::success(req.id, Value::Null);
                if let Err(e) = self.transport.send(&reply) {
                    events.push(LspEvent::TransportError(e.to_string()));
                }
            }
            IncomingMessage::Notification(n) => self.handle_notification(n, events),
            IncomingMessage::Response(r) => self.handle_response(r, events),
        }
    }

    fn handle_notification(&mut self, notif: ServerNotification, events: &mut Vec<LspEvent>) {
        if notif.method == PublishDiagnostics::METHOD {
            let Some(params) = notif.params else { return };
            match serde_json::from_value::<PublishDiagnosticsParams>(params) {
                Ok(p) => {
                    if let Ok(path) = p.uri.to_file_path() {
                        let diagnostics = p
                            .diagnostics
                            .into_iter()
                            .map(|d| LspDiagnostic {
                                line: d.range.start.line,
                                character: d.range.start.character,
                                severity: d.severity,
                                message: d.message,
                            })
                            .collect();
                        events.push(LspEvent::Diagnostics { path, diagnostics });
                    }
                }
                Err(e) => events.push(LspEvent::TransportError(e.to_string())),
            }
        } else if notif.method == LogMessage::METHOD {
            let Some(params) = notif.params else { return };
            match serde_json::from_value::<lsp_types::LogMessageParams>(params) {
                Ok(p) => events.push(LspEvent::LogMessage(p.message)),
                Err(e) => events.push(LspEvent::TransportError(e.to_string())),
            }
        } else if notif.method == ShowMessage::METHOD {
            let Some(params) = notif.params else { return };
            match serde_json::from_value::<lsp_types::ShowMessageParams>(params) {
                Ok(p) => events.push(LspEvent::LogMessage(p.message)),
                Err(e) => events.push(LspEvent::TransportError(e.to_string())),
            }
        }
    }

    fn handle_response(&mut self, response: ServerResponse, events: &mut Vec<LspEvent>) {
        let Some(kind) = self.pending.remove(&response.id) else {
            return;
        };

        if let Some(error) = response.error {
            let detail = match error.data {
                Some(d) => format!("{} (code {}, data: {d})", error.message, error.code),
                None => format!("{} (code {})", error.message, error.code),
            };
            events.push(LspEvent::TransportError(detail));
            return;
        }

        match kind {
            PendingRequest::Initialize => {
                // Send `initialized` notification, then flush queued messages.
                if let Err(e) = self
                    .transport
                    .send(&NotificationMessage::new(Initialized::METHOD, InitializedParams {}))
                {
                    events.push(LspEvent::TransportError(e.to_string()));
                    return;
                }

                for msg in std::mem::take(&mut self.queued) {
                    if let Err(e) = self.transport.send(&msg) {
                        events.push(LspEvent::TransportError(e.to_string()));
                        return;
                    }
                }

                self.state = ClientState::Ready;
                events.push(LspEvent::Initialized);
            }

            PendingRequest::Shutdown => {
                self.state = ClientState::Created;
                events.push(LspEvent::Shutdown);
            }

            PendingRequest::Hover => {
                let text = response
                    .result
                    .and_then(|v| serde_json::from_value::<Option<Hover>>(v).ok())
                    .flatten()
                    .map(|h| render_hover(h.contents));
                events.push(LspEvent::Hover { contents: text });
            }

            PendingRequest::GotoDefinition => {
                let location = response
                    .result
                    .and_then(|v| {
                        serde_json::from_value::<Option<GotoDefinitionResponse>>(v)
                            .ok()
                            .flatten()
                    })
                    .and_then(first_definition_location);
                events.push(LspEvent::Definition { location });
            }
        }
    }

    // ── Helpers ───────────────────────────────────────────────────────────────

    fn alloc_id(&mut self, kind: PendingRequest) -> RequestId {
        let id = RequestId::new(self.next_id);
        self.next_id += 1;
        self.pending.insert(id, kind);
        id
    }

    /// Send a notification now if the server is ready, otherwise queue it.
    fn send_notification_or_queue<P: serde::Serialize>(
        &mut self,
        method: &'static str,
        params: P,
    ) -> Result<()> {
        let msg = to_value(NotificationMessage::new(method, params))?;
        if self.state == ClientState::Ready {
            self.transport.send(&msg)?;
        } else {
            self.queued.push(msg);
        }
        Ok(())
    }

    /// Send a request now if the server is ready, otherwise queue it.
    fn send_request_or_queue<P: serde::Serialize>(
        &mut self,
        method: &'static str,
        id: RequestId,
        params: P,
    ) -> Result<()> {
        let msg = to_value(RequestMessage::new(id, method, params))?;
        if self.state == ClientState::Ready {
            self.transport.send(&msg)?;
        } else {
            self.queued.push(msg);
        }
        Ok(())
    }
}

// ── Pure helper functions ─────────────────────────────────────────────────────

fn path_to_uri(path: &Path) -> Result<Url> {
    Url::from_file_path(path)
        .map_err(|_| anyhow!("cannot convert '{}' to a file URI", path.display()))
}

fn render_hover(contents: HoverContents) -> String {
    match contents {
        HoverContents::Scalar(s) => render_marked_string(s),
        HoverContents::Array(items) => items
            .into_iter()
            .map(render_marked_string)
            .collect::<Vec<_>>()
            .join("\n\n"),
        HoverContents::Markup(m) => m.value,
    }
}

fn render_marked_string(s: MarkedString) -> String {
    match s {
        MarkedString::String(text) => text,
        MarkedString::LanguageString(block) => format!("{}\n{}", block.language, block.value),
    }
}

fn first_definition_location(resp: GotoDefinitionResponse) -> Option<(PathBuf, u32, u32)> {
    match resp {
        GotoDefinitionResponse::Scalar(loc) => location_to_pos(loc),
        GotoDefinitionResponse::Array(locs) => locs.into_iter().next().and_then(location_to_pos),
        GotoDefinitionResponse::Link(links) => links.into_iter().next().and_then(|link| {
            link.target_uri.to_file_path().ok().map(|path| {
                let s = link.target_selection_range.start;
                (path, s.line, s.character)
            })
        }),
    }
}

fn location_to_pos(loc: Location) -> Option<(PathBuf, u32, u32)> {
    loc.uri
        .to_file_path()
        .ok()
        .map(|p| (p, loc.range.start.line, loc.range.start.character))
}
