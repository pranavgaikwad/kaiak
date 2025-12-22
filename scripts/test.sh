#!/bin/bash
set -euo pipefail

echo "🧪 Running comprehensive test suite..."

echo "📦 Running unit tests..."
cargo test --lib --all-features

echo "🔧 Running integration tests..."
cargo test --test integration --all-features

echo "📋 Running contract tests..."
cargo test --test contract --all-features

echo "⚡ Running benchmarks..."
cargo test --benches --all-features

echo "📊 Generating coverage report..."
if ! command -v cargo-llvm-cov &> /dev/null; then
    echo "Installing cargo-llvm-cov..."
    cargo install cargo-llvm-cov
fi
cargo llvm-cov --all-features --workspace --html

echo "✅ Test suite completed!"
echo "📄 Coverage report: target/llvm-cov/html/index.html"