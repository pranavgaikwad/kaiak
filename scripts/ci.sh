#!/bin/bash
set -euo pipefail

echo "🚀 Running local CI checks..."

echo "📋 Checking formatting..."
cargo fmt --all -- --check

echo "📎 Running clippy..."
cargo clippy --all-targets --all-features -- -D warnings

echo "🧪 Running tests..."
cargo test --all-features --workspace

echo "🔍 Running integration tests..."
cargo test --test integration --all-features

echo "🔒 Security audit..."
if ! command -v cargo-audit &> /dev/null; then
    echo "Installing cargo-audit..."
    cargo install cargo-audit
fi
cargo audit

echo "✅ All CI checks passed!"