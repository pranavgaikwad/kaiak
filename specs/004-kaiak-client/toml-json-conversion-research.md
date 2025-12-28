# TOML to JSON Configuration Conversion Research

**Date**: 2025-12-27
**Feature**: 004-kaiak-client
**Research Focus**: TOML configuration file conversion to JSON-RPC compatible input

## Executive Summary

This research examines how to implement TOML configuration file support for the Kaiak client while ensuring all TOML configurations can be accurately converted to JSON format for the server's JSON-RPC API. The analysis covers current codebase usage, recommended Rust crates, conversion patterns, validation strategies, and integration approaches.

## 1. Current Kaiak Configuration Architecture

### 1.1 Existing Configuration Types

The Kaiak codebase has two distinct configuration systems:

#### Server-wide Configuration (`src/config/settings.rs`)
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerSettings {
    pub server: ServerConfig,
    pub ai: AiDefaultsConfig,
    pub workspace: DefaultWorkspaceConfig,
    pub security: SecurityConfig,
    pub performance: PerformanceConfig,
}
```

**Purpose**: Controls the Kaiak server itself (NOT individual agent sessions)
**Location**: `~/.config/kaiak/config.toml` (referenced in user docs)
**Current Status**: Partially implemented with environment variable overrides

#### Per-Session Agent Configuration (`src/models/configuration.rs`)
```rust
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct AgentConfiguration {
    #[validate(nested)]
    pub workspace: WorkspaceConfig,
    pub model: GooseModelConfig,  // serde_json::Value
    #[validate(nested)]
    pub tools: ToolConfig,
    pub session: GooseSessionConfig,
    #[validate(nested)]
    pub permissions: PermissionConfig,
}
```

**Purpose**: Sent by IDE clients via `kaiak/configure` JSON-RPC endpoint
**Current Status**: Fully implemented with `validator` crate integration
**Receives**: JSON input from clients over JSON-RPC protocol

### 1.2 Current Dependencies

From `Cargo.toml`:
```toml
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
toml = "0.8"
validator = { version = "0.18", features = ["derive"] }
```

**Analysis**:
- Already has `toml = "0.8"` dependency (latest stable)
- `serde` with derive feature enables unified serialization
- `validator` used for runtime validation of agent configurations
- `serde_json` for JSON-RPC communication

### 1.3 Validation System

The codebase uses two validation approaches:

1. **Structural validation** (`validator` crate):
```rust
// From src/models/configuration.rs
#[derive(Validate)]
pub struct WorkspaceConfig {
    #[validate(custom(function = "validate_workspace_path"))]
    pub working_dir: PathBuf,
    #[validate(length(min = 1, message = "At least one include pattern is required"))]
    pub include_patterns: Vec<String>,
    // ...
}
```

2. **Configuration validation** (`src/config/validation.rs`):
```rust
pub struct ConfigurationValidator {
    strict_mode: bool,
    warnings: Vec<String>,
    errors: Vec<String>,
}

impl ConfigurationValidator {
    pub fn validate_settings(&mut self, settings: &ServerSettings) -> Result<()>
}
```

## 2. TOML to JSON Conversion Patterns

### 2.1 Recommended Approach: Direct Serde Conversion

The most idiomatic and maintainable approach leverages Serde's cross-format capabilities:

```rust
use serde::{Deserialize, Serialize};
use anyhow::{Context, Result};

/// Universal configuration that works with both TOML and JSON
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct UnifiedConfig {
    #[validate(nested)]
    pub workspace: WorkspaceConfig,
    pub model: GooseModelConfig,
    // ... other fields
}

impl UnifiedConfig {
    /// Load from TOML file
    pub fn from_toml_file(path: impl AsRef<std::path::Path>) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref())
            .context("Failed to read TOML configuration file")?;

        let config: Self = toml::from_str(&content)
            .context("Failed to parse TOML configuration")?;

        // Validate after deserialization
        config.validate()
            .context("Configuration validation failed")?;

        Ok(config)
    }

    /// Convert to JSON string for JSON-RPC
    pub fn to_json_string(&self) -> Result<String> {
        serde_json::to_string_pretty(self)
            .context("Failed to serialize configuration to JSON")
    }

    /// Convert to JSON Value for JSON-RPC
    pub fn to_json_value(&self) -> Result<serde_json::Value> {
        serde_json::to_value(self)
            .context("Failed to convert configuration to JSON value")
    }

    /// Direct TOML -> JSON conversion without intermediate struct
    pub fn toml_file_to_json(toml_path: impl AsRef<std::path::Path>) -> Result<serde_json::Value> {
        let content = std::fs::read_to_string(toml_path.as_ref())?;
        let toml_value: toml::Value = toml::from_str(&content)?;
        let json_value = serde_json::to_value(&toml_value)?;
        Ok(json_value)
    }
}
```

**Key Benefits**:
- Single struct definition works for both formats
- Automatic validation via `validator` crate
- Type safety guaranteed by Rust compiler
- No manual field mapping required

### 2.2 Alternative: Cross-Format Value Conversion

For cases where you need to work with untyped data:

```rust
use anyhow::Result;

/// Convert TOML file directly to JSON Value (untyped)
pub fn toml_to_json_value(toml_content: &str) -> Result<serde_json::Value> {
    // Parse TOML into toml::Value
    let toml_value: toml::Value = toml::from_str(toml_content)?;

    // Convert to JSON Value
    let json_value = serde_json::to_value(&toml_value)?;

    Ok(json_value)
}

/// Convert TOML file to JSON string
pub fn toml_file_to_json_string(path: impl AsRef<std::path::Path>) -> Result<String> {
    let content = std::fs::read_to_string(path)?;
    let json_value = toml_to_json_value(&content)?;
    let json_string = serde_json::to_string_pretty(&json_value)?;
    Ok(json_string)
}
```

**Use Cases**:
- Configuration file format conversion utilities
- Dynamic configuration without predefined schemas
- CLI tools that transform between formats

### 2.3 Crate Recommendations

#### Primary: `toml` (v0.8.23)

Already in `Cargo.toml`. Latest stable release with excellent Serde integration.

**Pros**:
- Standard Serde traits (Serialize/Deserialize)
- Mature, stable, widely used
- No additional dependencies needed
- Perfect for configuration parsing

**Cons**:
- Does not preserve formatting/comments (not needed for Kaiak use case)

**When to use**: Default choice for all configuration parsing needs

#### Alternative: `toml_edit` (v0.23.7)

**NOT recommended for Kaiak** unless you need format-preserving edits.

**Pros**:
- Preserves comments, whitespace, ordering
- Suitable for programmatic config file editing

**Cons**:
- More complex API
- Heavier dependency
- Overkill for read-only configuration

**When to use**: Only if you need to programmatically edit user config files while preserving comments

#### Validation: `validator` (v0.18)

Already in use. Perfect for our needs.

**Integration example**:
```rust
use validator::{Validate, ValidationError};

#[derive(Debug, Deserialize, Validate)]
pub struct ServerConfig {
    #[validate(length(min = 1))]
    pub transport: String,

    #[validate(range(min = 1, max = 100))]
    pub max_concurrent_sessions: u32,

    #[validate(custom(function = "validate_log_level"))]
    pub log_level: String,
}

fn validate_log_level(level: &str) -> Result<(), ValidationError> {
    match level.to_lowercase().as_str() {
        "trace" | "debug" | "info" | "warn" | "error" => Ok(()),
        _ => Err(ValidationError::new("invalid_log_level")),
    }
}
```

#### Schema Validation (Optional): `schemars` + `jsonschema`

**NOT immediately needed** but useful for future JSON Schema generation:

```rust
use schemars::{schema_for, JsonSchema};

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct AgentConfiguration {
    // fields...
}

// Generate JSON Schema
let schema = schema_for!(AgentConfiguration);
println!("{}", serde_json::to_string_pretty(&schema).unwrap());
```

**Use cases**:
- Generate JSON Schema documentation
- Validate JSON-RPC requests against schema
- IDE autocomplete for config files

**Recommendation**: Consider for Phase 2 (documentation enhancement)

## 3. Error Handling Strategies

### 3.1 Comprehensive Error Types

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Failed to read configuration file: {0}")]
    FileRead(#[from] std::io::Error),

    #[error("Invalid TOML syntax: {0}")]
    TomlParse(#[from] toml::de::Error),

    #[error("Failed to convert to JSON: {0}")]
    JsonConversion(#[from] serde_json::Error),

    #[error("Configuration validation failed: {0}")]
    Validation(String),

    #[error("Missing required field: {field}")]
    MissingField { field: String },

    #[error("Invalid value for {field}: {reason}")]
    InvalidValue { field: String, reason: String },

    #[error("Configuration file not found at {path}")]
    FileNotFound { path: String },
}

// User-friendly error messages
impl ConfigError {
    pub fn user_message(&self) -> String {
        match self {
            ConfigError::TomlParse(e) => {
                format!(
                    "Invalid TOML syntax in configuration file.\n\
                    Error: {}\n\
                    Hint: Check for missing quotes, brackets, or commas.",
                    e
                )
            }
            ConfigError::Validation(msg) => {
                format!(
                    "Configuration validation failed: {}\n\
                    Run 'kaiak doctor config' to diagnose configuration issues.",
                    msg
                )
            }
            ConfigError::FileNotFound { path } => {
                format!(
                    "Configuration file not found: {}\n\
                    Run 'kaiak init' to create a default configuration file.",
                    path
                )
            }
            _ => format!("{}", self),
        }
    }
}
```

### 3.2 Validation Error Reporting

```rust
use validator::ValidationErrors;

pub fn format_validation_errors(errors: &ValidationErrors) -> String {
    let mut messages = Vec::new();

    for (field, field_errors) in errors.field_errors() {
        for error in field_errors {
            let message = error.message
                .as_ref()
                .map(|m| m.to_string())
                .unwrap_or_else(|| format!("Invalid value for field '{}'", field));

            messages.push(format!("  • {}: {}", field, message));
        }
    }

    format!(
        "Configuration validation failed:\n{}\n\n\
        Review your configuration file and ensure all required fields are present and valid.",
        messages.join("\n")
    )
}
```

### 3.3 Helpful CLI Error Output

```rust
pub fn handle_config_error(error: ConfigError, config_path: &str) -> anyhow::Error {
    eprintln!("❌ Configuration Error\n");
    eprintln!("{}\n", error.user_message());
    eprintln!("Configuration file: {}", config_path);

    // Provide context-specific help
    match &error {
        ConfigError::TomlParse(_) => {
            eprintln!("\n💡 Tips:");
            eprintln!("  • Validate TOML syntax online: https://www.toml-lint.com/");
            eprintln!("  • Check example config: kaiak config example");
        }
        ConfigError::FileNotFound { .. } => {
            eprintln!("\n💡 Quick fix:");
            eprintln!("  kaiak init --config {}", config_path);
        }
        _ => {}
    }

    anyhow::anyhow!(error)
}
```

## 4. Schema Validation Approach

### 4.1 Runtime Validation Strategy

**Recommendation**: Use `validator` crate (already in use) for runtime validation.

```rust
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct ConfigurationInput {
    #[validate(nested)]
    pub workspace: WorkspaceConfig,

    #[validate(nested)]
    pub model: ModelConfig,

    #[validate(nested)]
    pub tools: ToolConfig,
}

impl ConfigurationInput {
    /// Load from TOML, validate, convert to JSON
    pub fn from_toml_validated(path: impl AsRef<Path>) -> Result<serde_json::Value> {
        // 1. Load TOML
        let content = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;

        // 2. Validate structure
        config.validate()
            .map_err(|e| anyhow::anyhow!(format_validation_errors(&e)))?;

        // 3. Convert to JSON
        let json_value = serde_json::to_value(&config)?;

        Ok(json_value)
    }
}
```

### 4.2 Validation Phases

**Phase 1: Syntax Validation** (TOML parser)
```rust
// Handled by toml::from_str()
let toml_value: toml::Value = toml::from_str(&content)?;
// Fails on invalid TOML syntax
```

**Phase 2: Structural Validation** (validator crate)
```rust
// Handled by validator::Validate
config.validate()?;
// Fails on constraint violations (length, range, custom validators)
```

**Phase 3: Semantic Validation** (custom logic)
```rust
// Business logic validation
impl AgentConfiguration {
    pub fn semantic_validate(&self) -> Result<()> {
        // Check workspace exists
        if !self.workspace.working_dir.exists() {
            anyhow::bail!("Workspace directory does not exist: {:?}",
                self.workspace.working_dir);
        }

        // Check model provider is supported
        if let Some(provider) = self.model.get("provider") {
            match provider.as_str() {
                Some("databricks") | Some("openai") | Some("anthropic") => {}
                _ => anyhow::bail!("Unsupported model provider: {:?}", provider),
            }
        }

        Ok(())
    }
}
```

### 4.3 JSON Schema Generation (Future Enhancement)

```rust
use schemars::JsonSchema;

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct AgentConfiguration {
    // ... fields
}

pub fn generate_json_schema() -> serde_json::Value {
    schema_for!(AgentConfiguration)
}

// CLI command: kaiak schema export > config-schema.json
```

## 5. Integration with Existing Configuration System

### 5.1 Unified Configuration Flow

```
┌─────────────────────────────────────────────────────────────┐
│                    Configuration Sources                     │
├─────────────────┬──────────────────┬────────────────────────┤
│  TOML File      │  CLI Arguments   │  Environment Variables │
│  ~/.kaiak/      │  --model=...     │  KAIAK_MODEL=...      │
│  config.toml    │  --workspace=... │  OPENAI_API_KEY=...   │
└────────┬────────┴─────────┬────────┴───────────┬────────────┘
         │                  │                    │
         ▼                  ▼                    ▼
    ┌────────────────────────────────────────────────┐
    │          Configuration Loader                  │
    │  (Precedence: CLI > Env > TOML > Defaults)    │
    └────────────────────┬───────────────────────────┘
                         │
                         ▼
    ┌────────────────────────────────────────────────┐
    │          Validator (validator crate)           │
    │  • Structural validation                       │
    │  • Business rules                              │
    │  • Semantic checks                             │
    └────────────────────┬───────────────────────────┘
                         │
                         ▼
    ┌────────────────────────────────────────────────┐
    │       Unified Configuration Object             │
    │  (Single source of truth in memory)            │
    └────────────────────┬───────────────────────────┘
                         │
         ┌───────────────┴───────────────┐
         ▼                               ▼
┌──────────────────┐           ┌──────────────────┐
│  Server Config   │           │  Agent Config    │
│  (ServerSettings)│           │  (AgentConfig)   │
└────────┬─────────┘           └────────┬─────────┘
         │                               │
         ▼                               ▼
   Used Locally                   Converted to JSON
   by Server                      for JSON-RPC
```

### 5.2 Implementation Plan

#### Step 1: Create Unified Configuration Module

**File**: `src/config/unified.rs`

```rust
use serde::{Deserialize, Serialize};
use validator::Validate;
use anyhow::Result;
use std::path::{Path, PathBuf};

/// Unified configuration that supports both TOML and JSON
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct UnifiedConfig {
    #[validate(nested)]
    pub server: ServerConfig,

    #[validate(nested)]
    pub agent: AgentConfigDefaults,
}

impl UnifiedConfig {
    /// Load configuration with precedence: CLI > Env > File > Defaults
    pub fn load(
        cli_overrides: Option<ConfigOverrides>,
        config_path: Option<PathBuf>,
    ) -> Result<Self> {
        // 1. Start with defaults
        let mut config = Self::default();

        // 2. Apply config file if exists
        if let Some(path) = config_path.or_else(Self::default_config_path) {
            if path.exists() {
                config = Self::from_toml_file(&path)?;
            }
        }

        // 3. Apply environment variables
        config.apply_env_overrides();

        // 4. Apply CLI arguments
        if let Some(overrides) = cli_overrides {
            config.apply_cli_overrides(overrides);
        }

        // 5. Validate final configuration
        config.validate()?;

        Ok(config)
    }

    /// Load from TOML file
    pub fn from_toml_file(path: impl AsRef<Path>) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref())?;
        let config: Self = toml::from_str(&content)?;
        Ok(config)
    }

    /// Convert agent config portion to JSON for JSON-RPC
    pub fn agent_config_to_json(&self) -> Result<serde_json::Value> {
        serde_json::to_value(&self.agent)
            .map_err(Into::into)
    }

    fn default_config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join("kaiak").join("config.toml"))
    }

    fn apply_env_overrides(&mut self) {
        if let Ok(val) = std::env::var("KAIAK_LOG_LEVEL") {
            self.server.log_level = val;
        }
        // ... other env vars
    }

    fn apply_cli_overrides(&mut self, overrides: ConfigOverrides) {
        if let Some(transport) = overrides.transport {
            self.server.transport = transport;
        }
        // ... other CLI overrides
    }
}

#[derive(Debug, Default)]
pub struct ConfigOverrides {
    pub transport: Option<String>,
    pub socket_path: Option<String>,
    pub log_level: Option<String>,
    // ... other CLI-overridable options
}
```

#### Step 2: Update Server Initialization

**File**: `src/main.rs` (updated)

```rust
use kaiak::config::UnifiedConfig;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Load unified configuration
    let config = UnifiedConfig::load(
        cli.config_overrides(),
        cli.config_path,
    )?;

    // Validate configuration
    config.validate()
        .context("Configuration validation failed")?;

    // Initialize server with validated config
    let server = Server::new(config.server.clone()).await?;

    // Store agent config for JSON-RPC conversions
    server.set_default_agent_config(config.agent)?;

    server.run().await
}
```

#### Step 3: Client TOML to JSON Conversion

**File**: `src/cli/commands/client_ops.rs` (new)

```rust
use anyhow::{Context, Result};
use std::path::PathBuf;

pub async fn execute_configure(
    socket_path: &str,
    input_source: InputSource,
) -> Result<()> {
    // Load and convert configuration
    let json_value = match input_source {
        InputSource::TomlFile(path) => {
            AgentConfiguration::from_toml_file(&path)
                .context("Failed to load TOML configuration")?
                .to_json_value()
                .context("Failed to convert to JSON")?
        }
        InputSource::JsonFile(path) => {
            let content = std::fs::read_to_string(&path)?;
            serde_json::from_str(&content)?
        }
        InputSource::InlineJson(json_str) => {
            serde_json::from_str(&json_str)?
        }
    };

    // Send to server via JSON-RPC
    let client = JsonRpcClient::connect(socket_path).await?;
    let response = client.call("kaiak/configure", json_value).await?;

    println!("Configuration applied: {}",
        serde_json::to_string_pretty(&response)?);

    Ok(())
}

pub enum InputSource {
    TomlFile(PathBuf),
    JsonFile(PathBuf),
    InlineJson(String),
}
```

### 5.3 Example TOML Configuration Files

#### Server Configuration (`~/.kaiak/server.conf`)

```toml
[server]
transport = "socket"
socket_path = "/tmp/kaiak.sock"
log_level = "info"
max_concurrent_sessions = 10

[ai]
timeout = 300
max_turns = 50

[workspace]
exclude_patterns = [
    "target/",
    "node_modules/",
    ".git/",
    "*.tmp"
]
max_file_size = 1048576  # 1MB

[security]
require_approval = true
approval_timeout = 300

[performance]
stream_buffer_size = 1000
session_cache_size = 100
```

#### Agent Configuration (`~/.kaiak/agent.toml`)

```toml
[workspace]
working_dir = "/path/to/project"
include_patterns = ["**/*.rs", "**/*.toml"]
exclude_patterns = [".git/**", "target/**"]

[model]
provider = "databricks"
model = "databricks-meta-llama-3-1-405b-instruct"
temperature = 0.1
max_tokens = 4096

[tools]
enabled_extensions = ["developer", "todo", "extensionmanager"]
planning_mode = false
max_tool_calls = 10

[session]
id = "550e8400-e29b-41d4-a716-446655440000"
max_turns = 1000

[permissions.tool_permissions]
read_file = "allow"
write_file = "approve"
shell_command = "deny"
web_search = "allow"
```

### 5.4 CLI Interface

```bash
# Server commands
kaiak serve --config ~/.kaiak/server.conf
kaiak serve --socket /tmp/kaiak.sock

# Client commands with TOML input
kaiak configure --input ~/.kaiak/agent.toml
kaiak configure --input config.toml --validate-only

# Client commands with JSON input (also supported)
kaiak configure --input-json '{"model": {"provider": "openai"}}'
kaiak configure --input config.json

# Utility commands
kaiak config validate server.conf          # Validate TOML syntax
kaiak config convert agent.toml agent.json # Convert TOML to JSON
kaiak config example > default.toml        # Generate example config
```

## 6. Testing Strategy

### 6.1 Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_toml_to_json_conversion() {
        let toml_content = r#"
            [workspace]
            working_dir = "/tmp/test"
            include_patterns = ["**/*"]
            exclude_patterns = [".git/**"]

            [model]
            provider = "databricks"
            model = "test-model"
        "#;

        let config: AgentConfiguration = toml::from_str(toml_content)
            .expect("Failed to parse TOML");

        let json_value = serde_json::to_value(&config)
            .expect("Failed to convert to JSON");

        assert_eq!(json_value["workspace"]["working_dir"], "/tmp/test");
        assert_eq!(json_value["model"]["provider"], "databricks");
    }

    #[test]
    fn test_validation_errors() {
        let invalid_toml = r#"
            [workspace]
            working_dir = ""  # Invalid: empty path
            include_patterns = []  # Invalid: at least one required
        "#;

        let result: Result<AgentConfiguration, _> = toml::from_str(invalid_toml);
        assert!(result.is_ok()); // TOML parsing succeeds

        let config = result.unwrap();
        let validation_result = config.validate();
        assert!(validation_result.is_err()); // Validation fails
    }

    #[test]
    fn test_malformed_toml() {
        let malformed = r#"
            [workspace
            working_dir = "/tmp"  # Missing closing bracket
        "#;

        let result: Result<AgentConfiguration, _> = toml::from_str(malformed);
        assert!(result.is_err());

        let error = result.unwrap_err();
        assert!(error.to_string().contains("expected"));
    }
}
```

### 6.2 Integration Tests

```rust
// tests/test_config_conversion.rs

#[tokio::test]
async fn test_end_to_end_toml_to_jsonrpc() {
    // 1. Create TOML config file
    let temp_dir = tempfile::tempdir().unwrap();
    let config_path = temp_dir.path().join("test.toml");

    std::fs::write(&config_path, r#"
        [workspace]
        working_dir = "/tmp/test"
        include_patterns = ["**/*.rs"]
        exclude_patterns = [".git/**"]

        [model]
        provider = "databricks"
        model = "test-model"

        [session]
        id = "test-session-123"
    "#).unwrap();

    // 2. Load and validate
    let config = AgentConfiguration::from_toml_file(&config_path)
        .expect("Failed to load TOML");

    // 3. Convert to JSON
    let json_value = config.to_json_value().expect("Failed to convert to JSON");

    // 4. Verify JSON structure matches JSON-RPC expectations
    assert!(json_value.get("workspace").is_some());
    assert!(json_value.get("model").is_some());
    assert!(json_value.get("session").is_some());

    // 5. Verify it can be sent via JSON-RPC (mock client)
    let json_string = serde_json::to_string(&json_value).unwrap();
    let parsed: ConfigureRequest = serde_json::from_str(&format!(
        r#"{{"configuration": {}}}"#, json_string
    )).expect("Failed to parse as ConfigureRequest");

    assert_eq!(parsed.configuration.session.id, "test-session-123");
}
```

### 6.3 Error Handling Tests

```rust
#[test]
fn test_helpful_error_messages() {
    let invalid_toml = r#"
        [workspace]
        working_dir = ""
        include_patterns = []
    "#;

    let config: AgentConfiguration = toml::from_str(invalid_toml).unwrap();
    let errors = config.validate().unwrap_err();

    let formatted = format_validation_errors(&errors);

    // Verify error messages are helpful
    assert!(formatted.contains("include_patterns"));
    assert!(formatted.contains("at least one"));
}

#[test]
fn test_toml_syntax_error_reporting() {
    let malformed = r#"
        [workspace
        working_dir = "/tmp"
    "#;

    let error = toml::from_str::<AgentConfiguration>(malformed).unwrap_err();

    // Verify error includes line/column information
    let error_str = error.to_string();
    assert!(error_str.contains("expected") || error_str.contains("syntax"));
}
```

## 7. Best Practices Summary

### 7.1 DO

✅ **Use unified struct definitions** with `#[derive(Serialize, Deserialize)]`
✅ **Validate after parsing** using `validator` crate
✅ **Provide helpful error messages** with suggestions for fixes
✅ **Support multiple input formats** (TOML for files, JSON for inline)
✅ **Test conversion roundtrips** (TOML → struct → JSON → struct)
✅ **Document configuration schemas** with examples and comments
✅ **Use strong typing** - avoid `HashMap<String, Value>` when possible
✅ **Fail fast with clear errors** - don't silently ignore invalid config

### 7.2 DON'T

❌ **Don't use `toml_edit`** unless you need format-preserving edits
❌ **Don't manually map fields** - let Serde handle it
❌ **Don't skip validation** - always validate after deserialization
❌ **Don't expose internal types** - use clear API boundaries
❌ **Don't mix validation concerns** - separate syntax, structure, semantics
❌ **Don't provide vague errors** - always include context and suggestions
❌ **Don't support conflicting formats** - choose TOML OR JSON per use case

### 7.3 Configuration Design Guidelines

1. **Server configuration**: TOML file (`~/.kaiak/server.conf`)
   - Human-editable
   - Comments and documentation
   - Loaded once at server startup

2. **Agent configuration**: Support both
   - TOML file for templates/presets
   - JSON for programmatic/IDE integration
   - Convert TOML → JSON before sending to server

3. **Precedence order**: CLI > Environment > File > Defaults
   - Clear, predictable behavior
   - Document in user guide

4. **Validation phases**:
   - Syntax (parser)
   - Structure (validator crate)
   - Semantics (business logic)

## 8. Implementation Roadmap

### Phase 1: Core Conversion (MVP)
- [ ] Add TOML conversion utilities to `src/config/mod.rs`
- [ ] Update `AgentConfiguration` with `from_toml_file()` method
- [ ] Add comprehensive error types and messages
- [ ] Write unit tests for conversion logic

### Phase 2: Client Integration
- [ ] Add CLI option `--input <file.toml>` to client commands
- [ ] Implement automatic format detection (`.toml` vs `.json`)
- [ ] Add validation before sending to server
- [ ] Write integration tests for client workflow

### Phase 3: Server Configuration
- [ ] Unify `ServerSettings` with TOML loading
- [ ] Implement configuration precedence (CLI > Env > File)
- [ ] Add `kaiak config` utility commands
- [ ] Document configuration options

### Phase 4: Enhanced Features
- [ ] JSON Schema generation for documentation
- [ ] Configuration validation CLI command
- [ ] Example configuration templates
- [ ] Shell completion for config options

## 9. References

### Documentation
- [Rust - Converting between file formats - JSON, YAML, & TOML](https://tarquin-the-brave.github.io/blog/posts/rust-serde/)
- [Structured Data - Rust Cookbook](https://rust-lang-nursery.github.io/rust-cookbook/encoding/complex.html)
- [Serde JSON Guide [2025]](https://generalistprogrammer.com/tutorials/serde_json-rust-crate-guide)
- [TOML_edit Rust Guide [2025]](https://generalistprogrammer.com/tutorials/toml_edit-rust-crate-guide)

### Crates
- [toml - Rust](https://docs.rs/toml) - Current: v0.8.23
- [validator - Rust](https://docs.rs/validator) - Current: v0.18.1
- [jsonschema - Rust](https://docs.rs/jsonschema) - High-performance JSON Schema validator
- [schemars - Rust](https://docs.rs/schemars) - JSON Schema generation from Rust types

### Standards
- [TOML Specification v1.0.0](https://toml.io/en/v1.0.0)
- [JSON-RPC 2.0 Specification](https://www.jsonrpc.org/specification)
- [JSON Schema Draft 2020-12](https://json-schema.org/draft/2020-12/release-notes.html)

---

**Last Updated**: 2025-12-27
**Reviewed By**: Research Analysis for 004-kaiak-client
**Status**: Ready for Implementation
