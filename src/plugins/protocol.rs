use serde::{Deserialize, Serialize};

/// One JSON message per line on the plugin stdio channel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PluginMessage {
    Register(RegisterMessage),
    CommandResult(CommandResultMessage),
}

/// One JSON message per line sent from Noir to a plugin.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum HostMessage {
    ExecuteCommand(ExecuteCommandMessage),
}

/// Initial plugin handshake sent from the plugin process to Noir.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegisterMessage {
    pub plugin_name: String,
    #[serde(default)]
    pub commands: Vec<String>,
}

/// Minimal context passed to plugin command execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommandExecutionContext {
    pub workspace_root: String,
    pub active_file_path: Option<String>,
    pub cursor: Option<CursorPosition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CursorPosition {
    pub line: u32,
    pub column: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecuteCommandMessage {
    pub plugin_name: String,
    pub command_name: String,
    pub request_id: u64,
    pub context: CommandExecutionContext,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommandResultMessage {
    pub plugin_name: String,
    pub command_name: String,
    pub request_id: u64,
    pub success: bool,
    pub output: String,
}

pub fn parse_plugin_message(line: &str) -> Result<PluginMessage, serde_json::Error> {
    serde_json::from_str(line)
}

pub fn serialize_host_message(message: &HostMessage) -> Result<String, serde_json::Error> {
    serde_json::to_string(message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_register_message() {
        let src = r#"{"type":"register","plugin_name":"hello","commands":["hello.run"]}"#;
        let message = parse_plugin_message(src).unwrap();

        assert_eq!(
            message,
            PluginMessage::Register(RegisterMessage {
                plugin_name: "hello".to_string(),
                commands: vec!["hello.run".to_string()],
            })
        );
    }

    #[test]
    fn serializes_execute_command_message() {
        let message = HostMessage::ExecuteCommand(ExecuteCommandMessage {
            plugin_name: "hello".to_string(),
            command_name: "hello.run".to_string(),
            request_id: 7,
            context: CommandExecutionContext {
                workspace_root: "/tmp/demo".to_string(),
                active_file_path: Some("/tmp/demo/src/main.rs".to_string()),
                cursor: Some(CursorPosition { line: 4, column: 2 }),
            },
        });

        let json = serialize_host_message(&message).unwrap();
        assert!(json.contains("\"type\":\"execute_command\""));
        assert!(json.contains("\"request_id\":7"));
    }

    #[test]
    fn parses_command_result_message() {
        let src = r#"{"type":"command_result","plugin_name":"hello","command_name":"hello.run","request_id":7,"success":true,"output":"done"}"#;
        let message = parse_plugin_message(src).unwrap();

        assert_eq!(
            message,
            PluginMessage::CommandResult(CommandResultMessage {
                plugin_name: "hello".to_string(),
                command_name: "hello.run".to_string(),
                request_id: 7,
                success: true,
                output: "done".to_string(),
            })
        );
    }
}
