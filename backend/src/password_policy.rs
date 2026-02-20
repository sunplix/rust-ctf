use std::collections::HashSet;

use serde::Serialize;

use crate::config::AppConfig;

const COMMON_WEAK_PATTERNS: &[&str] = &[
    "password",
    "passw0rd",
    "qwerty",
    "qwertyui",
    "qwerty123",
    "abc123",
    "letmein",
    "admin",
    "welcome",
    "iloveyou",
    "111111",
    "123456",
    "12345678",
    "123456789",
    "123123",
];

const MIN_SEQUENCE_LEN: usize = 4;

#[derive(Debug, Clone, Copy, Default)]
pub struct PasswordContext<'a> {
    pub username: Option<&'a str>,
    pub email: Option<&'a str>,
}

#[derive(Debug, Clone)]
pub struct PasswordStrengthReport {
    pub score: u8,
    pub length: usize,
    pub unique_chars: usize,
    pub has_lowercase: bool,
    pub has_uppercase: bool,
    pub has_digit: bool,
    pub has_symbol: bool,
    pub has_whitespace: bool,
    pub has_common_pattern: bool,
    pub has_sequence: bool,
    pub has_repeating_runs: bool,
    pub contains_user_identifier: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct PasswordPolicySnapshot {
    pub min_length: usize,
    pub min_strength_score: u8,
    pub require_lowercase: bool,
    pub require_uppercase: bool,
    pub require_digit: bool,
    pub require_symbol: bool,
    pub min_unique_chars: usize,
    pub block_weak_patterns: bool,
}

impl PasswordPolicySnapshot {
    pub fn from_config(config: &AppConfig) -> Self {
        Self {
            min_length: config.auth_password_min_length.clamp(8, 128) as usize,
            min_strength_score: config.auth_password_min_strength_score.clamp(1, 4),
            require_lowercase: config.auth_password_require_lowercase,
            require_uppercase: config.auth_password_require_uppercase,
            require_digit: config.auth_password_require_digit,
            require_symbol: config.auth_password_require_symbol,
            min_unique_chars: config.auth_password_min_unique_chars.clamp(1, 64) as usize,
            block_weak_patterns: config.auth_password_block_weak_patterns,
        }
    }
}

pub fn enforce_password_policy(
    config: &AppConfig,
    password: &str,
    context: PasswordContext<'_>,
) -> Result<PasswordStrengthReport, String> {
    let policy = PasswordPolicySnapshot::from_config(config);
    let report = evaluate_password_strength(password, context);
    let mut issues = Vec::new();

    if report.length < policy.min_length {
        issues.push(format!(
            "password must be at least {} characters",
            policy.min_length
        ));
    }

    if policy.require_lowercase && !report.has_lowercase {
        issues.push("password must include lowercase letters".to_string());
    }

    if policy.require_uppercase && !report.has_uppercase {
        issues.push("password must include uppercase letters".to_string());
    }

    if policy.require_digit && !report.has_digit {
        issues.push("password must include digits".to_string());
    }

    if policy.require_symbol && !report.has_symbol {
        issues.push("password must include symbols".to_string());
    }

    if report.unique_chars < policy.min_unique_chars {
        issues.push(format!(
            "password must include at least {} unique characters",
            policy.min_unique_chars
        ));
    }

    if report.has_whitespace {
        issues.push("password must not contain whitespace".to_string());
    }

    if policy.block_weak_patterns {
        if report.has_common_pattern {
            issues.push("password contains weak/common patterns".to_string());
        }
        if report.has_sequence {
            issues.push("password contains sequential character runs".to_string());
        }
        if report.has_repeating_runs {
            issues.push("password contains repeated character runs".to_string());
        }
        if report.contains_user_identifier {
            issues.push("password must not contain username or email local-part".to_string());
        }
    }

    if report.score < policy.min_strength_score {
        issues.push(format!(
            "password strength score is {} but minimum required score is {}",
            report.score, policy.min_strength_score
        ));
    }

    if issues.is_empty() {
        Ok(report)
    } else {
        Err(issues.join("; "))
    }
}

pub fn evaluate_password_strength(
    password: &str,
    context: PasswordContext<'_>,
) -> PasswordStrengthReport {
    let length = password.chars().count();
    let unique_chars = password.chars().collect::<HashSet<_>>().len();
    let has_lowercase = password.chars().any(|ch| ch.is_ascii_lowercase());
    let has_uppercase = password.chars().any(|ch| ch.is_ascii_uppercase());
    let has_digit = password.chars().any(|ch| ch.is_ascii_digit());
    let has_symbol = password
        .chars()
        .any(|ch| !ch.is_ascii_alphanumeric() && !ch.is_ascii_whitespace());
    let has_whitespace = password.chars().any(|ch| ch.is_whitespace());
    let has_common_pattern = contains_common_pattern(password);
    let has_sequence = contains_ascii_sequence(password);
    let has_repeating_runs = contains_repeating_runs(password);
    let contains_user_identifier = contains_user_identifier(password, context);

    let mut score = 0_i32;

    if length >= 8 {
        score += 1;
    }
    if length >= 12 {
        score += 1;
    }
    if length >= 16 {
        score += 1;
    }
    if length >= 20 {
        score += 1;
    }

    let class_count = [has_lowercase, has_uppercase, has_digit, has_symbol]
        .into_iter()
        .filter(|value| *value)
        .count();

    if class_count >= 2 {
        score += 1;
    }
    if class_count >= 3 {
        score += 1;
    }
    if class_count >= 4 {
        score += 1;
    }

    let unique_ratio = if length > 0 {
        unique_chars as f64 / length as f64
    } else {
        0.0
    };
    if unique_ratio >= 0.55 {
        score += 1;
    }

    if has_whitespace {
        score -= 1;
    }
    if has_common_pattern {
        score -= 2;
    }
    if has_sequence {
        score -= 1;
    }
    if has_repeating_runs {
        score -= 1;
    }
    if contains_user_identifier {
        score -= 2;
    }

    PasswordStrengthReport {
        score: score.clamp(0, 4) as u8,
        length,
        unique_chars,
        has_lowercase,
        has_uppercase,
        has_digit,
        has_symbol,
        has_whitespace,
        has_common_pattern,
        has_sequence,
        has_repeating_runs,
        contains_user_identifier,
    }
}

fn contains_common_pattern(password: &str) -> bool {
    if password.is_empty() {
        return false;
    }

    let lowered = password.to_ascii_lowercase();
    if COMMON_WEAK_PATTERNS
        .iter()
        .any(|pattern| lowered.contains(pattern))
    {
        return true;
    }

    let mut chars = password.chars();
    if let Some(first) = chars.next() {
        if chars.all(|ch| ch == first) {
            return true;
        }
    }

    false
}

fn contains_repeating_runs(password: &str) -> bool {
    let mut chars = password.chars();
    let Some(mut prev) = chars.next() else {
        return false;
    };

    let mut run = 1_usize;
    for current in chars {
        if current == prev {
            run += 1;
            if run >= 3 {
                return true;
            }
        } else {
            run = 1;
        }
        prev = current;
    }

    false
}

fn contains_ascii_sequence(password: &str) -> bool {
    let bytes = password.to_ascii_lowercase().into_bytes();
    if bytes.len() < MIN_SEQUENCE_LEN {
        return false;
    }

    let mut asc_run = 1_usize;
    let mut desc_run = 1_usize;

    for index in 1..bytes.len() {
        let prev = bytes[index - 1];
        let current = bytes[index];

        if !prev.is_ascii_alphanumeric() || !current.is_ascii_alphanumeric() {
            asc_run = 1;
            desc_run = 1;
            continue;
        }

        if current == prev.saturating_add(1) {
            asc_run += 1;
        } else {
            asc_run = 1;
        }

        if current.saturating_add(1) == prev {
            desc_run += 1;
        } else {
            desc_run = 1;
        }

        if asc_run >= MIN_SEQUENCE_LEN || desc_run >= MIN_SEQUENCE_LEN {
            return true;
        }
    }

    false
}

fn contains_user_identifier(password: &str, context: PasswordContext<'_>) -> bool {
    let lowered_password = password.to_ascii_lowercase();

    if let Some(username) = context.username {
        let normalized = username.trim().to_ascii_lowercase();
        if normalized.len() >= 3 && lowered_password.contains(&normalized) {
            return true;
        }
    }

    if let Some(email) = context.email {
        let normalized_email = email.trim().to_ascii_lowercase();
        let local_part = normalized_email.split('@').next().unwrap_or_default();
        if local_part.len() >= 3 && lowered_password.contains(local_part) {
            return true;
        }
    }

    false
}
