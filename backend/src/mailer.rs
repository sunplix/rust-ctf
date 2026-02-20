use anyhow::{anyhow, Context};
use lettre::{
    message::Mailbox,
    transport::smtp::authentication::Credentials,
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
};
use tracing::{info, warn};

use crate::config::AppConfig;

#[derive(Debug, Clone)]
pub struct OutboundEmail {
    pub to: String,
    pub subject: String,
    pub text_body: String,
}

pub async fn send_outbound_email(config: &AppConfig, payload: OutboundEmail) -> anyhow::Result<()> {
    let mode = config.auth_email_delivery_mode.trim().to_ascii_lowercase();
    match mode.as_str() {
        "smtp" => send_via_smtp(config, &payload).await,
        "" | "log" => {
            log_email(&payload);
            Ok(())
        }
        unsupported => {
            warn!(
                mode = unsupported,
                "unknown AUTH_EMAIL_DELIVERY_MODE, fallback to log transport"
            );
            log_email(&payload);
            Ok(())
        }
    }
}

fn log_email(payload: &OutboundEmail) {
    info!(
        to = %payload.to,
        subject = %payload.subject,
        body = %payload.text_body,
        "outbound auth email (log transport)"
    );
}

async fn send_via_smtp(config: &AppConfig, payload: &OutboundEmail) -> anyhow::Result<()> {
    let host = config.auth_smtp_host.trim();
    if host.is_empty() {
        return Err(anyhow!("AUTH_SMTP_HOST is empty while SMTP mode is enabled"));
    }

    let from = parse_from_mailbox(config)?;
    let to: Mailbox = payload
        .to
        .parse()
        .with_context(|| format!("invalid recipient mailbox: {}", payload.to))?;

    let message = Message::builder()
        .from(from)
        .to(to)
        .subject(payload.subject.clone())
        .body(payload.text_body.clone())
        .context("failed to build smtp message")?;

    let mut builder = if config.auth_smtp_use_tls {
        AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(host)
            .with_context(|| format!("invalid AUTH_SMTP_HOST: {host}"))?
    } else {
        AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(host)
    }
    .port(config.auth_smtp_port);

    let smtp_username = config.auth_smtp_username.trim();
    if !smtp_username.is_empty() {
        builder = builder.credentials(Credentials::new(
            smtp_username.to_string(),
            config.auth_smtp_password.clone(),
        ));
    }

    let client = builder.build();
    client
        .send(message)
        .await
        .context("failed to send smtp email")?;

    Ok(())
}

fn parse_from_mailbox(config: &AppConfig) -> anyhow::Result<Mailbox> {
    let from_address = config.auth_email_from_address.trim();
    if from_address.is_empty() {
        return Err(anyhow!(
            "AUTH_EMAIL_FROM_ADDRESS is empty while SMTP mode is enabled"
        ));
    }

    let from_name = config.auth_email_from_name.trim();
    if from_name.is_empty() {
        return from_address
            .parse()
            .with_context(|| format!("invalid AUTH_EMAIL_FROM_ADDRESS: {from_address}"));
    }

    format!("{from_name} <{from_address}>")
        .parse()
        .with_context(|| format!("invalid AUTH_EMAIL_FROM_NAME/AUTH_EMAIL_FROM_ADDRESS pair"))
}
