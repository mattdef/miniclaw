//! Integration tests for the tool system
//!
//! These tests verify the end-to-end functionality of the tool system including:
//! - Tool registration and discovery
//! - Tool execution flow
//! - Integration with AgentLoop
//! - Error handling and propagation

use miniclaw::agent::tools::{
    Tool, ToolDefinition, ToolError, ToolExecutionContext, ToolRegistry, ToolResult,
};
use serde_json::{Value, json};
use std::collections::HashMap;

/// Mock tool for testing tool system integration
struct TestTool {
    name: String,
    description: String,
    should_fail: bool,
}

impl TestTool {
    fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            should_fail: false,
        }
    }

    fn failing(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            should_fail: true,
        }
    }
}

#[async_trait::async_trait]
impl Tool for TestTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "input": {
                    "type": "string",
                    "description": "Input to process"
                },
                "multiplier": {
                    "type": "integer",
                    "description": "Optional multiplier",
                    "default": 1
                }
            },
            "required": ["input"]
        })
    }

    async fn execute(
        &self,
        args: HashMap<String, Value>,
        _ctx: &ToolExecutionContext,
    ) -> ToolResult<String> {
        if self.should_fail {
            return Err(ToolError::ExecutionFailed {
                tool: self.name.clone(),
                message: "Simulated failure".to_string(),
            });
        }

        let input = args.get("input").and_then(|v| v.as_str()).ok_or_else(|| {
            ToolError::InvalidArguments {
                tool: self.name.clone(),
                message: "Missing required 'input' parameter".to_string(),
            }
        })?;

        let multiplier = args.get("multiplier").and_then(|v| v.as_u64()).unwrap_or(1);

        Ok(format!("{} x {}", input, multiplier))
    }
}

/// Integration test: Complete tool registration and execution flow
#[tokio::test]
async fn test_tool_registration_and_execution_flow() {
    let registry = ToolRegistry::new();

    // Register multiple tools
    let tool1 = TestTool::new("multiply", "Multiplies input");
    let tool2 = TestTool::new("echo", "Echoes input");

    registry.register(Box::new(tool1)).unwrap();
    registry.register(Box::new(tool2)).unwrap();

    assert_eq!(registry.len(), 2);

    // Execute first tool
    let mut args1 = HashMap::new();
    args1.insert("input".to_string(), json!("hello"));
    args1.insert("multiplier".to_string(), json!(3));

    let ctx = ToolExecutionContext::default();
    let result1 = registry.execute_tool("multiply", args1, &ctx).await;
    assert!(result1.is_ok());
    assert_eq!(result1.unwrap(), "hello x 3");

    // Execute second tool
    let mut args2 = HashMap::new();
    args2.insert("input".to_string(), json!("world"));

    let result2 = registry.execute_tool("echo", args2, &ctx).await;
    assert!(result2.is_ok());
    assert_eq!(result2.unwrap(), "world x 1");
}

/// Integration test: Tool listing and discovery
#[test]
fn test_tool_discovery_via_listing() {
    let registry = ToolRegistry::new();

    registry
        .register(Box::new(TestTool::new("tool_a", "First tool")))
        .unwrap();
    registry
        .register(Box::new(TestTool::new("tool_b", "Second tool")))
        .unwrap();
    registry
        .register(Box::new(TestTool::new("tool_c", "Third tool")))
        .unwrap();

    // Test list_tools()
    let tools = registry.list_tools();
    assert_eq!(tools.len(), 3);

    // Verify all tools are listed
    let names: Vec<String> = tools.iter().map(|(n, _, _)| n.clone()).collect();
    assert!(names.contains(&"tool_a".to_string()));
    assert!(names.contains(&"tool_b".to_string()));
    assert!(names.contains(&"tool_c".to_string()));

    // Test contains()
    assert!(registry.contains("tool_a"));
    assert!(registry.contains("tool_b"));
    assert!(registry.contains("tool_c"));
    assert!(!registry.contains("tool_d"));
}

/// Integration test: Tool definitions for LLM
#[test]
fn test_tool_definitions_for_llm() {
    let registry = ToolRegistry::new();

    registry
        .register(Box::new(TestTool::new(
            "calculator",
            "Performs calculations",
        )))
        .unwrap();

    // Get JSON definitions
    let definitions = registry.get_tool_definitions();
    assert_eq!(definitions.len(), 1);

    let def = &definitions[0];
    assert_eq!(def["type"], "function");
    assert_eq!(def["function"]["name"], "calculator");
    assert_eq!(def["function"]["description"], "Performs calculations");
    assert!(
        def["function"]["parameters"]["properties"]
            .get("input")
            .is_some()
    );

    // Get typed definitions
    let typed_defs = registry.get_definitions();
    assert_eq!(typed_defs.len(), 1);
    assert_eq!(typed_defs[0].name(), "calculator");
    assert_eq!(typed_defs[0].r#type, "function");
}

/// Integration test: Error propagation
#[tokio::test]
async fn test_tool_error_propagation() {
    let registry = ToolRegistry::new();

    // Register a failing tool
    registry
        .register(Box::new(TestTool::failing("failing_tool", "Always fails")))
        .unwrap();

    // Try to execute it
    let mut args = HashMap::new();
    args.insert("input".to_string(), json!("test"));

    let ctx = ToolExecutionContext::default();
    let result = registry.execute_tool("failing_tool", args, &ctx).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        ToolError::ExecutionFailed { tool, message } => {
            assert_eq!(tool, "failing_tool");
            assert_eq!(message, "Simulated failure");
        }
        _ => panic!("Expected ExecutionFailed error"),
    }
}

/// Integration test: Tool not found error
#[tokio::test]
async fn test_tool_not_found_error() {
    let registry = ToolRegistry::new();

    let args = HashMap::new();
    let ctx = ToolExecutionContext::default();
    let result = registry.execute_tool("nonexistent_tool", args, &ctx).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        ToolError::NotFound(tool) => {
            assert_eq!(tool, "nonexistent_tool");
        }
        _ => panic!("Expected NotFound error"),
    }
}

/// Integration test: Tool registration conflict
#[test]
fn test_tool_registration_conflict() {
    let registry = ToolRegistry::new();

    let tool1 = TestTool::new("unique_tool", "First registration");
    let tool2 = TestTool::new("unique_tool", "Duplicate name");

    // First registration succeeds
    registry.register(Box::new(tool1)).unwrap();

    // Second registration fails with same name
    let result = registry.register(Box::new(tool2));
    assert!(result.is_err());

    match result.unwrap_err() {
        ToolError::ExecutionFailed { tool, message } => {
            assert_eq!(tool, "unique_tool");
            assert!(message.contains("already registered"));
        }
        _ => panic!("Expected ExecutionFailed error with conflict message"),
    }
}

/// Integration test: Tool unregistration
#[tokio::test]
async fn test_tool_unregistration() {
    let registry = ToolRegistry::new();

    registry
        .register(Box::new(TestTool::new("temp_tool", "Temporary tool")))
        .unwrap();
    assert_eq!(registry.len(), 1);
    assert!(registry.contains("temp_tool"));

    // Unregister the tool
    let removed = registry.unregister("temp_tool");
    assert!(removed);
    assert_eq!(registry.len(), 0);
    assert!(!registry.contains("temp_tool"));

    // Trying to unregister again returns false
    let removed_again = registry.unregister("temp_tool");
    assert!(!removed_again);

    // Trying to execute unregistered tool fails
    let args = HashMap::new();
    let ctx = ToolExecutionContext::default();
    let result = registry.execute_tool("temp_tool", args, &ctx).await;
    assert!(matches!(result.unwrap_err(), ToolError::NotFound(_)));
}

/// Integration test: Concurrent tool registration (thread safety)
#[test]
fn test_concurrent_tool_access() {
    use std::sync::Arc;
    use std::thread;

    // ToolRegistry is now thread-safe with internal RwLock, no need for external Mutex
    let registry = Arc::new(ToolRegistry::new());

    let mut handles = vec![];

    // Spawn multiple threads to register tools
    for i in 0..5 {
        let reg = Arc::clone(&registry);
        let handle = thread::spawn(move || {
            let tool = TestTool::new(format!("thread_tool_{}", i), "Concurrent tool");
            reg.register(Box::new(tool)).unwrap();
        });
        handles.push(handle);
    }

    // Wait for all threads
    for handle in handles {
        handle.join().unwrap();
    }

    // Verify all tools were registered
    assert_eq!(registry.len(), 5);

    for i in 0..5 {
        assert!(registry.contains(&format!("thread_tool_{}", i)));
    }
}

/// Integration test: Tool execution with context
#[tokio::test]
async fn test_tool_execution_with_context() {
    struct ContextAwareTool;

    #[async_trait::async_trait]
    impl Tool for ContextAwareTool {
        fn name(&self) -> &str {
            "context_aware"
        }

        fn description(&self) -> &str {
            "Tool that uses execution context"
        }

        fn parameters(&self) -> Value {
            json!({"type": "object", "properties": {}})
        }

        async fn execute(
            &self,
            _args: HashMap<String, Value>,
            ctx: &ToolExecutionContext,
        ) -> ToolResult<String> {
            let channel = ctx.channel.as_deref().unwrap_or("unknown");
            let chat_id = ctx.chat_id.as_deref().unwrap_or("unknown");
            Ok(format!("channel={}, chat_id={}", channel, chat_id))
        }
    }

    let registry = ToolRegistry::new();
    registry.register(Box::new(ContextAwareTool)).unwrap();

    // Execute with context
    let ctx = ToolExecutionContext {
        channel: Some("telegram".to_string()),
        chat_id: Some("123456".to_string()),
    };

    let result = registry
        .execute_tool("context_aware", HashMap::new(), &ctx)
        .await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "channel=telegram, chat_id=123456");

    // Execute without context
    let empty_ctx = ToolExecutionContext::default();
    let result2 = registry
        .execute_tool("context_aware", HashMap::new(), &empty_ctx)
        .await;
    assert!(result2.is_ok());
    assert_eq!(result2.unwrap(), "channel=unknown, chat_id=unknown");
}

/// Integration test: Complex parameter handling
#[tokio::test]
async fn test_complex_parameter_handling() {
    struct ComplexTool;

    #[async_trait::async_trait]
    impl Tool for ComplexTool {
        fn name(&self) -> &str {
            "complex_tool"
        }

        fn description(&self) -> &str {
            "Tool with complex parameters"
        }

        fn parameters(&self) -> Value {
            json!({
                "type": "object",
                "properties": {
                    "string_field": { "type": "string" },
                    "int_field": { "type": "integer" },
                    "bool_field": { "type": "boolean" },
                    "array_field": {
                        "type": "array",
                        "items": { "type": "string" }
                    },
                    "object_field": {
                        "type": "object",
                        "properties": {
                            "nested": { "type": "string" }
                        }
                    }
                },
                "required": ["string_field"]
            })
        }

        async fn execute(
            &self,
            args: HashMap<String, Value>,
            _ctx: &ToolExecutionContext,
        ) -> ToolResult<String> {
            let string_field = args
                .get("string_field")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ToolError::InvalidArguments {
                    tool: "complex_tool".to_string(),
                    message: "Missing string_field".to_string(),
                })?;

            let int_field = args.get("int_field").and_then(|v| v.as_i64()).unwrap_or(0);
            let bool_field = args
                .get("bool_field")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            Ok(format!(
                "string={}, int={}, bool={}",
                string_field, int_field, bool_field
            ))
        }
    }

    let registry = ToolRegistry::new();
    registry.register(Box::new(ComplexTool)).unwrap();

    let mut args = HashMap::new();
    args.insert("string_field".to_string(), json!("test"));
    args.insert("int_field".to_string(), json!(42));
    args.insert("bool_field".to_string(), json!(true));
    args.insert("array_field".to_string(), json!(["item1", "item2"]));
    args.insert("object_field".to_string(), json!({"nested": "value"}));

    let ctx = ToolExecutionContext::default();
    let result = registry.execute_tool("complex_tool", args, &ctx).await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "string=test, int=42, bool=true");
}

/// Integration test: Empty registry behavior
#[test]
fn test_empty_registry_behavior() {
    let registry = ToolRegistry::new();

    assert!(registry.is_empty());
    assert_eq!(registry.len(), 0);
    assert!(!registry.contains("anything"));

    let tools = registry.list_tools();
    assert!(tools.is_empty());

    let definitions = registry.get_tool_definitions();
    assert!(definitions.is_empty());

    let typed_defs = registry.get_definitions();
    assert!(typed_defs.is_empty());
}

/// Integration test: Tool definition serialization
#[test]
fn test_tool_definition_serialization() {
    let def = ToolDefinition::new(
        "test_tool",
        "A test tool",
        json!({
            "type": "object",
            "properties": {
                "input": { "type": "string" }
            },
            "required": ["input"]
        }),
    );

    // Serialize to JSON
    let json_str = serde_json::to_string(&def).unwrap();
    assert!(json_str.contains("test_tool"));
    assert!(json_str.contains("A test tool"));
    assert!(json_str.contains("function"));

    // Deserialize back
    let deserialized: ToolDefinition = serde_json::from_str(&json_str).unwrap();
    assert_eq!(deserialized.name(), "test_tool");
    assert_eq!(deserialized.description(), "A test tool");
    assert_eq!(deserialized.r#type, "function");
}

/// Integration test: Error message clarity
#[tokio::test]
async fn test_error_messages_are_clear() {
    let registry = ToolRegistry::new();

    registry
        .register(Box::new(TestTool::new("test", "Test tool")))
        .unwrap();

    // Missing required parameter
    let args = HashMap::new();
    let ctx = ToolExecutionContext::default();
    let result = registry.execute_tool("test", args, &ctx).await;

    assert!(result.is_err());
    let error = result.unwrap_err();
    let error_msg = error.to_string();

    assert!(error_msg.contains("test"));
    assert!(error_msg.contains("Invalid arguments") || error_msg.contains("Missing"));
}

/// Integration test: Tool result formatting
#[tokio::test]
async fn test_tool_result_formatting() {
    let registry = ToolRegistry::new();

    registry
        .register(Box::new(TestTool::new("formatter", "Formats results")))
        .unwrap();

    let mut args = HashMap::new();
    args.insert("input".to_string(), json!("Hello World"));
    args.insert("multiplier".to_string(), json!(2));

    let ctx = ToolExecutionContext::default();
    let result = registry.execute_tool("formatter", args, &ctx).await;

    assert!(result.is_ok());
    let formatted_result = result.unwrap();

    // Verify result is a properly formatted string
    assert!(formatted_result.contains("Hello World"));
    assert!(formatted_result.contains("2"));
    assert_eq!(formatted_result, "Hello World x 2");
}

/// Integration test: Tool result format consistency
#[tokio::test]
async fn test_tool_result_format_consistency() {
    struct JsonResultTool;

    #[async_trait::async_trait]
    impl Tool for JsonResultTool {
        fn name(&self) -> &str {
            "json_tool"
        }

        fn description(&self) -> &str {
            "Returns JSON formatted results"
        }

        fn parameters(&self) -> Value {
            json!({"type": "object", "properties": {}, "required": []})
        }

        async fn execute(
            &self,
            _args: HashMap<String, Value>,
            _ctx: &ToolExecutionContext,
        ) -> ToolResult<String> {
            // Tools should return String, even if the content is JSON
            Ok(json!({"status": "success", "data": 42}).to_string())
        }
    }

    let registry = ToolRegistry::new();
    registry.register(Box::new(JsonResultTool)).unwrap();

    let result = registry
        .execute_tool(
            "json_tool",
            HashMap::new(),
            &ToolExecutionContext::default(),
        )
        .await;

    assert!(result.is_ok());
    let json_string = result.unwrap();

    // Verify it's a valid JSON string
    let parsed: serde_json::Value = serde_json::from_str(&json_string).unwrap();
    assert_eq!(parsed["status"], "success");
    assert_eq!(parsed["data"], 42);
}

/// Integration test: Verify WebTool is registered in oneshot mode
#[test]
fn test_web_tool_registration() {
    use miniclaw::agent::tools::{ToolRegistry, web::WebTool};

    let registry = ToolRegistry::new();

    // Register WebTool as done in oneshot.rs
    let web_tool = WebTool::new();
    registry.register(Box::new(web_tool)).unwrap();

    // Verify tool is registered
    assert!(registry.contains("web"));
    assert_eq!(registry.len(), 1);

    // Verify it appears in listings
    let tools = registry.list_tools();
    assert_eq!(tools.len(), 1);

    let (name, desc, _params) = &tools[0];
    assert_eq!(name, "web");
    assert!(desc.contains("Fetches web content"));
    assert!(desc.contains("HTTP/HTTPS only"));

    // Verify tool definition is correct
    let definitions = registry.get_tool_definitions();
    assert_eq!(definitions.len(), 1);
    assert_eq!(definitions[0]["function"]["name"], "web");
    assert!(definitions[0]["function"]["parameters"]["properties"]["url"].is_object());
    assert_eq!(
        definitions[0]["function"]["parameters"]["required"][0],
        "url"
    );
}

/// Integration test: Verify SpawnTool is registered and executes correctly
#[tokio::test]
async fn test_spawn_tool_integration() {
    use miniclaw::agent::tools::{ToolRegistry, spawn::SpawnTool};
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let registry = ToolRegistry::new();

    // Register SpawnTool as done in oneshot.rs
    let spawn_tool = SpawnTool::new(temp_dir.path().to_path_buf(), false).unwrap();
    registry.register(Box::new(spawn_tool)).unwrap();

    // Verify tool is registered
    assert!(registry.contains("spawn"));
    assert_eq!(registry.len(), 1);

    // Verify it appears in listings
    let tools = registry.list_tools();
    assert_eq!(tools.len(), 1);

    let (name, desc, _params) = &tools[0];
    assert_eq!(name, "spawn");
    assert!(desc.contains("background"));
    assert!(desc.contains("blacklist"));

    // Execute spawn tool with a simple command
    let mut args = HashMap::new();
    args.insert("command".to_string(), json!("echo"));
    args.insert("args".to_string(), json!(["test"]));

    let ctx = ToolExecutionContext::default();
    let result = registry.execute_tool("spawn", args, &ctx).await;

    // Should succeed and return JSON with PID
    assert!(result.is_ok());
    let output = result.unwrap();
    let parsed: Value = serde_json::from_str(&output).unwrap();
    assert_eq!(parsed["success"], true);
    assert!(parsed["pid"].as_u64().unwrap() > 0);
    assert!(
        parsed["message"]
            .as_str()
            .unwrap()
            .contains("spawned successfully")
    );
}

/// Integration test: Verify SpawnTool blacklist enforcement through registry
#[tokio::test]
async fn test_spawn_tool_blacklist_integration() {
    use miniclaw::agent::tools::{ToolRegistry, spawn::SpawnTool};
    use tempfile::TempDir;

    let temp_dir = TempDir::new().unwrap();
    let registry = ToolRegistry::new();

    let spawn_tool = SpawnTool::new(temp_dir.path().to_path_buf(), false).unwrap();
    registry.register(Box::new(spawn_tool)).unwrap();

    // Try to execute blacklisted command
    let mut args = HashMap::new();
    args.insert("command".to_string(), json!("sudo"));
    args.insert("args".to_string(), json!(["whoami"]));

    let ctx = ToolExecutionContext::default();
    let result = registry.execute_tool("spawn", args, &ctx).await;

    // Should fail with permission denied
    assert!(result.is_err());
    match result.unwrap_err() {
        ToolError::PermissionDenied { message, .. } => {
            assert!(message.contains("Command not allowed"));
            assert!(message.contains("sudo"));
        }
        _ => panic!("Expected PermissionDenied error"),
    }
}

/// Integration test: Memory tool basic functionality
#[tokio::test]
async fn test_memory_tool_basic_functionality() {
    use miniclaw::agent::tools::memory::MemoryTool;
    use tempfile::tempdir;

    let temp_dir = tempdir().unwrap();
    let workspace_path = temp_dir.path().to_path_buf();

    let memory_tool = MemoryTool::new(workspace_path).unwrap();

    // Test tool properties
    assert_eq!(memory_tool.name(), "write_memory");
    assert!(memory_tool.description().contains("memory"));

    // Test parameters schema
    let params = memory_tool.parameters();
    assert_eq!(params.get("type").unwrap(), "object");

    let properties = params.get("properties").unwrap();
    assert!(properties.get("content").is_some());
    assert!(properties.get("type").is_some());
}

/// Integration test: Memory tool long-term memory writing
#[tokio::test]
async fn test_memory_tool_long_term_memory() {
    use miniclaw::agent::tools::memory::MemoryTool;
    use tempfile::tempdir;

    let temp_dir = tempdir().unwrap();
    let workspace_path = temp_dir.path().to_path_buf();

    let memory_tool = MemoryTool::new(workspace_path).unwrap();

    let mut args = HashMap::new();
    args.insert("content".to_string(), json!("Test memory content"));
    args.insert("type".to_string(), json!("long_term"));

    let ctx = ToolExecutionContext::default();
    let result = memory_tool.execute(args, &ctx).await;

    assert!(result.is_ok());
    let response = result.unwrap();
    let response_value: Value = serde_json::from_str(&response).unwrap();

    assert_eq!(response_value.get("success").unwrap(), true);
    assert!(
        response_value
            .get("message")
            .unwrap()
            .as_str()
            .unwrap()
            .contains("Memory updated")
    );
    assert!(response_value.get("file_path").is_some());
}

/// Integration test: Memory tool daily notes
#[tokio::test]
async fn test_memory_tool_daily_notes() {
    use miniclaw::agent::tools::memory::MemoryTool;
    use tempfile::tempdir;

    let temp_dir = tempdir().unwrap();
    let workspace_path = temp_dir.path().to_path_buf();

    let memory_tool = MemoryTool::new(workspace_path).unwrap();

    let mut args = HashMap::new();
    args.insert("content".to_string(), json!("Test daily note"));
    args.insert("type".to_string(), json!("daily"));

    let ctx = ToolExecutionContext::default();
    let result = memory_tool.execute(args, &ctx).await;

    assert!(result.is_ok());
    let response = result.unwrap();
    let response_value: Value = serde_json::from_str(&response).unwrap();

    assert_eq!(response_value.get("success").unwrap(), true);
    assert!(
        response_value
            .get("message")
            .unwrap()
            .as_str()
            .unwrap()
            .contains("Daily note created")
    );
    assert!(response_value.get("file_path").is_some());
}

/// Integration test: Memory tool error handling
#[tokio::test]
async fn test_memory_tool_error_handling() {
    use miniclaw::agent::tools::memory::MemoryTool;
    use tempfile::tempdir;

    let temp_dir = tempdir().unwrap();
    let workspace_path = temp_dir.path().to_path_buf();

    let memory_tool = MemoryTool::new(workspace_path).unwrap();

    // Test missing content parameter
    let args = HashMap::new();
    let ctx = ToolExecutionContext::default();
    let result = memory_tool.execute(args, &ctx).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        ToolError::InvalidArguments { tool, message } => {
            assert_eq!(tool, "write_memory");
            assert!(message.contains("content"));
        }
        _ => panic!("Expected InvalidArguments error"),
    }

    // Test invalid memory type
    let mut args = HashMap::new();
    args.insert("content".to_string(), json!("Test content"));
    args.insert("type".to_string(), json!("invalid_type"));

    let result = memory_tool.execute(args, &ctx).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        ToolError::InvalidArguments { tool, message } => {
            assert_eq!(tool, "write_memory");
            assert!(message.contains("Invalid memory type"));
        }
        _ => panic!("Expected InvalidArguments error"),
    }
}

/// Integration test: Memory tool empty content validation
#[tokio::test]
async fn test_memory_tool_empty_content() {
    use miniclaw::agent::tools::memory::MemoryTool;
    use tempfile::tempdir;

    let temp_dir = tempdir().unwrap();
    let workspace_path = temp_dir.path().to_path_buf();

    let memory_tool = MemoryTool::new(workspace_path).unwrap();

    let mut args = HashMap::new();
    args.insert("content".to_string(), json!(""));

    let ctx = ToolExecutionContext::default();
    let result = memory_tool.execute(args, &ctx).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        ToolError::InvalidArguments { tool, message } => {
            assert_eq!(tool, "write_memory");
            assert!(message.contains("empty"));
        }
        _ => panic!("Expected InvalidArguments error"),
    }
}
