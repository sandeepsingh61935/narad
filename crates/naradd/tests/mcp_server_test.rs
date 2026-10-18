//! MCP Server integration tests (FR-MCP-1, ADR-005).
//!
//! Tests JSON-RPC 2.0 initialize handshake, tools/list, and tools/call
//! for all 5 tools over asynchronous memory streams (`tokio::io::duplex`).

use std::sync::Arc;
use narad_core::store::{HistoryStore, InsertStatus, NewHistoryEntry};
use naradd::mcp::{
    LocalMcpBackend, McpServer, PROTOCOL_VERSION, SERVER_NAME, SERVER_VERSION,
};
use serde_json::json;
use tempfile::TempDir;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

struct TestMcpClient {
    reader: BufReader<tokio::io::ReadHalf<tokio::io::DuplexStream>>,
    writer: tokio::io::WriteHalf<tokio::io::DuplexStream>,
    next_id: u64,
}

impl TestMcpClient {
    async fn call(&mut self, method: &str, params: Option<serde_json::Value>) -> serde_json::Value {
        let id = self.next_id;
        self.next_id += 1;

        let req = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params.unwrap_or(json!({})),
        });

        let mut req_str = serde_json::to_string(&req).unwrap();
        req_str.push('\n');
        self.writer.write_all(req_str.as_bytes()).await.unwrap();
        self.writer.flush().await.unwrap();

        let mut line = String::new();
        self.reader.read_line(&mut line).await.unwrap();
        let resp: serde_json::Value = serde_json::from_str(&line)
            .unwrap_or_else(|e| panic!("Failed to parse response: {e}, raw line: {line}"));

        assert_eq!(resp.get("jsonrpc").and_then(|v| v.as_str()), Some("2.0"));
        assert_eq!(resp.get("id").and_then(|v| v.as_u64()), Some(id));
        resp
    }

    async fn notify(&mut self, method: &str, params: Option<serde_json::Value>) {
        let req = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params.unwrap_or(json!({})),
        });

        let mut req_str = serde_json::to_string(&req).unwrap();
        req_str.push('\n');
        self.writer.write_all(req_str.as_bytes()).await.unwrap();
        self.writer.flush().await.unwrap();
    }
}

async fn setup_test_server() -> (TestMcpClient, Arc<LocalMcpBackend>, TempDir) {
    let temp_dir = TempDir::new().expect("tempdir");
    let db_path = temp_dir.path().join("mcp_test.db");
    let store = HistoryStore::open(&db_path).expect("open store");

    let backend = Arc::new(LocalMcpBackend::new(store));
    let server = McpServer::from_arc(backend.clone());

    let (client_stream, server_stream) = tokio::io::duplex(8192);
    let (server_reader, server_writer) = tokio::io::split(server_stream);
    let (client_reader, client_writer) = tokio::io::split(client_stream);

    tokio::spawn(async move {
        let _ = server.run(server_reader, server_writer).await;
    });

    let client = TestMcpClient {
        reader: BufReader::new(client_reader),
        writer: client_writer,
        next_id: 1,
    };

    (client, backend, temp_dir)
}

#[tokio::test]
async fn test_mcp_initialize_handshake() {
    let (mut client, _backend, _temp) = setup_test_server().await;

    let init_resp = client
        .call(
            "initialize",
            Some(json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {
                    "name": "antigravity-coding-agent",
                    "version": "1.0.0"
                }
            })),
        )
        .await;

    let result = init_resp.get("result").expect("initialize result missing");
    assert_eq!(
        result.get("protocolVersion").and_then(|v| v.as_str()),
        Some(PROTOCOL_VERSION)
    );

    let capabilities = result.get("capabilities").expect("capabilities missing");
    assert!(capabilities.get("tools").is_some());

    let server_info = result.get("serverInfo").expect("serverInfo missing");
    assert_eq!(
        server_info.get("name").and_then(|v| v.as_str()),
        Some(SERVER_NAME)
    );
    assert_eq!(
        server_info.get("version").and_then(|v| v.as_str()),
        Some(SERVER_VERSION)
    );

    // Send post-initialization notification (must not produce response)
    client.notify("notifications/initialized", None).await;

    // Ping server to confirm stream continues normally
    let ping_resp = client.call("ping", None).await;
    assert_eq!(ping_resp.get("result"), Some(&json!({})));
}

#[tokio::test]
async fn test_mcp_tools_list() {
    let (mut client, _backend, _temp) = setup_test_server().await;

    // Handshake first
    client
        .call(
            "initialize",
            Some(json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": { "name": "test-agent", "version": "1.0" }
            })),
        )
        .await;
    client.notify("notifications/initialized", None).await;

    let list_resp = client.call("tools/list", None).await;
    let result = list_resp.get("result").expect("tools/list result");
    let tools = result.get("tools").and_then(|v| v.as_array()).expect("tools array");

    assert_eq!(tools.len(), 5);

    let tool_names: Vec<&str> = tools
        .iter()
        .filter_map(|t| t.get("name").and_then(|n| n.as_str()))
        .collect();

    assert!(tool_names.contains(&"narad_dictate"));
    assert!(tool_names.contains(&"narad_get_history"));
    assert!(tool_names.contains(&"narad_add_vocabulary"));
    assert!(tool_names.contains(&"narad_switch_model"));
    assert!(tool_names.contains(&"narad_get_system_status"));

    // Check schema for narad_dictate
    let dictate_tool = tools.iter().find(|t| t["name"] == "narad_dictate").unwrap();
    assert!(dictate_tool["inputSchema"]["properties"]["action"].is_object());
    assert!(dictate_tool["inputSchema"]["properties"]["mode"].is_object());
    assert_eq!(dictate_tool["inputSchema"]["required"], json!(["action"]));

    // Check schema for narad_add_vocabulary
    let vocab_tool = tools.iter().find(|t| t["name"] == "narad_add_vocabulary").unwrap();
    assert!(vocab_tool["inputSchema"]["properties"]["spoken"].is_object());
    assert!(vocab_tool["inputSchema"]["properties"]["written"].is_object());
    assert_eq!(vocab_tool["inputSchema"]["required"], json!(["spoken", "written"]));
}

#[tokio::test]
async fn test_mcp_tools_call_all_five_tools() {
    let (mut client, backend, _temp) = setup_test_server().await;

    // Handshake
    client
        .call(
            "initialize",
            Some(json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": { "name": "test-agent", "version": "1.0" }
            })),
        )
        .await;
    client.notify("notifications/initialized", None).await;

    // 1. Test `narad_dictate`: actions ("start", "stop", "cancel"), optional "mode"
    let start_resp = client
        .call(
            "tools/call",
            Some(json!({
                "name": "narad_dictate",
                "arguments": {
                    "action": "start",
                    "mode": "code"
                }
            })),
        )
        .await;
    let start_res = start_resp.get("result").expect("start result");
    assert_eq!(start_res.get("isError"), None);
    let start_text = start_res["content"][0]["text"].as_str().unwrap();
    assert!(start_text.contains("code"));

    // Verify system state reflects 'listening'
    let status_after_start = client
        .call(
            "tools/call",
            Some(json!({
                "name": "narad_get_system_status",
                "arguments": {}
            })),
        )
        .await;
    let status_text = status_after_start["result"]["content"][0]["text"].as_str().unwrap();
    let status_json: serde_json::Value = serde_json::from_str(status_text).unwrap();
    assert_eq!(status_json["state"], "listening");
    assert_eq!(status_json["active_mode"], "code");

    // Stop dictate
    let stop_resp = client
        .call(
            "tools/call",
            Some(json!({
                "name": "narad_dictate",
                "arguments": {
                    "action": "stop"
                }
            })),
        )
        .await;
    let stop_res = stop_resp.get("result").expect("stop result");
    assert_eq!(stop_res.get("isError"), None);
    assert!(stop_res["content"][0]["text"].as_str().unwrap().contains("stopped"));

    // Cancel dictate
    let cancel_resp = client
        .call(
            "tools/call",
            Some(json!({
                "name": "narad_dictate",
                "arguments": {
                    "action": "cancel"
                }
            })),
        )
        .await;
    let cancel_res = cancel_resp.get("result").expect("cancel result");
    assert_eq!(cancel_res.get("isError"), None);
    assert!(cancel_res["content"][0]["text"].as_str().unwrap().contains("cancelled"));

    // Invalid dictate action
    let bad_dictate = client
        .call(
            "tools/call",
            Some(json!({
                "name": "narad_dictate",
                "arguments": {
                    "action": "invalid_action"
                }
            })),
        )
        .await;
    assert_eq!(bad_dictate["result"]["isError"], true);

    // 2. Test `narad_get_history`: optional "query", optional "limit"
    // First, populate SQLite store with entries
    {
        let mut store = backend.store().lock().unwrap();
        store
            .insert(&NewHistoryEntry {
                duration_ms: 1500,
                mode: "plain".into(),
                src_lang: "en".into(),
                dst_lang: "en".into(),
                asr_model: "parakeet-ctc-110m-int8".into(),
                mt_model: None,
                llm_model: None,
                app_id: Some("org.gnome.TextEditor".into()),
                raw_text: "welcome to local speech recognition".into(),
                final_text: "Welcome to local speech recognition.".into(),
                insert_status: InsertStatus::Success,
                cleanup_flagged: false,
                audio_path: None,
                pinned: false,
                kind: "dictation".into(),
            })
            .expect("insert 1");

        store
            .insert(&NewHistoryEntry {
                duration_ms: 2200,
                mode: "code".into(),
                src_lang: "en".into(),
                dst_lang: "en".into(),
                asr_model: "parakeet-ctc-110m-int8".into(),
                mt_model: None,
                llm_model: None,
                app_id: Some("code".into()),
                raw_text: "let mut socket listener bind".into(),
                final_text: "let mut socket_listener = bind();".into(),
                insert_status: InsertStatus::Success,
                cleanup_flagged: false,
                audio_path: None,
                pinned: true,
                kind: "dictation".into(),
            })
            .expect("insert 2");
    }

    // Call get_history without query
    let hist_all = client
        .call(
            "tools/call",
            Some(json!({
                "name": "narad_get_history",
                "arguments": { "limit": 10 }
            })),
        )
        .await;
    let all_text = hist_all["result"]["content"][0]["text"].as_str().unwrap();
    let all_entries: Vec<serde_json::Value> = serde_json::from_str(all_text).unwrap();
    assert_eq!(all_entries.len(), 2);

    // Call get_history with query matching "socket"
    let hist_search = client
        .call(
            "tools/call",
            Some(json!({
                "name": "narad_get_history",
                "arguments": { "query": "socket" }
            })),
        )
        .await;
    let search_text = hist_search["result"]["content"][0]["text"].as_str().unwrap();
    let search_entries: Vec<serde_json::Value> = serde_json::from_str(search_text).unwrap();
    assert_eq!(search_entries.len(), 1);
    assert!(search_entries[0]["final_text"].as_str().unwrap().contains("socket_listener"));

    // Call get_history with limit 1
    let hist_limit = client
        .call(
            "tools/call",
            Some(json!({
                "name": "narad_get_history",
                "arguments": { "limit": 1 }
            })),
        )
        .await;
    let limit_text = hist_limit["result"]["content"][0]["text"].as_str().unwrap();
    let limit_entries: Vec<serde_json::Value> = serde_json::from_str(limit_text).unwrap();
    assert_eq!(limit_entries.len(), 1);

    // 3. Test `narad_add_vocabulary`: "spoken" and "written" parameters
    let vocab_resp = client
        .call(
            "tools/call",
            Some(json!({
                "name": "narad_add_vocabulary",
                "arguments": {
                    "spoken": "antigravity",
                    "written": "Antigravity AI"
                }
            })),
        )
        .await;
    let vocab_res = vocab_resp.get("result").expect("vocab result");
    assert_eq!(vocab_res.get("isError"), None);
    let vocab_text = vocab_res["content"][0]["text"].as_str().unwrap();
    assert!(vocab_text.contains("antigravity") && vocab_text.contains("Antigravity AI"));

    // Check backend state holds the registered vocabulary
    {
        let st = backend.state().lock().unwrap();
        assert_eq!(st.vocabulary.len(), 1);
        assert_eq!(st.vocabulary[0].spoken, "antigravity");
        assert_eq!(st.vocabulary[0].written, "Antigravity AI");
    }

    // Invalid vocabulary with empty field
    let bad_vocab = client
        .call(
            "tools/call",
            Some(json!({
                "name": "narad_add_vocabulary",
                "arguments": {
                    "spoken": "",
                    "written": "Test"
                }
            })),
        )
        .await;
    assert_eq!(bad_vocab["result"]["isError"], true);

    // 4. Test `narad_switch_model`: "model_id" to switch
    let switch_resp = client
        .call(
            "tools/call",
            Some(json!({
                "name": "narad_switch_model",
                "arguments": {
                    "model_id": "sherpa-onnx-nemo-parakeet-tdt-0.6b-v2-int8"
                }
            })),
        )
        .await;
    let switch_res = switch_resp.get("result").expect("switch result");
    assert_eq!(switch_res.get("isError"), None);
    assert!(switch_res["content"][0]["text"]
        .as_str()
        .unwrap()
        .contains("sherpa-onnx-nemo-parakeet-tdt-0.6b-v2-int8"));

    // Verify active_model in backend state
    {
        let st = backend.state().lock().unwrap();
        assert_eq!(st.active_model, "sherpa-onnx-nemo-parakeet-tdt-0.6b-v2-int8");
    }

    // Invalid model switch with empty model_id
    let bad_switch = client
        .call(
            "tools/call",
            Some(json!({
                "name": "narad_switch_model",
                "arguments": { "model_id": "  " }
            })),
        )
        .await;
    assert_eq!(bad_switch["result"]["isError"], true);

    // 5. Test `narad_get_system_status`: returns state, active_model, and hardware capabilities
    let status_resp = client
        .call(
            "tools/call",
            Some(json!({
                "name": "narad_get_system_status",
                "arguments": {}
            })),
        )
        .await;
    let status_res = status_resp.get("result").expect("status result");
    assert_eq!(status_res.get("isError"), None);
    let st_str = status_res["content"][0]["text"].as_str().unwrap();
    let status: serde_json::Value = serde_json::from_str(st_str).unwrap();

    assert_eq!(status["state"], "idle");
    assert_eq!(status["active_model"], "sherpa-onnx-nemo-parakeet-tdt-0.6b-v2-int8");
    assert_eq!(status["active_mode"], "code");

    let hw = status.get("hardware_capabilities").expect("hardware_capabilities");
    assert!(hw.get("arch").is_some());
    assert!(hw.get("os").is_some());
    assert!(hw.get("cpu_cores").and_then(|v| v.as_u64()).is_some());
    assert_eq!(hw.get("provider").and_then(|v| v.as_str()), Some("cpu"));
}

#[tokio::test]
async fn test_mcp_unknown_tool_and_method() {
    let (mut client, _backend, _temp) = setup_test_server().await;

    // Unknown tool
    let unknown_tool = client
        .call(
            "tools/call",
            Some(json!({
                "name": "non_existent_tool",
                "arguments": {}
            })),
        )
        .await;
    assert_eq!(unknown_tool["result"]["isError"], true);

    // Unknown method
    let unknown_method = client.call("non_existent_method", None).await;
    assert!(unknown_method.get("error").is_some());
    assert_eq!(unknown_method["error"]["code"], -32601);
}
