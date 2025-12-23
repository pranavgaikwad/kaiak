# Test Recordings Directory

This directory contains recorded model interactions for the TestProvider infrastructure (T014). These recordings enable reliable CI/PR testing without requiring external API keys or network access.

## How It Works

The TestProvider system supports three modes:

1. **Record Mode** (`KAIAK_TEST_MODE=record`): Records real model interactions to files
2. **Replay Mode** (`KAIAK_TEST_MODE=replay`): Replays recorded interactions from files
3. **Live Mode** (`KAIAK_TEST_MODE=live`): Passes through to real model (development only)

## Usage

### Recording New Interactions (Development Only)

```bash
# Record interactions for a specific test
KAIAK_TEST_MODE=record cargo test test_agent_integration_end_to_end

# Record all integration tests
KAIAK_RECORD_TESTS=1 cargo test --test goose_integration
```

### Replaying Recorded Interactions (CI/PR)

```bash
# Replay mode (default in CI)
KAIAK_TEST_MODE=replay cargo test

# CI automatically uses replay mode
cargo test  # In CI environments
```

### Live Testing (Development Only)

```bash
# Use real model interactions (requires API keys)
KAIAK_TEST_MODE=live cargo test test_agent_integration_end_to_end
```

## Recording Files

Each test creates a JSON file with the following naming convention:
- Format: `{test_name}.json`
- Example: `agent_integration_end_to_end.json`

### Recording File Structure

```json
{
  "test_name": "agent_integration_end_to_end",
  "recorded_at": "2025-12-23T10:30:00Z",
  "kaiak_version": "0.1.0",
  "environment": {
    "os": "linux",
    "arch": "x86_64",
    "rust_version": "1.75.0",
    "git_commit": "abc123",
    "is_ci": false
  },
  "interactions": [
    {
      "test_name": "agent_integration_end_to_end",
      "timestamp": "2025-12-23T10:30:01Z",
      "request_type": "session_init",
      "input": { "workspace_path": "/tmp/test", "session_id": "test" },
      "output": { "status": "ready", "capabilities": [...] },
      "metadata": {
        "model": "test-model",
        "provider": "test",
        "execution_time_ms": 150,
        "success": true,
        "session_id": "test",
        "request_id": "init-001"
      }
    }
  ]
}
```

## Safety Guards

- **CI Recording Prevention**: Recording mode is automatically blocked in CI environments
- **Environment Detection**: Automatically detects CI environments (GitHub Actions, GitLab CI, etc.)
- **Replay Validation**: Ensures recorded interactions match test expectations
- **Deterministic Timing**: Simulates realistic execution times during replay

## Environment Variables

| Variable | Values | Description |
|----------|--------|-------------|
| `KAIAK_TEST_MODE` | `record`, `replay`, `live` | Explicit mode selection |
| `KAIAK_RECORD_TESTS` | Any value | Enable recording in development |
| `CI` | Any value | Detected CI environment (auto-replay) |
| `GITHUB_ACTIONS` | Any value | GitHub Actions CI detection |

## Best Practices

### For Developers

1. **Record First**: Create recordings in development before pushing
2. **Review Changes**: Check recording diffs when model behavior changes
3. **Update Regularly**: Re-record when adding new test scenarios
4. **Test Modes**: Verify tests work in both record and replay modes

### For CI/PR

1. **Replay Only**: CI automatically uses replay mode for reliability
2. **No API Keys**: Tests run without requiring external API access
3. **Deterministic**: Identical results across different environments
4. **Fast Execution**: No network delays, consistent timing

### Recording Management

```bash
# Re-record all tests (use with caution)
find tests/recordings -name "*.json" -delete
KAIAK_TEST_MODE=record cargo test --test goose_integration

# Re-record specific test
rm tests/recordings/agent_integration_end_to_end.json
KAIAK_TEST_MODE=record cargo test test_agent_integration_end_to_end

# Validate recordings
KAIAK_TEST_MODE=replay cargo test --test goose_integration
```

## Troubleshooting

### Missing Recording Files

```
Error: Recording file not found for test 'test_name' at path: tests/recordings/test_name.json
```

**Solution**: Record the test first in development:
```bash
KAIAK_TEST_MODE=record cargo test test_name
```

### Replay Mismatch

```
Error: Replay mismatch. Expected request_type 'session_init', got 'fix_generation' at index 0
```

**Solution**: The test logic changed since recording. Re-record:
```bash
rm tests/recordings/test_name.json
KAIAK_TEST_MODE=record cargo test test_name
```

### Recording in CI

```
Error: Recording mode is not allowed in CI environment
```

**Solution**: This is intentional. CI should only use replay mode for reliability.

## Success Criteria

- **95% Test Success Rate** (SC-003): Achieved through deterministic replay
- **No API Dependencies**: CI/PR tests run without external services
- **Comprehensive Coverage**: All model interaction scenarios recorded
- **Safety Compliance**: Recording prevented in CI environments

## Files in This Directory

- `README.md` - This documentation
- `*.json` - Individual test recordings (created by running tests in record mode)
- `.gitignore` - Excludes temporary files while including recordings

Note: Recording files are checked into version control to ensure CI/PR tests have access to them.