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
    ]
}
