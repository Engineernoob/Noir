mod client;
mod protocol;
mod transport;

pub use client::{LspClient, LspDiagnostic, LspEvent};
pub use lsp_types::DiagnosticSeverity;
// ServerConfig lives in crate::languages; re-export here for convenience.
#[allow(unused_imports)]
pub use crate::languages::ServerConfig;
