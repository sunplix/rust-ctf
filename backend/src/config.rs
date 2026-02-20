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
    pub instance_default_cpu_limit: f64,
    pub instance_default_memory_limit_mb: i64,
    pub instance_public_host: String,
    pub instance_host_port_min: u16,
    pub instance_host_port_max: u16,
    pub default_admin_enabled: bool,
    pub default_admin_username: String,
    pub default_admin_email: String,
    pub default_admin_password: String,
    pub default_admin_force_password_reset: bool,
    pub auth_human_verification_enabled: bool,
    pub auth_turnstile_secret_key: String,
    pub auth_turnstile_siteverify_url: String,
    pub auth_turnstile_expected_hostname: String,
    pub auth_human_verification_timeout_seconds: u64,
    pub auth_password_min_length: u64,
    pub auth_password_min_strength_score: u8,
    pub auth_password_require_lowercase: bool,
    pub auth_password_require_uppercase: bool,
    pub auth_password_require_digit: bool,
    pub auth_password_require_symbol: bool,
    pub auth_password_min_unique_chars: u64,
    pub auth_password_block_weak_patterns: bool,
    pub auth_email_verification_enabled: bool,
    pub auth_email_verification_required: bool,
    pub auth_email_verification_token_ttl_minutes: i64,
    pub auth_password_reset_enabled: bool,
    pub auth_password_reset_token_ttl_minutes: i64,
    pub auth_email_base_url: String,
    pub auth_email_delivery_mode: String,
    pub auth_email_from_name: String,
    pub auth_email_from_address: String,
    pub auth_smtp_host: String,
    pub auth_smtp_port: u16,
    pub auth_smtp_username: String,
    pub auth_smtp_password: String,
    pub auth_smtp_use_tls: bool,
    pub runtime_alert_scan_enabled: bool,
    pub runtime_alert_scan_interval_seconds: u64,
    pub runtime_alert_scan_initial_delay_seconds: u64,
    pub instance_reaper_enabled: bool,
    pub instance_reaper_interval_seconds: u64,
    pub instance_reaper_initial_delay_seconds: u64,
    pub instance_reaper_batch_size: i64,
    pub instance_heartbeat_stale_seconds: u64,
    pub instance_heartbeat_report_url: String,
    pub instance_heartbeat_report_interval_seconds: u64,
    pub instance_stale_reaper_enabled: bool,
    pub instance_stale_reaper_batch_size: i64,
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
            .set_default("instance_default_cpu_limit", 1.0_f64)?
            .set_default("instance_default_memory_limit_mb", 512_i64)?
            .set_default("instance_public_host", "127.0.0.1")?
            .set_default("instance_host_port_min", 32768_u16)?
            .set_default("instance_host_port_max", 60999_u16)?
            .set_default("default_admin_enabled", true)?
            .set_default("default_admin_username", "admin")?
            .set_default("default_admin_email", "admin@rust-ctf.local")?
            .set_default("default_admin_password", "admin123456")?
            .set_default("default_admin_force_password_reset", false)?
            .set_default("auth_human_verification_enabled", false)?
            .set_default("auth_turnstile_secret_key", "")?
            .set_default(
                "auth_turnstile_siteverify_url",
                "https://challenges.cloudflare.com/turnstile/v0/siteverify",
            )?
            .set_default("auth_turnstile_expected_hostname", "")?
            .set_default("auth_human_verification_timeout_seconds", 8_u64)?
            .set_default("auth_password_min_length", 10_u64)?
            .set_default("auth_password_min_strength_score", 3_u8)?
            .set_default("auth_password_require_lowercase", true)?
            .set_default("auth_password_require_uppercase", true)?
            .set_default("auth_password_require_digit", true)?
            .set_default("auth_password_require_symbol", false)?
            .set_default("auth_password_min_unique_chars", 6_u64)?
            .set_default("auth_password_block_weak_patterns", true)?
            .set_default("auth_email_verification_enabled", true)?
            .set_default("auth_email_verification_required", false)?
            .set_default("auth_email_verification_token_ttl_minutes", 30_i64)?
            .set_default("auth_password_reset_enabled", true)?
            .set_default("auth_password_reset_token_ttl_minutes", 30_i64)?
            .set_default("auth_email_base_url", "http://127.0.0.1:5173")?
            .set_default("auth_email_delivery_mode", "log")?
            .set_default("auth_email_from_name", "Rust-CTF")?
            .set_default("auth_email_from_address", "no-reply@rust-ctf.local")?
            .set_default("auth_smtp_host", "")?
            .set_default("auth_smtp_port", 587_u16)?
            .set_default("auth_smtp_username", "")?
            .set_default("auth_smtp_password", "")?
            .set_default("auth_smtp_use_tls", true)?
            .set_default("runtime_alert_scan_enabled", true)?
            .set_default("runtime_alert_scan_interval_seconds", 60_u64)?
            .set_default("runtime_alert_scan_initial_delay_seconds", 10_u64)?
            .set_default("instance_reaper_enabled", true)?
            .set_default("instance_reaper_interval_seconds", 60_u64)?
            .set_default("instance_reaper_initial_delay_seconds", 20_u64)?
            .set_default("instance_reaper_batch_size", 30_i64)?
            .set_default("instance_heartbeat_stale_seconds", 300_u64)?
            .set_default(
                "instance_heartbeat_report_url",
                "http://host.docker.internal:8080/api/v1/instances/heartbeat/report",
            )?
            .set_default("instance_heartbeat_report_interval_seconds", 30_u64)?
            .set_default("instance_stale_reaper_enabled", false)?
            .set_default("instance_stale_reaper_batch_size", 20_i64)?
            .add_source(::config::Environment::default().separator("__"));

        builder.build()?.try_deserialize().map_err(Into::into)
    }
}
