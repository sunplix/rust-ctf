use crate::{
    config::AppConfig,
    password_policy::{enforce_password_policy, PasswordContext},
};
use anyhow::Context;
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Argon2,
};
use redis::aio::ConnectionManager;
use sqlx::{postgres::PgPoolOptions, FromRow, PgPool};
use tracing::{info, warn};

pub struct AppState {
    pub config: AppConfig,
    pub db: PgPool,
    pub redis_client: redis::Client,
    pub redis: ConnectionManager,
}

impl AppState {
    pub async fn new(config: AppConfig) -> anyhow::Result<Self> {
        if config.jwt_secret.len() < 32 {
            warn!("JWT_SECRET is shorter than 32 characters; use a stronger secret");
        }

        let db = PgPoolOptions::new()
            .max_connections(20)
            .connect(&config.database_url)
            .await
            .context("failed to connect postgres")?;

        sqlx::migrate!("./migrations")
            .run(&db)
            .await
            .context("failed to run database migrations")?;

        ensure_default_admin_user(&db, &config).await?;

        let redis_client =
            redis::Client::open(config.redis_url.clone()).context("invalid redis url")?;
        let redis = ConnectionManager::new(redis_client.clone())
            .await
            .context("failed to connect redis")?;

        Ok(Self {
            config,
            db,
            redis_client,
            redis,
        })
    }
}

#[derive(Debug, FromRow)]
struct ExistingAdminRow {
    id: uuid::Uuid,
    username: String,
    email: String,
}

async fn ensure_default_admin_user(db: &PgPool, config: &AppConfig) -> anyhow::Result<()> {
    if !config.default_admin_enabled {
        return Ok(());
    }

    let username = config.default_admin_username.trim();
    let email = config.default_admin_email.trim().to_lowercase();
    let password = config.default_admin_password.as_str();

    if username.is_empty() || email.is_empty() {
        warn!(
            "default admin bootstrap skipped: DEFAULT_ADMIN_USERNAME/DEFAULT_ADMIN_EMAIL is empty"
        );
        return Ok(());
    }

    if let Err(reason) = enforce_password_policy(
        config,
        password,
        PasswordContext {
            username: Some(username),
            email: Some(&email),
        },
    ) {
        warn!(
            reason = %reason,
            "default admin bootstrap skipped: DEFAULT_ADMIN_PASSWORD does not pass policy"
        );
        return Ok(());
    }

    if password == "admin123456" {
        warn!("DEFAULT_ADMIN_PASSWORD is using the default value; change it before production");
    }

    let existing = sqlx::query_as::<_, ExistingAdminRow>(
        "SELECT id, username, email
         FROM users
         WHERE LOWER(username) = LOWER($1) OR LOWER(email) = LOWER($2)
         LIMIT 1",
    )
    .bind(username)
    .bind(&email)
    .fetch_optional(db)
    .await
    .context("failed to query default admin user")?;

    let password_hash = hash_password(password).context("failed to hash default admin password")?;

    match existing {
        Some(existing) => {
            if config.default_admin_force_password_reset {
                sqlx::query(
                    "UPDATE users
                     SET role = 'admin',
                         status = 'active',
                         email_verified = TRUE,
                         email_verified_at = COALESCE(email_verified_at, NOW()),
                         password_hash = $2,
                         updated_at = NOW()
                     WHERE id = $1",
                )
                .bind(existing.id)
                .bind(password_hash)
                .execute(db)
                .await
                .context("failed to update default admin user")?;

                info!(
                    admin_user_id = %existing.id,
                    username = %existing.username,
                    email = %existing.email,
                    "default admin user ensured (password reset by configuration)"
                );
            } else {
                sqlx::query(
                    "UPDATE users
                     SET role = 'admin',
                         status = 'active',
                         email_verified = TRUE,
                         email_verified_at = COALESCE(email_verified_at, NOW()),
                         updated_at = NOW()
                     WHERE id = $1",
                )
                .bind(existing.id)
                .execute(db)
                .await
                .context("failed to promote existing default admin user")?;

                info!(
                    admin_user_id = %existing.id,
                    username = %existing.username,
                    email = %existing.email,
                    "default admin user ensured (kept existing password)"
                );
            }
        }
        None => {
            let created_id = sqlx::query_scalar::<_, uuid::Uuid>(
                "INSERT INTO users (username, email, password_hash, role, status, email_verified, email_verified_at)
                 VALUES ($1, $2, $3, 'admin', 'active', TRUE, NOW())
                 RETURNING id",
            )
            .bind(username)
            .bind(&email)
            .bind(password_hash)
            .fetch_one(db)
            .await
            .context("failed to create default admin user")?;

            info!(
                admin_user_id = %created_id,
                username = %username,
                email = %email,
                "default admin user created"
            );
        }
    }

    Ok(())
}

fn hash_password(password: &str) -> anyhow::Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|err| anyhow::anyhow!("argon2 password hashing failed: {err}"))?
        .to_string();
    Ok(hash)
}
