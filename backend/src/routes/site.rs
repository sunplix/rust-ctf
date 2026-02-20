use std::sync::Arc;

use axum::{extract::State, routing::get, Json, Router};
use serde::Serialize;
use sqlx::FromRow;

use crate::{
    error::{AppError, AppResult},
    state::AppState,
};

#[derive(Debug, Serialize, FromRow)]
struct PublicSiteSettings {
    site_name: String,
    site_subtitle: String,
    home_title: String,
    home_tagline: String,
    home_signature: String,
    footer_text: String,
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/site/settings", get(get_site_settings))
}

async fn get_site_settings(
    State(state): State<Arc<AppState>>,
) -> AppResult<Json<PublicSiteSettings>> {
    let row = sqlx::query_as::<_, PublicSiteSettings>(
        "SELECT site_name,
                site_subtitle,
                home_title,
                home_tagline,
                home_signature,
                footer_text
         FROM site_settings
         WHERE id = TRUE
         LIMIT 1",
    )
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::internal)?
    .ok_or_else(|| AppError::Internal(anyhow::anyhow!("site settings not initialized")))?;

    Ok(Json(row))
}
