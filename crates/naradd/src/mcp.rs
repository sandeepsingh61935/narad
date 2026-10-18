//! Local Model Context Protocol (MCP) server for coding agents (ADR-005, FR-MCP-1).
//!
//! Exposes Narad dictation controls, transcription history, vocabulary injection,
//! model hot-swapping, and system/hardware capabilities over JSON-RPC 2.0 stdio,
//! Unix domain sockets, and async streams.

use std::path::Path;
use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader};
use tracing::{debug, info, warn};

use narad_core::store::HistoryStore;

pub const PROTOCOL_VERSION: &str = "2024-11-05";
pub const SERVER_NAME: &str = "narad";
pub const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

/// JSON-RPC 2.0 Request or Notification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    #[serde(default)]
    pub id: Option<Value>,
    pub method: String,
    #[serde(default)]
    pub params: Option<Value>,
}

/// JSON-RPC 2.0 Response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

/// JSON-RPC 2.0 Error.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl JsonRpcError {
    pub fn parse_error(data: Option<Value>) -> Self {
        Self {
            code: -32700,
            message: "Parse error".to_string(),
            data,
        }
    }

    pub fn invalid_request(msg: &str) -> Self {
        Self {
            code: -32600,
            message: format!("Invalid Request: {msg}"),
            data: None,
        }
    }

    pub fn method_not_found(method: &str) -> Self {
        Self {
            code: -32601,
            message: format!("Method not found: {method}"),
            data: None,
        }
    }

    pub fn invalid_params(msg: &str) -> Self {
        Self {
            code: -32602,
            message: format!("Invalid params: {msg}"),
            data: None,
        }
    }

    pub fn internal_error(msg: &str) -> Self {
        Self {
            code: -32603,
            message: format!("Internal error: {msg}"),
            data: None,
        }
    }
}

/// MCP tool description structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
}

/// MCP tool call content element.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolContent {
    #[serde(rename = "type")]
    pub content_type: String,
    pub text: String,
}

/// MCP tool call result structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallResult {
    pub content: Vec<ToolContent>,
    #[serde(rename = "isError", skip_serializing_if = "std::ops::Not::not")]
    pub is_error: bool,
}

impl ToolCallResult {
    pub fn success(text: impl Into<String>) -> Self {
        Self {
            content: vec![ToolContent {
                content_type: "text".to_string(),
                text: text.into(),
            }],
            is_error: false,
        }
    }

    pub fn error(err: impl Into<String>) -> Self {
        Self {
            content: vec![ToolContent {
                content_type: "text".to_string(),
                text: format!("Error: {}", err.into()),
            }],
            is_error: true,
        }
    }
}

/// Backend interface providing operational access for MCP tools.
pub trait McpBackend: Send + Sync {
    /// Control dictation session: 'start', 'stop', or 'cancel'.
    fn dictate(&self, action: &str, mode: Option<&str>) -> Result<String, String>;

    /// Query history entries from the local SQLite store.
    fn get_history(&self, query: Option<&str>, limit: Option<usize>) -> Result<Vec<Value>, String>;

    /// Add custom domain jargon / spoken-to-written mapping.
    fn add_vocabulary(&self, spoken: &str, written: &str) -> Result<String, String>;

    /// Switch the active ASR model identifier.
    fn switch_model(&self, model_id: &str) -> Result<String, String>;

    /// Retrieve runtime daemon status and hardware capabilities.
    fn get_system_status(&self) -> Result<Value, String>;
}

/// Vocabulary replacement pair.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VocabularyEntry {
    pub spoken: String,
    pub written: String,
}

/// In-memory daemon runtime state for LocalMcpBackend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalDaemonState {
    pub state: String,
    pub active_mode: String,
    pub active_model: String,
    pub vocabulary: Vec<VocabularyEntry>,
}

impl Default for LocalDaemonState {
    fn default() -> Self {
        Self {
            state: "idle".to_string(),
            active_mode: "plain".to_string(),
            active_model: "parakeet-ctc-110m-int8".to_string(),
            vocabulary: Vec::new(),
        }
    }
}

/// Concrete local backend connected to SQLite HistoryStore and runtime state.
pub struct LocalMcpBackend {
    store: Arc<Mutex<HistoryStore>>,
    state: Arc<Mutex<LocalDaemonState>>,
}

impl LocalMcpBackend {
    pub fn new(store: HistoryStore) -> Self {
        Self {
            store: Arc::new(Mutex::new(store)),
            state: Arc::new(Mutex::new(LocalDaemonState::default())),
        }
    }

    pub fn with_store_and_state(
        store: Arc<Mutex<HistoryStore>>,
        state: Arc<Mutex<LocalDaemonState>>,
    ) -> Self {
        Self { store, state }
    }

    pub fn state(&self) -> &Arc<Mutex<LocalDaemonState>> {
        &self.state
    }

    pub fn store(&self) -> &Arc<Mutex<HistoryStore>> {
        &self.store
    }
}

impl McpBackend for LocalMcpBackend {
    fn dictate(&self, action: &str, mode: Option<&str>) -> Result<String, String> {
        let action_clean = action.trim().to_lowercase();
        let mut st = self.state.lock().map_err(|e| e.to_string())?;

        match action_clean.as_str() {
            "start" => {
                st.state = "listening".to_string();
                if let Some(m) = mode {
                    if !m.trim().is_empty() {
                        st.active_mode = m.trim().to_string();
                    }
                }
                Ok(format!("Dictation started in '{}' mode", st.active_mode))
            }
            "stop" => {
                st.state = "idle".to_string();
                Ok("Dictation stopped".to_string())
            }
            "cancel" => {
                st.state = "idle".to_string();
                Ok("Dictation cancelled".to_string())
            }
            other => Err(format!(
                "Invalid action: '{other}'. Expected 'start', 'stop', or 'cancel'."
            )),
        }
    }

    fn get_history(&self, query: Option<&str>, limit: Option<usize>) -> Result<Vec<Value>, String> {
        let store = self.store.lock().map_err(|e| e.to_string())?;
        let lim = limit.unwrap_or(10).min(500) as u32;

        let entries = match query {
            Some(q) if !q.trim().is_empty() => {
                store.search(q.trim(), 0, lim).map_err(|e| e.to_string())?
            }
            _ => store.list(0, lim).map_err(|e| e.to_string())?,
        };

        entries
            .into_iter()
            .map(|entry| serde_json::to_value(&entry).map_err(|e| e.to_string()))
            .collect()
    }

    fn add_vocabulary(&self, spoken: &str, written: &str) -> Result<String, String> {
        let spoken_clean = spoken.trim();
        let written_clean = written.trim();

        if spoken_clean.is_empty() || written_clean.is_empty() {
            return Err("Both 'spoken' and 'written' parameters must be non-empty.".to_string());
        }

        let mut st = self.state.lock().map_err(|e| e.to_string())?;
        if let Some(existing) = st
            .vocabulary
            .iter_mut()
            .find(|v| v.spoken.eq_ignore_ascii_case(spoken_clean))
        {
            existing.written = written_clean.to_string();
        } else {
            st.vocabulary.push(VocabularyEntry {
                spoken: spoken_clean.to_string(),
                written: written_clean.to_string(),
            });
        }

        Ok(format!(
            "Registered vocabulary: '{}' -> '{}'",
            spoken_clean, written_clean
        ))
    }

    fn switch_model(&self, model_id: &str) -> Result<String, String> {
        let model_clean = model_id.trim();
        if model_clean.is_empty() {
            return Err("Parameter 'model_id' cannot be empty.".to_string());
        }

        let mut st = self.state.lock().map_err(|e| e.to_string())?;
        st.active_model = model_clean.to_string();
        Ok(format!("Switched active model to '{}'", model_clean))
    }

    fn get_system_status(&self) -> Result<Value, String> {
        let st = self.state.lock().map_err(|e| e.to_string())?;
        let hw = detect_hardware_capabilities();

        Ok(json!({
            "state": st.state,
            "active_mode": st.active_mode,
            "active_model": st.active_model,
            "hardware_capabilities": hw,
        }))
    }
}

/// Detects local CPU architecture, core count, SIMD extensions, and execution provider.
pub fn detect_hardware_capabilities() -> Value {
    let cores = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);
    let arch = std::env::consts::ARCH;
    let os = std::env::consts::OS;

    #[cfg(target_arch = "x86_64")]
    let (avx2, avx512, fma) = {
        (
            std::is_x86_feature_detected!("avx2"),
            std::is_x86_feature_detected!("avx512f"),
            std::is_x86_feature_detected!("fma"),
        )
    };

    #[cfg(not(target_arch = "x86_64"))]
    let (avx2, avx512, fma) = (false, false, false);

    json!({
        "arch": arch,
        "os": os,
        "cpu_cores": cores,
        "avx2": avx2,
        "avx512": avx512,
        "fma": fma,
        "provider": "cpu",
    })
}

/// Returns the 5 tool definitions exposed by Narad MCP server.
pub fn get_tool_definitions() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "narad_dictate".to_string(),
            description: "Start, stop, or cancel local voice dictation in Narad.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["start", "stop", "cancel"],
                        "description": "Dictation action to perform: 'start', 'stop', or 'cancel'."
                    },
                    "mode": {
                        "type": "string",
                        "description": "Optional dictation mode (e.g. 'plain', 'code', 'command')."
                    }
                },
                "required": ["action"]
            }),
        },
        ToolDefinition {
            name: "narad_get_history".to_string(),
            description: "Retrieve transcription history records from the local SQLite store.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Optional search query to filter transcriptions."
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Optional maximum number of records to return (defaults to 10)."
                    }
                }
            }),
        },
        ToolDefinition {
            name: "narad_add_vocabulary".to_string(),
            description: "Register custom jargon and domain vocabulary for speech recognition.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "spoken": {
                        "type": "string",
                        "description": "The phonetic or spoken form of the word or phrase."
                    },
                    "written": {
                        "type": "string",
                        "description": "The exact written representation to output."
                    }
                },
                "required": ["spoken", "written"]
            }),
        },
        ToolDefinition {
            name: "narad_switch_model".to_string(),
            description: "Switch the active speech recognition (ASR) model.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "model_id": {
                        "type": "string",
                        "description": "Identifier of the model pack to switch to."
                    }
                },
                "required": ["model_id"]
            }),
        },
        ToolDefinition {
            name: "narad_get_system_status".to_string(),
            description: "Get current daemon state, active model, and hardware capabilities.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {}
            }),
        },
    ]
}

/// Model Context Protocol Server dispatching JSON-RPC 2.0 requests to the backend.
pub struct McpServer<B: McpBackend = LocalMcpBackend> {
    backend: Arc<B>,
}

impl<B: McpBackend> Clone for McpServer<B> {
    fn clone(&self) -> Self {
        Self {
            backend: self.backend.clone(),
        }
    }
}

impl<B: McpBackend + 'static> McpServer<B> {
    pub fn new(backend: B) -> Self {
        Self {
            backend: Arc::new(backend),
        }
    }

    pub fn from_arc(backend: Arc<B>) -> Self {
        Self { backend }
    }

    pub fn backend(&self) -> &Arc<B> {
        &self.backend
    }

    /// Process a single incoming JSON-RPC raw text payload.
    /// Returns `Some(response_json)` if the request requires a response, or `None` if notification.
    pub async fn handle_message(&self, raw: &str) -> Option<String> {
        let parsed: Value = match serde_json::from_str(raw) {
            Ok(v) => v,
            Err(e) => {
                let err_resp = JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: Value::Null,
                    result: None,
                    error: Some(JsonRpcError::parse_error(Some(Value::String(e.to_string())))),
                };
                return serde_json::to_string(&err_resp).ok();
            }
        };

        if let Some(arr) = parsed.as_array() {
            let mut responses = Vec::new();
            for item in arr {
                if let Some(resp) = self.handle_single_request(item).await {
                    responses.push(resp);
                }
            }
            if responses.is_empty() {
                None
            } else {
                serde_json::to_string(&responses).ok()
            }
        } else {
            let resp = self.handle_single_request(&parsed).await?;
            serde_json::to_string(&resp).ok()
        }
    }

    async fn handle_single_request(&self, val: &Value) -> Option<JsonRpcResponse> {
        let req: JsonRpcRequest = match serde_json::from_value(val.clone()) {
            Ok(r) => r,
            Err(err) => {
                let id = val.get("id").cloned().unwrap_or(Value::Null);
                return Some(JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id,
                    result: None,
                    error: Some(JsonRpcError::invalid_request(&err.to_string())),
                });
            }
        };

        let is_notification = req.id.is_none();
        let id = req.id.unwrap_or(Value::Null);

        match req.method.as_str() {
            "initialize" => {
                let result = json!({
                    "protocolVersion": PROTOCOL_VERSION,
                    "capabilities": {
                        "tools": {}
                    },
                    "serverInfo": {
                        "name": SERVER_NAME,
                        "version": SERVER_VERSION,
                    }
                });
                Some(JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id,
                    result: Some(result),
                    error: None,
                })
            }
            "notifications/initialized" => {
                debug!("MCP client initialized notification received");
                None
            }
            "ping" => Some(JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(json!({})),
                error: None,
            }),
            "tools/list" => {
                let tools = get_tool_definitions();
                Some(JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id,
                    result: Some(json!({ "tools": tools })),
                    error: None,
                })
            }
            "tools/call" => {
                let params = match req.params {
                    Some(p) => p,
                    None => {
                        return Some(JsonRpcResponse {
                            jsonrpc: "2.0".to_string(),
                            id,
                            result: None,
                            error: Some(JsonRpcError::invalid_params("Missing params object")),
                        });
                    }
                };

                let tool_name = match params.get("name").and_then(|v| v.as_str()) {
                    Some(n) => n,
                    None => {
                        return Some(JsonRpcResponse {
                            jsonrpc: "2.0".to_string(),
                            id,
                            result: None,
                            error: Some(JsonRpcError::invalid_params("Missing or non-string tool 'name'")),
                        });
                    }
                };

                let arguments = params.get("arguments");
                let tool_result = self.execute_tool(tool_name, arguments).await;

                Some(JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id,
                    result: Some(serde_json::to_value(tool_result).unwrap_or(Value::Null)),
                    error: None,
                })
            }
            unknown => {
                if is_notification {
                    None
                } else {
                    Some(JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        id,
                        result: None,
                        error: Some(JsonRpcError::method_not_found(unknown)),
                    })
                }
            }
        }
    }

    async fn execute_tool(&self, name: &str, arguments: Option<&Value>) -> ToolCallResult {
        match name {
            "narad_dictate" => {
                let args = match arguments {
                    Some(a) => a,
                    None => return ToolCallResult::error("Missing arguments for narad_dictate"),
                };
                let action = match args.get("action").and_then(|v| v.as_str()) {
                    Some(a) => a,
                    None => {
                        return ToolCallResult::error("Missing required string parameter 'action'")
                    }
                };
                let mode = args.get("mode").and_then(|v| v.as_str());
                match self.backend.dictate(action, mode) {
                    Ok(msg) => ToolCallResult::success(msg),
                    Err(err) => ToolCallResult::error(err),
                }
            }
            "narad_get_history" => {
                let (query, limit) = match arguments {
                    Some(args) => {
                        let q = args.get("query").and_then(|v| v.as_str());
                        let l = args.get("limit").and_then(|v| v.as_u64()).map(|u| u as usize);
                        (q, l)
                    }
                    None => (None, None),
                };

                match self.backend.get_history(query, limit) {
                    Ok(entries) => match serde_json::to_string_pretty(&entries) {
                        Ok(json_str) => ToolCallResult::success(json_str),
                        Err(e) => ToolCallResult::error(format!("Serialization failed: {e}")),
                    },
                    Err(err) => ToolCallResult::error(err),
                }
            }
            "narad_add_vocabulary" => {
                let args = match arguments {
                    Some(a) => a,
                    None => return ToolCallResult::error("Missing arguments for narad_add_vocabulary"),
                };
                let spoken = match args.get("spoken").and_then(|v| v.as_str()) {
                    Some(s) => s,
                    None => {
                        return ToolCallResult::error("Missing required string parameter 'spoken'")
                    }
                };
                let written = match args.get("written").and_then(|v| v.as_str()) {
                    Some(w) => w,
                    None => {
                        return ToolCallResult::error("Missing required string parameter 'written'")
                    }
                };
                match self.backend.add_vocabulary(spoken, written) {
                    Ok(msg) => ToolCallResult::success(msg),
                    Err(err) => ToolCallResult::error(err),
                }
            }
            "narad_switch_model" => {
                let args = match arguments {
                    Some(a) => a,
                    None => return ToolCallResult::error("Missing arguments for narad_switch_model"),
                };
                let model_id = match args.get("model_id").and_then(|v| v.as_str()) {
                    Some(m) => m,
                    None => {
                        return ToolCallResult::error("Missing required string parameter 'model_id'")
                    }
                };
                match self.backend.switch_model(model_id) {
                    Ok(msg) => ToolCallResult::success(msg),
                    Err(err) => ToolCallResult::error(err),
                }
            }
            "narad_get_system_status" => match self.backend.get_system_status() {
                Ok(status) => match serde_json::to_string_pretty(&status) {
                    Ok(json_str) => ToolCallResult::success(json_str),
                    Err(e) => ToolCallResult::error(format!("Serialization failed: {e}")),
                },
                Err(err) => ToolCallResult::error(err),
            },
            other => ToolCallResult::error(format!("Unknown tool: '{other}'")),
        }
    }

    /// Run the MCP server over an async reader and writer.
    pub async fn run<R, W>(&self, reader: R, mut writer: W) -> std::io::Result<()>
    where
        R: AsyncRead + Unpin + Send + 'static,
        W: AsyncWrite + Unpin + Send + 'static,
    {
        let mut lines = BufReader::new(reader).lines();
        while let Some(line) = lines.next_line().await? {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if let Some(response) = self.handle_message(trimmed).await {
                writer.write_all(response.as_bytes()).await?;
                writer.write_all(b"\n").await?;
                writer.flush().await?;
            }
        }
        Ok(())
    }

    /// Run the MCP server over process standard input and output.
    pub async fn run_stdio(&self) -> std::io::Result<()> {
        info!("Starting Narad MCP server on stdio");
        self.run(tokio::io::stdin(), tokio::io::stdout()).await
    }

    /// Run the MCP server over a local Unix domain socket.
    pub async fn run_socket<P: AsRef<Path>>(&self, path: P) -> std::io::Result<()> {
        let p = path.as_ref();
        if p.exists() {
            let _ = std::fs::remove_file(p);
        }
        if let Some(parent) = p.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let listener = tokio::net::UnixListener::bind(p)?;
        info!(socket = %p.display(), "Narad MCP server listening on Unix socket");

        loop {
            let (stream, _) = listener.accept().await?;
            let (reader, writer) = stream.into_split();
            let srv = self.clone();
            tokio::spawn(async move {
                if let Err(err) = srv.run(reader, writer).await {
                    warn!(%err, "MCP client connection error");
                }
            });
        }
    }
}
