mod client;
mod protocol;
mod transport;

pub use client::{LspClient, LspDiagnostic, LspEvent};
pub use lsp_types::DiagnosticSeverity;
