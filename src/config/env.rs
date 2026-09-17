use std::{env, str::FromStr};

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("missing required environment variable `{0}`")]
    Missing(&'static str),

    #[error("invalid value for `{name}`: {reason}")]
    Invalid { name: &'static str, reason: String },
}

impl ConfigError {
    pub fn invalid(name: &'static str, reason: impl Into<String>) -> Self {
        Self::Invalid {
            name,
            reason: reason.into(),
        }
    }
}

pub fn optional<T>(name: &'static str) -> Result<Option<T>, ConfigError>
where
    T: FromStr,
    T::Err: std::fmt::Display,
{
    match env::var(name) {
        Ok(value) if !value.trim().is_empty() => value
            .trim()
            .parse()
            .map(Some)
            .map_err(|error: T::Err| ConfigError::invalid(name, error.to_string())),
        _ => Ok(None),
    }
}

pub fn optional_or<T>(name: &'static str, default: T) -> Result<T, ConfigError>
where
    T: FromStr,
    T::Err: std::fmt::Display,
{
    Ok(optional(name)?.unwrap_or(default))
}

pub fn required<T>(name: &'static str) -> Result<T, ConfigError>
where
    T: FromStr,
    T::Err: std::fmt::Display,
{
    optional(name)?.ok_or(ConfigError::Missing(name))
}
