//! Database seeders for built-in service templates.

mod ai_ml;
mod analytics_automation;
mod cms_communication;
mod devtools;
mod documentation;
mod infrastructure;
mod media_monitoring;
mod project_mgmt;
mod security_search;

use anyhow::Result;
use sqlx::SqlitePool;
use tracing::info;

type TemplateEntry = (
    &'static str, // id
    &'static str, // name
    &'static str, // description
    &'static str, // category
    &'static str, // icon
    &'static str, // compose_template
    &'static str, // env_schema
);

pub async fn seed_service_templates(pool: &SqlitePool) -> Result<()> {
    info!("Seeding built-in service templates...");

    let mut templates: Vec<TemplateEntry> = Vec::new();
    templates.extend(infrastructure::templates());
    templates.extend(ai_ml::templates());
    templates.extend(analytics_automation::templates());
    templates.extend(cms_communication::templates());
    templates.extend(devtools::templates());
    templates.extend(documentation::templates());
    templates.extend(media_monitoring::templates());
    templates.extend(security_search::templates());
    templates.extend(project_mgmt::templates());

    let template_count = templates.len();
    for (id, name, description, category, icon, compose, env_schema) in templates {
        sqlx::query(
            r#"
            INSERT OR REPLACE INTO service_templates
            (id, name, description, category, icon, compose_template, env_schema, is_builtin, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, 1, COALESCE((SELECT created_at FROM service_templates WHERE id = ?), datetime('now')))
            "#,
        )
        .bind(id)
        .bind(name)
        .bind(description)
        .bind(category)
        .bind(icon)
        .bind(compose)
        .bind(env_schema)
        .bind(id)
        .execute(pool)
        .await?;
    }

    info!("Seeded {} built-in service templates", template_count);
    Ok(())
}
