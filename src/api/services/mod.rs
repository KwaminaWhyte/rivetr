//! API handlers for Docker Compose services.

pub(crate) mod compose;
mod control;
mod crud;
mod deploy_config;
mod deployments;
mod export_db;
mod import_db;

// Re-export everything callers need
pub use control::{
    get_service_generated_vars, get_service_logs, get_service_stats, preview_compose,
    restart_service, start_service, stop_service, stream_service_logs,
};
pub use crud::{
    check_port, create_service, delete_service, get_service, list_services, update_service,
};
pub use deploy_config::{get_deploy_config, update_deploy_config};
pub use deployments::{
    deploy_service, get_service_deployment, get_service_deployment_logs, list_service_deployments,
    rollback_service_deployment, service_deployment_logs_ws,
};
pub use export_db::export_service_db;
pub use import_db::import_service_db;
