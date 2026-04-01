#![allow(dead_code)]

use std::{
    collections::HashMap,
    fs,
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
    sync::{
        Arc, Mutex,
        mpsc::{self, Receiver, Sender},
    },
    thread,
};

use anyhow::{Context, Result, anyhow};

use super::{
    manifest::Manifest,
    protocol::{
        CommandExecutionContext, CommandResultMessage, ExecuteCommandMessage, HostMessage,
        PluginMessage, RegisterMessage, parse_plugin_message, serialize_host_message,
    },
};

// ── Plugin ────────────────────────────────────────────────────────────────────

/// A discovered plugin: its on-disk location plus its parsed manifest.
#[derive(Debug, Clone)]
pub struct Plugin {
    /// Absolute path to the plugin's own directory.
    pub dir: PathBuf,
    pub manifest: Manifest,
}

impl Plugin {
    /// Absolute path to the plugin entry point.
    pub fn entry_path(&self) -> PathBuf {
        self.dir.join(&self.manifest.entry)
    }

    pub fn is_enabled(&self) -> bool {
        self.manifest.is_enabled()
    }
}

// ── Runtime State ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginRegistration {
    pub plugin_name: String,
    pub commands: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginCommand {
    pub plugin_name: String,
    pub command_name: String,
    pub title: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginCommandResult {
    pub plugin_name: String,
    pub command_name: String,
    pub request_id: u64,
    pub success: bool,
    pub output: String,
}

/// Live process handle for a running plugin.
pub struct PluginProcess {
    child: Child,
    stdin: ChildStdin,
}

impl PluginProcess {
    pub fn new(child: Child, stdin: ChildStdin) -> Self {
        Self { child, stdin }
    }

    pub fn id(&self) -> u32 {
        self.child.id()
    }

    pub fn child(&self) -> &Child {
        &self.child
    }

    pub fn child_mut(&mut self) -> &mut Child {
        &mut self.child
    }

    pub fn stdin_mut(&mut self) -> &mut ChildStdin {
        &mut self.stdin
    }
}

#[derive(Debug, Clone)]
pub struct PluginStartupError {
    pub plugin_name: String,
    pub entry_path: PathBuf,
    pub message: String,
}

#[derive(Debug, Clone, Default)]
pub struct PluginStartupSummary {
    pub started: Vec<String>,
    pub failed: Vec<PluginStartupError>,
}

impl PluginStartupSummary {
    pub fn started_count(&self) -> usize {
        self.started.len()
    }

    pub fn failed_count(&self) -> usize {
        self.failed.len()
    }

    pub fn is_empty(&self) -> bool {
        self.started.is_empty() && self.failed.is_empty()
    }
}

// ── PluginManager ─────────────────────────────────────────────────────────────

/// Holds all discovered plugins.
///
/// Discovery is a separate step (`discover`) so the manager can be constructed
/// cheaply and populated later (or not at all when the directory is absent).
pub struct PluginManager {
    plugins: Vec<Plugin>,
    processes: HashMap<String, PluginProcess>,
    registrations: Arc<Mutex<HashMap<String, PluginRegistration>>>,
    events_rx: Receiver<PluginCommandResult>,
    events_tx: Sender<PluginCommandResult>,
    next_request_id: u64,
}

impl Default for PluginManager {
    fn default() -> Self {
        let (events_tx, events_rx) = mpsc::channel();
        Self {
            plugins: Vec::new(),
            processes: HashMap::new(),
            registrations: Arc::new(Mutex::new(HashMap::new())),
            events_rx,
            events_tx,
            next_request_id: 1,
        }
    }
}

impl PluginManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Scan `plugins_dir` for subdirectories that contain a `plugin.toml`.
    ///
    /// Each subdirectory is treated as one plugin.  Directories whose manifest
    /// cannot be parsed emit a warning to `stderr` and are skipped rather than
    /// failing the whole load.  Returns `Ok` even when `plugins_dir` does not
    /// exist (Noir simply has no plugins).
    pub fn discover(&mut self, plugins_dir: &Path) -> Result<()> {
        if !plugins_dir.exists() {
            return Ok(());
        }

        let entries = fs::read_dir(plugins_dir)
            .with_context(|| format!("cannot read plugins dir: {}", plugins_dir.display()))?;

        for entry in entries.flatten() {
            let dir = entry.path();
            if !dir.is_dir() {
                continue;
            }

            let manifest_path = dir.join("plugin.toml");
            if !manifest_path.exists() {
                continue;
            }

            match load_manifest(&manifest_path) {
                Ok(manifest) => self.plugins.push(Plugin { dir, manifest }),
                Err(e) => {
                    eprintln!(
                        "[noir] skipping plugin at {}: {e}",
                        dir.display()
                    );
                }
            }
        }

        // Stable order: sort by plugin name so the list is deterministic.
        self.plugins.sort_by(|a, b| a.manifest.name.cmp(&b.manifest.name));

        Ok(())
    }

    /// All discovered plugins, enabled or not.
    pub fn all(&self) -> &[Plugin] {
        &self.plugins
    }

    /// Only the enabled plugins.
    pub fn enabled(&self) -> impl Iterator<Item = &Plugin> {
        self.plugins.iter().filter(|p| p.is_enabled())
    }

    /// Look up a plugin by name (case-sensitive).
    pub fn find_by_name(&self, name: &str) -> Option<&Plugin> {
        self.plugins.iter().find(|p| p.manifest.name == name)
    }

    /// Launch every enabled plugin that is not already running.
    ///
    /// Startup failures are collected and returned to the caller so Noir can
    /// surface them without aborting editor startup.
    pub fn start_enabled(&mut self) -> PluginStartupSummary {
        let mut summary = PluginStartupSummary::default();

        for plugin in self.enabled_plugins_to_start() {
            match start_plugin_process(
                &plugin,
                Arc::clone(&self.registrations),
                self.events_tx.clone(),
            ) {
                Ok(process) => {
                    summary.started.push(plugin.manifest.name.clone());
                    self.processes
                        .insert(plugin.manifest.name.clone(), process);
                }
                Err(err) => {
                    eprintln!(
                        "[noir] failed to start plugin '{}' ({}): {}",
                        err.plugin_name,
                        err.entry_path.display(),
                        err.message
                    );
                    summary.failed.push(err);
                }
            }
        }

        summary
    }

    /// Running process state keyed by plugin name.
    pub fn running(&self) -> &HashMap<String, PluginProcess> {
        &self.processes
    }

    pub fn running_mut(&mut self) -> &mut HashMap<String, PluginProcess> {
        &mut self.processes
    }

    pub fn is_running(&self, name: &str) -> bool {
        self.processes.contains_key(name)
    }

    /// Snapshot of plugin registrations keyed by plugin name.
    pub fn registrations(&self) -> HashMap<String, PluginRegistration> {
        self.registrations
            .lock()
            .map(|registrations| registrations.clone())
            .unwrap_or_default()
    }

    pub fn registration_for(&self, name: &str) -> Option<PluginRegistration> {
        self.registrations
            .lock()
            .ok()
            .and_then(|registrations| registrations.get(name).cloned())
    }

    pub fn registered_commands(&self) -> Vec<PluginCommand> {
        let mut commands = Vec::new();

        if let Ok(registrations) = self.registrations.lock() {
            for registration in registrations.values() {
                for command_name in &registration.commands {
                    commands.push(PluginCommand {
                        plugin_name: registration.plugin_name.clone(),
                        command_name: command_name.clone(),
                        title: format!(
                            "{}: {}",
                            registration.plugin_name,
                            humanize_plugin_command(command_name)
                        ),
                        description: format!(
                            "Plugin command from {} ({})",
                            registration.plugin_name, command_name
                        ),
                    });
                }
            }
        }

        commands.sort_by(|a, b| a.title.cmp(&b.title));
        commands
    }

    pub fn execute_command(
        &mut self,
        plugin_name: &str,
        command_name: &str,
        context: CommandExecutionContext,
    ) -> Result<u64> {
        let request_id = self.next_request_id;
        self.next_request_id += 1;

        let process = self
            .processes
            .get_mut(plugin_name)
            .ok_or_else(|| anyhow!("plugin '{plugin_name}' is not running"))?;

        let message = HostMessage::ExecuteCommand(ExecuteCommandMessage {
            plugin_name: plugin_name.to_string(),
            command_name: command_name.to_string(),
            request_id,
            context,
        });

        let payload = serialize_host_message(&message)
            .with_context(|| format!("failed to serialize command for plugin '{plugin_name}'"))?;

        process
            .stdin_mut()
            .write_all(payload.as_bytes())
            .with_context(|| format!("failed to write to plugin '{plugin_name}' stdin"))?;
        process
            .stdin_mut()
            .write_all(b"\n")
            .with_context(|| format!("failed to terminate plugin '{plugin_name}' command"))?;
        process
            .stdin_mut()
            .flush()
            .with_context(|| format!("failed to flush plugin '{plugin_name}' stdin"))?;

        Ok(request_id)
    }

    pub fn drain_command_results(&self) -> Vec<PluginCommandResult> {
        let mut results = Vec::new();

        while let Ok(result) = self.events_rx.try_recv() {
            results.push(result);
        }

        results
    }

    fn enabled_plugins_to_start(&self) -> Vec<Plugin> {
        self.plugins
            .iter()
            .filter(|plugin| plugin.is_enabled())
            .filter(|plugin| !self.processes.contains_key(&plugin.manifest.name))
            .cloned()
            .collect()
    }
}

impl Drop for PluginManager {
    fn drop(&mut self) {
        for process in self.processes.values_mut() {
            let _ = process.child_mut().kill();
            let _ = process.child_mut().wait();
        }
    }
}

// ── helpers ───────────────────────────────────────────────────────────────────

fn load_manifest(path: &Path) -> Result<Manifest> {
    let source = fs::read_to_string(path)
        .with_context(|| format!("cannot read {}", path.display()))?;
    Manifest::from_str(&source)
        .with_context(|| format!("invalid manifest at {}", path.display()))
}

fn start_plugin_process(
    plugin: &Plugin,
    registrations: Arc<Mutex<HashMap<String, PluginRegistration>>>,
    events_tx: Sender<PluginCommandResult>,
) -> std::result::Result<PluginProcess, PluginStartupError> {
    let entry_path = plugin.entry_path();
    if !entry_path.exists() {
        return Err(PluginStartupError {
            plugin_name: plugin.manifest.name.clone(),
            entry_path,
            message: "entry path does not exist".to_string(),
        });
    }

    let mut child = Command::new(&entry_path)
        .current_dir(&plugin.dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|err| PluginStartupError {
            plugin_name: plugin.manifest.name.clone(),
            entry_path: entry_path.clone(),
            message: err.to_string(),
        })?;

    let stdin = child.stdin.take().ok_or_else(|| PluginStartupError {
        plugin_name: plugin.manifest.name.clone(),
        entry_path: entry_path.clone(),
        message: "failed to capture plugin stdin".to_string(),
    })?;
    let stdout = child.stdout.take().ok_or_else(|| PluginStartupError {
        plugin_name: plugin.manifest.name.clone(),
        entry_path: entry_path.clone(),
        message: "failed to capture plugin stdout".to_string(),
    })?;

    spawn_stdout_reader(
        plugin.manifest.name.clone(),
        stdout,
        registrations,
        events_tx,
    );

    Ok(PluginProcess::new(child, stdin))
}

fn spawn_stdout_reader(
    source_plugin: String,
    stdout: ChildStdout,
    registrations: Arc<Mutex<HashMap<String, PluginRegistration>>>,
    events_tx: Sender<PluginCommandResult>,
) {
    thread::spawn(move || {
        let reader = BufReader::new(stdout);
        read_plugin_stdout(&source_plugin, reader, &registrations, &events_tx);
    });
}

fn read_plugin_stdout(
    source_plugin: &str,
    mut reader: BufReader<ChildStdout>,
    registrations: &Arc<Mutex<HashMap<String, PluginRegistration>>>,
    events_tx: &Sender<PluginCommandResult>,
) {
    loop {
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }

                match parse_plugin_message(line) {
                    Ok(PluginMessage::Register(message)) => {
                        apply_registration(source_plugin, message, registrations);
                    }
                    Ok(PluginMessage::CommandResult(message)) => {
                        apply_command_result(source_plugin, message, events_tx);
                    }
                    Err(err) => {
                        eprintln!(
                            "[noir] invalid plugin message from '{}': {} | {}",
                            source_plugin, err, line
                        );
                    }
                }
            }
            Err(err) => {
                eprintln!(
                    "[noir] failed reading plugin stdout from '{}': {}",
                    source_plugin, err
                );
                break;
            }
        }
    }
}

fn apply_registration(
    source_plugin: &str,
    message: RegisterMessage,
    registrations: &Arc<Mutex<HashMap<String, PluginRegistration>>>,
) {
    if message.plugin_name != source_plugin {
        eprintln!(
            "[noir] plugin registration name mismatch: manifest='{}' registration='{}'",
            source_plugin, message.plugin_name
        );
        return;
    }

    let registration = PluginRegistration {
        plugin_name: message.plugin_name.clone(),
        commands: message.commands,
    };

    match registrations.lock() {
        Ok(mut registrations) => {
            registrations.insert(source_plugin.to_string(), registration);
        }
        Err(err) => {
            eprintln!(
                "[noir] failed to store registration for '{}': {}",
                source_plugin, err
            );
        }
    }
}

fn apply_command_result(
    source_plugin: &str,
    message: CommandResultMessage,
    events_tx: &Sender<PluginCommandResult>,
) {
    if message.plugin_name != source_plugin {
        eprintln!(
            "[noir] plugin result name mismatch: manifest='{}' result='{}'",
            source_plugin, message.plugin_name
        );
        return;
    }

    let result = PluginCommandResult {
        plugin_name: message.plugin_name,
        command_name: message.command_name,
        request_id: message.request_id,
        success: message.success,
        output: message.output,
    };

    let _ = events_tx.send(result);
}

fn humanize_plugin_command(command_name: &str) -> String {
    let tail = command_name
        .rsplit(['.', ':', '/'])
        .next()
        .unwrap_or(command_name);

    tail.split(['_', '-'])
        .filter(|part| !part.is_empty())
        .map(title_case_word)
        .collect::<Vec<_>>()
        .join(" ")
}

fn title_case_word(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        Some(first) => {
            let mut out = String::new();
            out.extend(first.to_uppercase());
            out.push_str(chars.as_str());
            out
        }
        None => String::new(),
    }
}
