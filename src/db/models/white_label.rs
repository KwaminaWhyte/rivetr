//! White label configuration model.

use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

/// White label configuration stored in the database.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct WhiteLabel {
    pub id: i64,
    pub app_name: String,
    pub app_description: Option<String>,
    pub logo_url: Option<String>,
    pub favicon_url: Option<String>,
    pub custom_css: Option<String>,
    pub footer_text: Option<String>,
    pub support_url: Option<String>,
    pub docs_url: Option<String>,
    pub login_page_message: Option<String>,
    pub updated_at: String,
}

/// Request body for updating white label configuration (all fields optional).
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateWhiteLabelRequest {
    pub app_name: Option<String>,
    pub app_description: Option<String>,
    pub logo_url: Option<String>,
    pub favicon_url: Option<String>,
    pub custom_css: Option<String>,
    pub footer_text: Option<String>,
    pub support_url: Option<String>,
    pub docs_url: Option<String>,
    pub login_page_message: Option<String>,
}

impl WhiteLabel {
    /// Load the white label configuration from the database.
    pub async fn load(db: &SqlitePool) -> Result<Self, sqlx::Error> {
        let row = sqlx::query_as::<_, Self>("SELECT * FROM white_label WHERE id = 1")
            .fetch_optional(db)
            .await?;

        // Return defaults if the row doesn't exist yet
        Ok(row.unwrap_or_else(|| Self {
            id: 1,
            app_name: "Rivetr".to_string(),
            app_description: None,
            logo_url: None,
            favicon_url: None,
            custom_css: None,
            footer_text: None,
            support_url: None,
            docs_url: None,
            login_page_message: None,
            updated_at: chrono::Utc::now().to_rfc3339(),
        }))
    }

    /// Update the white label configuration.
    pub async fn update(db: &SqlitePool, req: &UpdateWhiteLabelRequest) -> Result<Self, sqlx::Error> {
        let now = chrono::Utc::now().to_rfc3339();

        sqlx::query(
            r#"
            INSERT INTO white_label (id, app_name, app_description, logo_url, favicon_url,
                                     custom_css, footer_text, support_url, docs_url,
                                     login_page_message, updated_at)
            VALUES (1,
                COALESCE(?, 'Rivetr'),
                ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                app_name = COALESCE(excluded.app_name, app_name),
                app_description = COALESCE(excluded.app_description, app_description),
                logo_url = excluded.logo_url,
                favicon_url = excluded.favicon_url,
                custom_css = excluded.custom_css,
                footer_text = excluded.footer_text,
                support_url = excluded.support_url,
                docs_url = excluded.docs_url,
                login_page_message = excluded.login_page_message,
                updated_at = excluded.updated_at
            "#,
        )
        .bind(req.app_name.as_deref())
        .bind(req.app_description.as_deref())
        .bind(req.logo_url.as_deref())
        .bind(req.favicon_url.as_deref())
        .bind(req.custom_css.as_deref())
        .bind(req.footer_text.as_deref())
        .bind(req.support_url.as_deref())
        .bind(req.docs_url.as_deref())
        .bind(req.login_page_message.as_deref())
        .bind(&now)
        .execute(db)
        .await?;

        Self::load(db).await
    }
}
