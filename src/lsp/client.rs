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

use crate::lsp::{
    protocol::{
        IncomingMessage, NotificationMessage, RequestId, RequestMessage, ResponseMessage,
        ServerNotification, ServerResponse,
    },
    transport::{StdioTransport, TransportEvent},
};

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
        /// `None` means the server returned no result.
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

pub struct LspClient {
    transport: StdioTransport,
    next_request_id: u64,
    pending_requests: HashMap<RequestId, PendingRequest>,
    queued_messages: Vec<Value>,
    open_documents: HashSet<Url>,
    root_uri: Url,
    root_name: String,
    state: ClientState,
}

impl LspClient {
    pub fn start(root: &Path) -> Result<Self> {
        let root_uri = Url::from_file_path(root)
            .map_err(|_| anyhow!("failed to convert {} to file URI", root.display()))?;
        let root_name = root
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(".")
            .to_string();

        Ok(Self {
            transport: StdioTransport::start("rust-analyzer")?,
            next_request_id: 1,
            pending_requests: HashMap::new(),
            queued_messages: Vec::new(),
            open_documents: HashSet::new(),
            root_uri,
            root_name,
            state: ClientState::Created,
        })
    }

    pub fn initialize(&mut self) -> Result<()> {
        if self.state != ClientState::Created {
            return Ok(());
        }

        let workspace_folder = WorkspaceFolder {
            uri: self.root_uri.clone(),
            name: self.root_name.clone(),
        };

        let request_id = self.allocate_request(PendingRequest::Initialize);
        let params = json!({
            "processId": std::process::id(),
            "rootUri": self.root_uri,
            "workspaceFolders": [workspace_folder],
            "clientInfo": {
                "name": "Noir",
                "version": env!("CARGO_PKG_VERSION"),
            },
            "capabilities": {
                "textDocument": {
                    "publishDiagnostics": {},
                    "synchronization": {
                        "didSave": true,
                        "dynamicRegistration": false,
                        "willSave": false,
                        "willSaveWaitUntil": false
                    }
                }
            },
        });

        self.transport
            .send(&RequestMessage::new(request_id, Initialize::METHOD, params))?;
        self.state = ClientState::Initializing;
        Ok(())
    }

    pub fn open_document(&mut self, path: &Path, text: String, version: i32) -> Result<()> {
        let uri = path_to_uri(path)?;
        if self.open_documents.contains(&uri) {
            return Ok(());
        }

        let params = DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: language_id(path).to_string(),
                version,
                text,
            },
        };

        self.open_documents.insert(uri);
        self.send_notification_or_queue(DidOpenTextDocument::METHOD, params)
    }

    pub fn change_document(&mut self, path: &Path, text: String, version: i32) -> Result<()> {
        let uri = path_to_uri(path)?;

        if !self.open_documents.contains(&uri) {
            return self.open_document(path, text, version);
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

    pub fn shutdown(&mut self) -> Result<()> {
        if !matches!(self.state, ClientState::Ready)
            || matches!(
                self.state,
                ClientState::ShutdownRequested | ClientState::Exited
            )
        {
            return Ok(());
        }

        let request_id = self.allocate_request(PendingRequest::Shutdown);
        self.transport
            .send(&RequestMessage::new(request_id, Shutdown::METHOD, ()))?;
        self.state = ClientState::ShutdownRequested;
        Ok(())
    }

    pub fn hover(&mut self, path: &Path, line: u32, character: u32) -> Result<()> {
        let uri = path_to_uri(path)?;
        let params = TextDocumentPositionParams {
            text_document: TextDocumentIdentifier { uri },
            position: Position { line, character },
        };

        let request_id = self.allocate_request(PendingRequest::Hover);
        self.send_request_or_queue(HoverRequest::METHOD, request_id, params)
    }

    pub fn definition(&mut self, path: &Path, line: u32, character: u32) -> Result<()> {
        let uri = path_to_uri(path)?;
        let params = TextDocumentPositionParams {
            text_document: TextDocumentIdentifier { uri },
            position: Position { line, character },
        };

        let request_id = self.allocate_request(PendingRequest::GotoDefinition);
        self.send_request_or_queue(GotoDefinition::METHOD, request_id, params)
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

    pub fn drain_events(&mut self) -> Vec<LspEvent> {
        let mut events = Vec::new();

        while let Some(event) = self.transport.try_recv() {
            match event {
                TransportEvent::Message(message) => self.handle_message(message, &mut events),
                TransportEvent::ReadError(error) => events.push(LspEvent::TransportError(error)),
                TransportEvent::Closed => events.push(LspEvent::ServerExited),
            }
        }

        events
    }

    fn handle_message(&mut self, message: IncomingMessage, events: &mut Vec<LspEvent>) {
        match message {
            IncomingMessage::Request(request) => {
                if let Err(err) = self
                    .transport
                    .send(&ResponseMessage::success(request.id, Value::Null))
                {
                    events.push(LspEvent::TransportError(err.to_string()));
                }
            }
            IncomingMessage::Notification(notification) => {
                self.handle_notification(notification, events);
            }
            IncomingMessage::Response(response) => {
                self.handle_response(response, events);
            }
        }
    }

    fn handle_notification(
        &mut self,
        notification: ServerNotification,
        events: &mut Vec<LspEvent>,
    ) {
        if notification.method == PublishDiagnostics::METHOD {
            if let Some(params) = notification.params {
                match serde_json::from_value::<PublishDiagnosticsParams>(params) {
                    Ok(params) => {
                        if let Ok(path) = params.uri.to_file_path() {
                            let diagnostics = params
                                .diagnostics
                                .into_iter()
                                .map(|diagnostic| LspDiagnostic {
                                    line: diagnostic.range.start.line,
                                    character: diagnostic.range.start.character,
                                    severity: diagnostic.severity,
                                    message: diagnostic.message,
                                })
                                .collect();

                            events.push(LspEvent::Diagnostics { path, diagnostics });
                        }
                    }
                    Err(err) => events.push(LspEvent::TransportError(err.to_string())),
                }
            }
        } else if notification.method == LogMessage::METHOD {
            if let Some(params) = notification.params {
                match serde_json::from_value::<lsp_types::LogMessageParams>(params) {
                    Ok(params) => events.push(LspEvent::LogMessage(params.message)),
                    Err(err) => events.push(LspEvent::TransportError(err.to_string())),
                }
            }
        } else if notification.method == ShowMessage::METHOD {
            if let Some(params) = notification.params {
                match serde_json::from_value::<lsp_types::ShowMessageParams>(params) {
                    Ok(params) => events.push(LspEvent::LogMessage(params.message)),
                    Err(err) => events.push(LspEvent::TransportError(err.to_string())),
                }
            }
        }
    }

    fn handle_response(&mut self, response: ServerResponse, events: &mut Vec<LspEvent>) {
        let Some(pending_request) = self.pending_requests.remove(&response.id) else {
            return;
        };

        if let Some(error) = response.error {
            let details = match error.data {
                Some(data) => format!("{} (code {}, data: {data})", error.message, error.code),
                None => format!("{} (code {})", error.message, error.code),
            };
            events.push(LspEvent::TransportError(details));
            return;
        }

        match pending_request {
            PendingRequest::Initialize => {
                if let Err(err) = self.transport.send(&NotificationMessage::new(
                    Initialized::METHOD,
                    InitializedParams {},
                )) {
                    events.push(LspEvent::TransportError(err.to_string()));
                    return;
                }

                for message in std::mem::take(&mut self.queued_messages) {
                    if let Err(err) = self.transport.send(&message) {
                        events.push(LspEvent::TransportError(err.to_string()));
                        return;
                    }
                }

                self.state = ClientState::Ready;
                events.push(LspEvent::Initialized);
            }
            PendingRequest::Shutdown => {
                let _ = response.result;
                self.state = ClientState::Created;
                events.push(LspEvent::Shutdown);
            }
            PendingRequest::Hover => {
                let contents = response
                    .result
                    .and_then(|value| serde_json::from_value::<Option<Hover>>(value).ok())
                    .flatten()
                    .map(|hover| render_hover_contents(hover.contents));

                events.push(LspEvent::Hover { contents });
            }
            PendingRequest::GotoDefinition => {
                let location = response.result.and_then(|value| {
                    serde_json::from_value::<Option<GotoDefinitionResponse>>(value)
                        .ok()
                        .flatten()
                        .and_then(first_definition_location)
                });

                events.push(LspEvent::Definition { location });
            }
        }
    }

    fn allocate_request(&mut self, pending: PendingRequest) -> RequestId {
        let id = RequestId::new(self.next_request_id);
        self.next_request_id += 1;
        self.pending_requests.insert(id, pending);
        id
    }

    fn send_notification_or_queue<P>(&mut self, method: &'static str, params: P) -> Result<()>
    where
        P: serde::Serialize,
    {
        let message = to_value(NotificationMessage::new(method, params))?;

        if self.state == ClientState::Ready {
            self.transport.send(&message)?;
        } else {
            self.queued_messages.push(message);
        }

        Ok(())
    }

    fn send_request_or_queue<P>(
        &mut self,
        method: &'static str,
        id: RequestId,
        params: P,
    ) -> Result<()>
    where
        P: serde::Serialize,
    {
        let message = to_value(RequestMessage::new(id, method, params))?;

        if self.state == ClientState::Ready {
            self.transport.send(&message)?;
        } else {
            self.queued_messages.push(message);
        }

        Ok(())
    }
}

fn path_to_uri(path: &Path) -> Result<Url> {
    Url::from_file_path(path)
        .map_err(|_| anyhow!("failed to convert {} to file URI", path.display()))
}

fn language_id(path: &Path) -> &'static str {
    match path.extension().and_then(|ext| ext.to_str()) {
        Some("rs") => "rust",
        _ => "plaintext",
    }
}


fn render_hover_contents(contents: HoverContents) -> String {
    match contents {
        HoverContents::Scalar(marked) => render_marked_string(marked),
        HoverContents::Array(items) => items
            .into_iter()
            .map(render_marked_string)
            .collect::<Vec<_>>()
            .join("\n\n"),
        HoverContents::Markup(markup) => markup.value,
    }
}

fn render_marked_string(marked: MarkedString) -> String {
    match marked {
        MarkedString::String(text) => text,
        MarkedString::LanguageString(block) => format!("{}\n{}", block.language, block.value),
    }
}

fn first_definition_location(response: GotoDefinitionResponse) -> Option<(PathBuf, u32, u32)> {
    match response {
        GotoDefinitionResponse::Scalar(loc) => location_to_file_position(loc),
        GotoDefinitionResponse::Array(locs) => {
            locs.into_iter().next().and_then(location_to_file_position)
        }
        GotoDefinitionResponse::Link(links) => links.into_iter().next().and_then(|link| {
            link.target_uri.to_file_path().ok().map(|path| {
                let pos = link.target_selection_range.start;
                (path, pos.line, pos.character)
            })
        }),
    }
}

fn location_to_file_position(loc: Location) -> Option<(PathBuf, u32, u32)> {
    loc.uri
        .to_file_path()
        .ok()
        .map(|path| (path, loc.range.start.line, loc.range.start.character))
}
