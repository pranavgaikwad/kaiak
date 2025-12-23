#!/bin/bash
set -euo pipefail

echo "🔍 Running code quality checks..."

echo "📋 Checking formatting..."
cargo fmt --all -- --check

echo "📎 Running clippy with strict lints..."
cargo clippy --all-targets --all-features -- -D warnings

echo "🔒 Security audit..."
if ! command -v cargo-audit &> /dev/null; then
    echo "Installing cargo-audit..."
    cargo install cargo-audit
fi
cargo audit

echo "📊 Checking for unused dependencies..."
if ! command -v cargo-machete &> /dev/null; then
    echo "Installing cargo-machete..."
    cargo install cargo-machete
fi
cargo machete

echo "✅ All quality checks passed!"