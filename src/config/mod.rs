//! Strongly typed configuration parsed once at startup.
//!
//! Every value comes from environment variables. Invalid critical configuration
//! fails fast before the server binds a port.

mod env;
mod secret;

use std::{
    fmt,
    net::{IpAddr, SocketAddr},
    str::FromStr,
    time::Duration,
};

use axum::http::HeaderValue;

pub use env::ConfigError;
pub use secret::Secret;

use env::{optional, optional_or, required};

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub environment: Environment,
    pub server: ServerConfig,
    pub telemetry: TelemetryConfig,
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub auth: AuthConfig,
    pub storage: Option<StorageConfig>,
    pub cors: CorsConfig,
    pub jobs: JobsConfig,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        let environment: Environment = optional_or("APP_ENV", Environment::Development)?;

        let config = Self {
            environment,
            server: ServerConfig {
                host: optional_or("APP_HOST", IpAddr::from([0, 0, 0, 0]))?,
                port: optional_or("APP_PORT", 8080)?,
                request_body_limit_bytes: optional_or("APP_REQUEST_BODY_LIMIT", 2 * 1024 * 1024)?,
                request_timeout: Duration::from_secs(optional_or("APP_REQUEST_TIMEOUT", 30)?),
                api_docs_enabled: optional_or(
                    "APP_API_DOCS_ENABLED",
                    environment == Environment::Development,
                )?,
            },
            telemetry: TelemetryConfig {
                format: optional_or("LOG_FORMAT", LogFormat::default_for(environment))?,
                level: optional_or("LOG_LEVEL", "info".to_owned())?,
            },
            database: DatabaseConfig {
                url: Secret::new(required("DATABASE_URL")?),
                max_connections: optional_or("DATABASE_MAX_CONNECTIONS", 20)?,
                min_connections: optional_or("DATABASE_MIN_CONNECTIONS", 2)?,
                acquire_timeout: Duration::from_secs(optional_or("DATABASE_ACQUIRE_TIMEOUT", 5)?),
                run_migrations: optional_or("DATABASE_RUN_MIGRATIONS", false)?,
            },
            redis: RedisConfig {
                url: Secret::new(required("REDIS_URL")?),
            },
            auth: AuthConfig {
                jwt_secret: Secret::new(required("JWT_SECRET")?),
                jwt_issuer: optional_or("JWT_ISSUER", "miracle-api".to_owned())?,
                access_token_ttl: Duration::from_secs(optional_or("JWT_ACCESS_TTL", 900)?),
                refresh_token_ttl: Duration::from_secs(optional_or("JWT_REFRESH_TTL", 2_592_000)?),
            },
            storage: StorageConfig::from_env()?,
            cors: CorsConfig::from_env()?,
            jobs: JobsConfig {
                enabled: optional_or("JOBS_ENABLED", true)?,
                poll_interval: Duration::from_millis(optional_or("JOBS_POLL_INTERVAL_MS", 1000)?),
                batch_size: optional_or("JOBS_BATCH_SIZE", 10)?,
            },
        };

        config.validate()?;
        Ok(config)
    }

    /// Cross-field and environment-specific rules.
    fn validate(&self) -> Result<(), ConfigError> {
        if self.auth.jwt_secret.expose().len() < 32 {
            return Err(ConfigError::invalid(
                "JWT_SECRET",
                "must be at least 32 bytes",
            ));
        }
        if self.auth.access_token_ttl >= self.auth.refresh_token_ttl {
            return Err(ConfigError::invalid(
                "JWT_ACCESS_TTL",
                "must be shorter than JWT_REFRESH_TTL",
            ));
        }
        if self.database.min_connections > self.database.max_connections {
            return Err(ConfigError::invalid(
                "DATABASE_MIN_CONNECTIONS",
                "must not exceed DATABASE_MAX_CONNECTIONS",
            ));
        }
        if self.environment.is_deployed() {
            if self.cors.allow_any_origin {
                return Err(ConfigError::invalid(
                    "FRONTEND_URL",
                    "wildcard origin is not allowed outside development",
                ));
            }
            if self.auth.jwt_secret.expose().starts_with("change-me") {
                return Err(ConfigError::invalid(
                    "JWT_SECRET",
                    "placeholder secret detected",
                ));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Environment {
    Development,
    Test,
    Staging,
    Production,
}

impl Environment {
    /// Staging and production must follow production-grade security policy.
    pub fn is_deployed(self) -> bool {
        matches!(self, Self::Staging | Self::Production)
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Development => "development",
            Self::Test => "test",
            Self::Staging => "staging",
            Self::Production => "production",
        }
    }
}

impl fmt::Display for Environment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Environment {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_ascii_lowercase().as_str() {
            "development" | "dev" | "local" => Ok(Self::Development),
            "test" => Ok(Self::Test),
            "staging" => Ok(Self::Staging),
            "production" | "prod" => Ok(Self::Production),
            other => Err(format!("unknown environment `{other}`")),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub host: IpAddr,
    pub port: u16,
    pub request_body_limit_bytes: usize,
    pub request_timeout: Duration,
    pub api_docs_enabled: bool,
}

impl ServerConfig {
    pub fn socket_addr(&self) -> SocketAddr {
        SocketAddr::new(self.host, self.port)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogFormat {
    Pretty,
    Json,
}

impl LogFormat {
    fn default_for(environment: Environment) -> Self {
        if environment.is_deployed() {
            Self::Json
        } else {
            Self::Pretty
        }
    }
}

impl FromStr for LogFormat {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_ascii_lowercase().as_str() {
            "pretty" => Ok(Self::Pretty),
            "json" => Ok(Self::Json),
            other => Err(format!("unknown log format `{other}`")),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TelemetryConfig {
    pub format: LogFormat,
    pub level: String,
}

#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub url: Secret,
    pub max_connections: u32,
    pub min_connections: u32,
    pub acquire_timeout: Duration,
    pub run_migrations: bool,
}

#[derive(Debug, Clone)]
pub struct RedisConfig {
    pub url: Secret,
}

#[derive(Debug, Clone)]
pub struct AuthConfig {
    pub jwt_secret: Secret,
    pub jwt_issuer: String,
    pub access_token_ttl: Duration,
    pub refresh_token_ttl: Duration,
}

#[derive(Debug, Clone)]
pub struct StorageConfig {
    pub endpoint: Option<String>,
    pub bucket: String,
    pub region: String,
    pub access_key: Secret,
    pub secret_key: Secret,
}

impl StorageConfig {
    /// Storage stays optional until the documents module ships; when `S3_BUCKET`
    /// is present, the remaining S3 variables become mandatory.
    fn from_env() -> Result<Option<Self>, ConfigError> {
        let Some(bucket) = optional::<String>("S3_BUCKET")? else {
            return Ok(None);
        };
        Ok(Some(Self {
            endpoint: optional("S3_ENDPOINT")?,
            bucket,
            region: required("S3_REGION")?,
            access_key: Secret::new(required("S3_ACCESS_KEY")?),
            secret_key: Secret::new(required("S3_SECRET_KEY")?),
        }))
    }
}

#[derive(Debug, Clone)]
pub struct CorsConfig {
    pub allowed_origins: Vec<HeaderValue>,
    pub allow_any_origin: bool,
}

impl CorsConfig {
    fn from_env() -> Result<Self, ConfigError> {
        let raw: String = required("FRONTEND_URL")?;
        let mut allowed_origins = Vec::new();
        let mut allow_any_origin = false;

        for origin in raw.split(',').map(str::trim).filter(|o| !o.is_empty()) {
            if origin == "*" {
                allow_any_origin = true;
                continue;
            }
            let value = HeaderValue::from_str(origin.trim_end_matches('/'))
                .map_err(|_| ConfigError::invalid("FRONTEND_URL", "contains an invalid origin"))?;
            allowed_origins.push(value);
        }

        if allowed_origins.is_empty() && !allow_any_origin {
            return Err(ConfigError::invalid(
                "FRONTEND_URL",
                "at least one origin is required",
            ));
        }

        Ok(Self {
            allowed_origins,
            allow_any_origin,
        })
    }
}

#[derive(Debug, Clone)]
pub struct JobsConfig {
    pub enabled: bool,
    pub poll_interval: Duration,
    pub batch_size: i64,
}
