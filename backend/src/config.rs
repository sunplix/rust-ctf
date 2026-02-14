use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub app_host: String,
    pub app_port: u16,
    pub database_url: String,
    pub redis_url: String,
    pub jwt_secret: String,
}

impl AppConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        let builder = ::config::Config::builder()
            .set_default("app_host", "0.0.0.0")?
            .set_default("app_port", 8080)?
            .set_default("database_url", "postgres://ctf:ctf@localhost:5432/rust_ctf")?
            .set_default("redis_url", "redis://localhost:6379")?
            .set_default("jwt_secret", "change_me_in_production")?
            .add_source(::config::Environment::default().separator("__"));

        builder.build()?.try_deserialize().map_err(Into::into)
    }
}
