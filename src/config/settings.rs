use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub server: ServerConfig,
    pub ai: AiConfig,
    pub workspace: WorkspaceConfig,
    pub security: SecurityConfig,
    pub performance: PerformanceConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub transport: String,
    pub socket_path: Option<String>,
    pub log_level: String,
    pub max_concurrent_sessions: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConfig {
    pub provider: String,
    pub model: String,
    pub timeout: u32,
    pub max_turns: u32,
    pub providers: ProviderConfigs,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfigs {
    pub openai: ProviderConfig,
    pub anthropic: ProviderConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub api_key: Option<String>,
    pub base_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    pub exclude_patterns: Vec<String>,
    pub max_file_size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub require_approval: bool,
    pub approval_timeout: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    pub stream_buffer_size: u32,
    pub session_cache_size: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                transport: "stdio".to_string(),
                socket_path: Some("/tmp/kaiak.sock".to_string()),
                log_level: "info".to_string(),
                max_concurrent_sessions: 10,
            },
            ai: AiConfig {
                provider: "openai".to_string(),
                model: "gpt-4".to_string(),
                timeout: 300,
                max_turns: 50,
                providers: ProviderConfigs {
                    openai: ProviderConfig {
                        api_key: std::env::var("OPENAI_API_KEY").ok(),
                        base_url: Some("https://api.openai.com/v1".to_string()),
                    },
                    anthropic: ProviderConfig {
                        api_key: std::env::var("ANTHROPIC_API_KEY").ok(),
                        base_url: None,
                    },
                },
            },
            workspace: WorkspaceConfig {
                exclude_patterns: vec![
                    "target/".to_string(),
                    "node_modules/".to_string(),
                    ".git/".to_string(),
                    "*.tmp".to_string(),
                ],
                max_file_size: 1_048_576, // 1MB
            },
            security: SecurityConfig {
                require_approval: true,
                approval_timeout: 300, // 5 minutes
            },
            performance: PerformanceConfig {
                stream_buffer_size: 1000,
                session_cache_size: 100,
            },
        }
    }
}

impl Settings {
    pub fn load() -> Result<Self> {
        let mut settings = Self::default();

        // Override with environment variables
        if let Ok(val) = std::env::var("KAIAK_LOG_LEVEL") {
            settings.server.log_level = val;
        }

        if let Ok(val) = std::env::var("KAIAK_TRANSPORT") {
            settings.server.transport = val;
        }

        if let Ok(val) = std::env::var("KAIAK_SOCKET_PATH") {
            settings.server.socket_path = Some(val);
        }

        if let Ok(val) = std::env::var("KAIAK_MAX_SESSIONS") {
            settings.server.max_concurrent_sessions = val.parse().unwrap_or(10);
        }

        if let Ok(val) = std::env::var("KAIAK_AI_PROVIDER") {
            settings.ai.provider = val;
        }

        if let Ok(val) = std::env::var("KAIAK_AI_MODEL") {
            settings.ai.model = val;
        }

        Ok(settings)
    }

    pub fn config_path() -> PathBuf {
        if let Ok(custom_path) = std::env::var("KAIAK_CONFIG_PATH") {
            PathBuf::from(custom_path)
        } else {
            dirs::config_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("kaiak")
                .join("config.toml")
        }
    }

    pub fn validate(&self) -> Result<()> {
        if self.ai.providers.openai.api_key.is_none()
            && self.ai.providers.anthropic.api_key.is_none() {
            anyhow::bail!("At least one AI provider API key must be configured");
        }

        if self.server.max_concurrent_sessions == 0 {
            anyhow::bail!("Max concurrent sessions must be greater than 0");
        }

        Ok(())
    }
}