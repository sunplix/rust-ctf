use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub app_host: String,
    pub app_port: u16,
    pub database_url: String,
    pub redis_url: String,
    pub jwt_secret: String,
    pub instance_runtime_root: String,
    pub compose_command_timeout_seconds: u64,
    pub default_admin_enabled: bool,
    pub default_admin_username: String,
    pub default_admin_email: String,
    pub default_admin_password: String,
    pub default_admin_force_password_reset: bool,
    pub runtime_alert_scan_enabled: bool,
    pub runtime_alert_scan_interval_seconds: u64,
    pub runtime_alert_scan_initial_delay_seconds: u64,
}

impl AppConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        let builder = ::config::Config::builder()
            .set_default("app_host", "0.0.0.0")?
            .set_default("app_port", 8080)?
            .set_default("database_url", "postgres://ctf:ctf@localhost:5432/rust_ctf")?
            .set_default("redis_url", "redis://localhost:6379")?
            .set_default("jwt_secret", "change_me_in_production")?
            .set_default("instance_runtime_root", "./runtime/instances")?
            .set_default("compose_command_timeout_seconds", 120_u64)?
            .set_default("default_admin_enabled", true)?
            .set_default("default_admin_username", "admin")?
            .set_default("default_admin_email", "admin@rust-ctf.local")?
            .set_default("default_admin_password", "admin123456")?
            .set_default("default_admin_force_password_reset", false)?
            .set_default("runtime_alert_scan_enabled", true)?
            .set_default("runtime_alert_scan_interval_seconds", 60_u64)?
            .set_default("runtime_alert_scan_initial_delay_seconds", 10_u64)?
            .add_source(::config::Environment::default().separator("__"));

        builder.build()?.try_deserialize().map_err(Into::into)
    }
}
