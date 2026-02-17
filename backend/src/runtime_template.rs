use std::collections::{HashMap, HashSet};

use serde_json::Value;

const COMPOSE_VARIABLES_KEY: &str = "compose_variables";
const RUNTIME_KEY: &str = "runtime";

const RESERVED_PLACEHOLDERS: &[&str] = &[
    "PROJECT_NAME",
    "COMPOSE_PROJECT_NAME",
    "NETWORK_NAME",
    "SUBNET",
    "SUBNET_CIDR",
    "TEAM_ID",
    "CONTEST_ID",
    "CHALLENGE_ID",
    "ENTRYPOINT_URL",
    "ENTRYPOINT_HOST",
    "GATEWAY_IP",
    "PUBLIC_HOST",
    "HOST_PORT",
    "ACCESS_HOST_PORT",
    "ACCESS_USERNAME",
    "ACCESS_PASSWORD",
    "CPU_LIMIT",
    "MEMORY_LIMIT_MB",
    "MEMORY_LIMIT",
    "HEARTBEAT_REPORT_URL",
    "HEARTBEAT_REPORT_TOKEN",
    "HEARTBEAT_INTERVAL_SECONDS",
    "DYNAMIC_FLAG",
    "FLAG",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeMode {
    Compose,
    SingleImage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeEndpointProtocol {
    Http,
    Https,
    Tcp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeAccessMode {
    Direct,
    SshBastion,
}

#[derive(Debug, Clone)]
pub struct RuntimeMetadataOptions {
    pub mode: RuntimeMode,
    pub access_mode: RuntimeAccessMode,
    pub single_image: Option<SingleImageRuntimeConfig>,
}

#[derive(Debug, Clone)]
pub struct SingleImageRuntimeConfig {
    pub image: String,
    pub internal_port: u16,
    pub protocol: RuntimeEndpointProtocol,
}

#[derive(Debug, Clone)]
struct ComposeVariableDef {
    value: String,
    required: bool,
}

pub fn parse_runtime_metadata_options(metadata: &Value) -> Result<RuntimeMetadataOptions, String> {
    let runtime = metadata
        .get(RUNTIME_KEY)
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();

    let mode = match runtime
        .get("mode")
        .and_then(Value::as_str)
        .unwrap_or("compose")
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "compose" | "compose_template" => RuntimeMode::Compose,
        "single_image" | "single-image" | "image" => RuntimeMode::SingleImage,
        other => {
            return Err(format!(
                "metadata.runtime.mode is invalid: '{other}', allowed: compose,single_image"
            ));
        }
    };

    let access_mode = match runtime
        .get("access_mode")
        .and_then(Value::as_str)
        .map(|value| value.trim().to_ascii_lowercase())
    {
        Some(value) if value == "direct" => RuntimeAccessMode::Direct,
        Some(value) if value == "ssh_bastion" || value == "ssh-bastion" || value == "bastion" => {
            RuntimeAccessMode::SshBastion
        }
        Some(value) => {
            return Err(format!(
                "metadata.runtime.access_mode is invalid: '{value}', allowed: direct,ssh_bastion"
            ));
        }
        None => {
            if mode == RuntimeMode::Compose {
                RuntimeAccessMode::SshBastion
            } else {
                RuntimeAccessMode::Direct
            }
        }
    };

    let single_image = if mode == RuntimeMode::SingleImage {
        let image = runtime
            .get("image")
            .or_else(|| runtime.get("repository"))
            .or_else(|| runtime.get("repository_link"))
            .and_then(Value::as_str)
            .map(str::trim)
            .ok_or_else(|| "metadata.runtime.image is required when mode=single_image".to_string())?
            .to_string();
        if image.is_empty() {
            return Err("metadata.runtime.image must not be empty".to_string());
        }
        if image.chars().any(char::is_whitespace) {
            return Err("metadata.runtime.image must not contain spaces".to_string());
        }
        if image.contains('"') || image.contains('\'') {
            return Err("metadata.runtime.image contains unsupported quote characters".to_string());
        }

        let internal_port = runtime
            .get("internal_port")
            .or_else(|| runtime.get("container_port"))
            .and_then(Value::as_u64)
            .ok_or_else(|| {
                "metadata.runtime.internal_port is required when mode=single_image".to_string()
            })?;
        if internal_port == 0 || internal_port > u16::MAX as u64 {
            return Err("metadata.runtime.internal_port must be in 1..65535".to_string());
        }

        let protocol = match runtime
            .get("protocol")
            .or_else(|| runtime.get("entrypoint_protocol"))
            .or_else(|| runtime.get("scheme"))
            .and_then(Value::as_str)
            .unwrap_or("http")
            .trim()
            .to_ascii_lowercase()
            .as_str()
        {
            "http" => RuntimeEndpointProtocol::Http,
            "https" => RuntimeEndpointProtocol::Https,
            "tcp" => RuntimeEndpointProtocol::Tcp,
            other => {
                return Err(format!(
                    "metadata.runtime.protocol is invalid: '{other}', allowed: http,https,tcp"
                ));
            }
        };

        Some(SingleImageRuntimeConfig {
            image,
            internal_port: internal_port as u16,
            protocol,
        })
    } else {
        None
    };

    Ok(RuntimeMetadataOptions {
        mode,
        access_mode,
        single_image,
    })
}

pub fn build_single_image_compose_template(image: &str, internal_port: u16) -> String {
    format!(
        "services:\n  target:\n    image: \"{image}\"\n    restart: unless-stopped\n    ports:\n      - \"{{{{HOST_PORT}}}}:{internal_port}\"\n    environment:\n      DYNAMIC_FLAG: \"{{{{DYNAMIC_FLAG}}}}\"\n      FLAG: \"{{{{FLAG}}}}\"\n      TEAM_ID: \"{{{{TEAM_ID}}}}\"\n      CONTEST_ID: \"{{{{CONTEST_ID}}}}\"\n      CHALLENGE_ID: \"{{{{CHALLENGE_ID}}}}\"\n      HEARTBEAT_REPORT_URL: \"{{{{HEARTBEAT_REPORT_URL}}}}\"\n      HEARTBEAT_REPORT_TOKEN: \"{{{{HEARTBEAT_REPORT_TOKEN}}}}\"\n      HEARTBEAT_INTERVAL_SECONDS: \"{{{{HEARTBEAT_INTERVAL_SECONDS}}}}\"\n    networks:\n      - \"{{{{NETWORK_NAME}}}}\"\nnetworks:\n  \"{{{{NETWORK_NAME}}}}\":\n    driver: bridge\n    ipam:\n      config:\n        - subnet: \"{{{{SUBNET}}}}\"\n"
    )
}

pub fn validate_compose_template_schema(template: &str, metadata: &Value) -> Result<(), String> {
    let normalized = template.trim();
    if normalized.is_empty() {
        return Err("challenge runtime template is missing".to_string());
    }

    let tokens = collect_placeholder_tokens(normalized)?;
    if tokens.is_empty() {
        return Ok(());
    }

    let variable_defs = parse_compose_variable_defs(metadata)?;
    let variable_names = variable_defs.keys().cloned().collect::<HashSet<String>>();

    for token in &tokens {
        if is_reserved_placeholder(token) {
            continue;
        }

        let Some(variable_name) = token.strip_prefix("VAR:") else {
            return Err(format!(
                "unsupported compose placeholder '{{{{{token}}}}}', only reserved placeholders and '{{{{VAR:NAME}}}}' are allowed"
            ));
        };

        validate_variable_name(variable_name)?;
        if !variable_names.contains(variable_name) {
            return Err(format!(
                "compose variable '{variable_name}' is referenced but not defined in metadata.{COMPOSE_VARIABLES_KEY}"
            ));
        }
    }

    for (name, def) in &variable_defs {
        if def.required && def.value.trim().is_empty() {
            return Err(format!(
                "compose variable '{name}' is required but resolved value is empty"
            ));
        }
    }

    Ok(())
}

pub fn render_compose_template_variables(
    template: &str,
    metadata: &Value,
) -> Result<String, String> {
    let variable_defs = parse_compose_variable_defs(metadata)?;
    if variable_defs.is_empty() {
        return Ok(template.to_string());
    }

    let mut rendered = template.to_string();
    let tokens = collect_placeholder_tokens(template)?;
    let mut unique_tokens = HashSet::new();
    for token in tokens {
        if !unique_tokens.insert(token.clone()) {
            continue;
        }

        let Some(variable_name) = token.strip_prefix("VAR:") else {
            continue;
        };

        let Some(def) = variable_defs.get(variable_name) else {
            return Err(format!(
                "compose variable '{variable_name}' is referenced but not defined in metadata.{COMPOSE_VARIABLES_KEY}"
            ));
        };

        let placeholder = format!("{{{{{token}}}}}");
        rendered = rendered.replace(&placeholder, &def.value);
    }

    Ok(rendered)
}

fn is_reserved_placeholder(token: &str) -> bool {
    RESERVED_PLACEHOLDERS.iter().any(|item| item == &token)
}

fn collect_placeholder_tokens(template: &str) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    let mut cursor = 0usize;

    while let Some(start_offset) = template[cursor..].find("{{") {
        let start = cursor + start_offset;
        if template[cursor..start].contains("}}") {
            return Err("compose template contains an unmatched '}}' token".to_string());
        }
        let after_start = start + 2;
        let Some(end_offset) = template[after_start..].find("}}") else {
            return Err("compose template contains an unclosed '{{' placeholder".to_string());
        };

        let end = after_start + end_offset;
        let token = template[after_start..end].trim();
        if token.is_empty() {
            return Err("compose template contains an empty placeholder '{{}}'".to_string());
        }

        out.push(token.to_string());
        cursor = end + 2;
    }

    if template[cursor..].contains("}}") {
        return Err("compose template contains an unmatched '}}' token".to_string());
    }

    Ok(out)
}

fn parse_compose_variable_defs(
    metadata: &Value,
) -> Result<HashMap<String, ComposeVariableDef>, String> {
    let Some(value) = metadata.get(COMPOSE_VARIABLES_KEY) else {
        return Ok(HashMap::new());
    };

    match value {
        Value::Array(items) => parse_compose_variables_from_array(items),
        Value::Object(map) => parse_compose_variables_from_object_map(map),
        _ => Err(format!(
            "metadata.{COMPOSE_VARIABLES_KEY} must be an object map or array"
        )),
    }
}

fn parse_compose_variables_from_array(
    items: &[Value],
) -> Result<HashMap<String, ComposeVariableDef>, String> {
    let mut out = HashMap::new();

    for (index, item) in items.iter().enumerate() {
        let Value::Object(entry) = item else {
            return Err(format!(
                "metadata.{COMPOSE_VARIABLES_KEY}[{index}] must be an object"
            ));
        };

        let name = entry
            .get("name")
            .and_then(Value::as_str)
            .map(str::trim)
            .ok_or_else(|| {
                format!("metadata.{COMPOSE_VARIABLES_KEY}[{index}].name must be a non-empty string")
            })?
            .to_string();

        validate_variable_name(&name)?;
        if out.contains_key(&name) {
            return Err(format!(
                "metadata.{COMPOSE_VARIABLES_KEY} contains duplicated variable '{name}'"
            ));
        }

        let value = extract_variable_value(entry, &format!("{COMPOSE_VARIABLES_KEY}[{index}]"))?;
        out.insert(name, value);
    }

    Ok(out)
}

fn parse_compose_variables_from_object_map(
    map: &serde_json::Map<String, Value>,
) -> Result<HashMap<String, ComposeVariableDef>, String> {
    let mut out = HashMap::new();

    for (name, raw_value) in map {
        validate_variable_name(name)?;
        if out.contains_key(name) {
            return Err(format!(
                "metadata.{COMPOSE_VARIABLES_KEY} contains duplicated variable '{name}'"
            ));
        }

        let value = match raw_value {
            Value::Object(entry) => {
                extract_variable_value(entry, &format!("{COMPOSE_VARIABLES_KEY}.{name}"))?
            }
            _ => ComposeVariableDef {
                value: stringify_scalar(raw_value, &format!("{COMPOSE_VARIABLES_KEY}.{name}"))?
                    .unwrap_or_default(),
                required: false,
            },
        };

        out.insert(name.clone(), value);
    }

    Ok(out)
}

fn extract_variable_value(
    entry: &serde_json::Map<String, Value>,
    field: &str,
) -> Result<ComposeVariableDef, String> {
    let required = entry
        .get("required")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    let explicit_value = entry
        .get("value")
        .map(|value| stringify_scalar(value, &format!("{field}.value")))
        .transpose()?
        .flatten();
    let default_value = entry
        .get("default")
        .map(|value| stringify_scalar(value, &format!("{field}.default")))
        .transpose()?
        .flatten();

    let value = explicit_value.or(default_value).unwrap_or_default();
    Ok(ComposeVariableDef { value, required })
}

fn stringify_scalar(value: &Value, field: &str) -> Result<Option<String>, String> {
    match value {
        Value::Null => Ok(None),
        Value::String(raw) => Ok(Some(raw.trim().to_string())),
        Value::Number(raw) => Ok(Some(raw.to_string())),
        Value::Bool(raw) => Ok(Some(raw.to_string())),
        _ => Err(format!("{field} must be string/number/bool/null")),
    }
}

fn validate_variable_name(name: &str) -> Result<(), String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("compose variable name must not be empty".to_string());
    }
    if trimmed.len() > 64 {
        return Err(format!(
            "compose variable '{trimmed}' is too long (max 64 chars)"
        ));
    }

    let valid = trimmed
        .chars()
        .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '_');
    if !valid {
        return Err(format!(
            "compose variable '{trimmed}' is invalid, only [A-Z0-9_] is allowed"
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{
        build_single_image_compose_template, parse_runtime_metadata_options,
        render_compose_template_variables, validate_compose_template_schema, RuntimeAccessMode,
        RuntimeEndpointProtocol, RuntimeMode,
    };

    #[test]
    fn accepts_reserved_only_template() {
        let template = "services:\n  app:\n    image: test\n    container_name: {{PROJECT_NAME}}";
        let metadata = json!({});
        let res = validate_compose_template_schema(template, &metadata);
        assert!(res.is_ok());
    }

    #[test]
    fn accepts_defined_custom_variables() {
        let template = "services:\n  app:\n    environment:\n      - PORT={{VAR:APP_PORT}}";
        let metadata = json!({
            "compose_variables": [
                {"name": "APP_PORT", "value": "8080", "required": true}
            ]
        });

        let res = validate_compose_template_schema(template, &metadata);
        assert!(res.is_ok());

        let rendered = render_compose_template_variables(template, &metadata).unwrap();
        assert!(rendered.contains("PORT=8080"));
    }

    #[test]
    fn rejects_missing_custom_variable_definition() {
        let template = "services:\n  app:\n    environment:\n      - PORT={{VAR:APP_PORT}}";
        let metadata = json!({});
        let res = validate_compose_template_schema(template, &metadata);
        assert!(res.is_err());
    }

    #[test]
    fn rejects_unsupported_placeholder() {
        let template = "services:\n  app:\n    environment:\n      - PORT={{APP_PORT}}";
        let metadata = json!({});
        let res = validate_compose_template_schema(template, &metadata);
        assert!(res.is_err());
    }

    #[test]
    fn parses_single_image_runtime_metadata() {
        let metadata = json!({
            "runtime": {
                "mode": "single_image",
                "image": "nginx:alpine",
                "internal_port": 80,
                "protocol": "http"
            }
        });

        let options = parse_runtime_metadata_options(&metadata).unwrap();
        assert_eq!(options.mode, RuntimeMode::SingleImage);
        assert_eq!(options.access_mode, RuntimeAccessMode::Direct);

        let single = options.single_image.unwrap();
        assert_eq!(single.image, "nginx:alpine");
        assert_eq!(single.internal_port, 80);
        assert_eq!(single.protocol, RuntimeEndpointProtocol::Http);
    }

    #[test]
    fn compose_mode_defaults_to_ssh_bastion_access() {
        let metadata = json!({});
        let options = parse_runtime_metadata_options(&metadata).unwrap();
        assert_eq!(options.mode, RuntimeMode::Compose);
        assert_eq!(options.access_mode, RuntimeAccessMode::SshBastion);
    }

    #[test]
    fn single_image_template_is_schema_valid() {
        let template = build_single_image_compose_template("nginx:alpine", 80);
        let metadata = json!({});
        let res = validate_compose_template_schema(&template, &metadata);
        assert!(res.is_ok());
    }
}
