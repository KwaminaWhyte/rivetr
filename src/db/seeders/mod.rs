//! Database seeders for built-in service templates.

mod ai_extras;
mod ai_ml;
mod analytics_automation;
mod auth_identity;
mod business;
mod cms_communication;
mod cms_extra;
mod communication_extra;
mod databases_tools;
mod devtools;
mod devtools_extra;
mod documentation;
mod extra_services;
mod infrastructure;
mod media_monitoring;
mod media_productivity;
mod misc_extras;
mod monitoring_extra;
mod networking_extra;
mod project_mgmt;
mod security_search;
mod sprint15;
mod sprint16;
mod sprint18;
mod sprint19;
mod sprint20;
mod sprint21;
mod sprint22;
mod sprint23;
mod sprint24;
mod sprint25;

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
    templates.extend(media_productivity::templates());
    templates.extend(security_search::templates());
    templates.extend(project_mgmt::templates());
    templates.extend(extra_services::templates());
    // Sprint 14 additions
    templates.extend(ai_extras::templates());
    templates.extend(auth_identity::templates());
    templates.extend(business::templates());
    templates.extend(cms_extra::templates());
    templates.extend(communication_extra::templates());
    templates.extend(databases_tools::templates());
    templates.extend(devtools_extra::templates());
    templates.extend(misc_extras::templates());
    templates.extend(monitoring_extra::templates());
    templates.extend(networking_extra::templates());
    // Sprint 15 additions
    templates.extend(sprint15::templates());
    // Sprint 16 additions
    templates.extend(sprint16::templates());
    // Sprint 18 additions
    templates.extend(sprint18::templates());
    // Sprint 19 additions
    templates.extend(sprint19::templates());
    // Sprint 20 additions
    templates.extend(sprint20::templates());
    // Sprint 21 additions
    templates.extend(sprint21::templates());
    // Sprint 22 additions
    templates.extend(sprint22::templates());
    // Sprint 23 additions
    templates.extend(sprint23::templates());
    // Sprint 24 additions
    templates.extend(sprint24::templates());
    // Sprint 25 additions
    templates.extend(sprint25::templates());

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
