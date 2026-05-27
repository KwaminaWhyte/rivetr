//! MCP tool definitions in JSON Schema format.

use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct McpTool {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

pub fn get_tools() -> Vec<McpTool> {
    vec![
        McpTool {
            name: "list_apps".to_string(),
            description: "List all deployed applications".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "project_id": {"type": "string", "description": "Filter by project"}
                }
            }),
        },
        McpTool {
            name: "deploy_app".to_string(),
            description: "Trigger a deployment for an application".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "required": ["app_id"],
                "properties": {
                    "app_id": {"type": "string", "description": "Application ID to deploy"}
                }
            }),
        },
        McpTool {
            name: "get_app_status".to_string(),
            description: "Get the current status of an application".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "required": ["app_id"],
                "properties": {
                    "app_id": {"type": "string", "description": "Application ID"}
                }
            }),
        },
        McpTool {
            name: "get_deployment_logs".to_string(),
            description: "Get logs for a specific deployment".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "required": ["deployment_id"],
                "properties": {
                    "deployment_id": {"type": "string"}
                }
            }),
        },
        McpTool {
            name: "list_deployments".to_string(),
            description: "List recent deployments, optionally filtered by application".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "app_id": {"type": "string", "description": "Filter by application ID"},
                    "limit": {"type": "integer", "description": "Max rows (default 20, max 100)"}
                }
            }),
        },
        McpTool {
            name: "get_deployment_status".to_string(),
            description: "Get the status of a specific deployment".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "required": ["deployment_id"],
                "properties": {
                    "deployment_id": {"type": "string"}
                }
            }),
        },
        McpTool {
            name: "list_services".to_string(),
            description: "List all deployed services (Docker Compose / template services)"
                .to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "project_id": {"type": "string", "description": "Filter by project"}
                }
            }),
        },
        McpTool {
            name: "list_databases".to_string(),
            description: "List all managed databases".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "project_id": {"type": "string", "description": "Filter by project"}
                }
            }),
        },
        McpTool {
            name: "list_projects".to_string(),
            description: "List all projects".to_string(),
            input_schema: serde_json::json!({"type": "object", "properties": {}}),
        },
        McpTool {
            name: "restart_app".to_string(),
            description: "Trigger a redeploy (restart) of an application".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "required": ["app_id"],
                "properties": {
                    "app_id": {"type": "string", "description": "Application ID to restart"}
                }
            }),
        },
    ]
}
