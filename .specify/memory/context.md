# Project Context: AI-Driven Code Migration with Kaiak

## Overview

This document provides a comprehensive understanding of the architectural evolution from a TypeScript LangGraph-based AI system to a Rust-powered Goose agent system, with Kaiak serving as the critical JSON-RPC bridge between these worlds.

## The Current State: Editor Extensions with LangGraph

### What We Have Today

The **[editor-extensions](https://github.com/konveyor/editor-extensions)** project is a sophisticated VSCode extension ecosystem designed for application migration and modernization. It consists of:

#### Core Architecture
- **[Core Extension](https://github.com/konveyor/editor-extensions/tree/main/vscode/core)** (`vscode/`): Orchestrates the entire migration workflow
- **Language-Specific Extensions**: Specialized analyzers for Java, TypeScript, Go, and other languages
- **[Webview UI](https://github.com/konveyor/editor-extensions/tree/main/webview-ui)** (`webview-ui/`): React-based user interface using PatternFly
- **[Agentic Module](https://github.com/konveyor/editor-extensions/tree/main/agentic)** (`agentic/`): Current AI-powered fix generation system using LangGraph

#### Current Workflow
1. **Analysis Phase**: Language-specific extensions perform static code analysis
2. **Issue Detection**: Kai analyzer RPC server identifies migration issues
3. **AI-Powered Fixing**: LangGraph-based agentic workflows generate and apply fixes
4. **User Interaction**: WebView UI presents results and allows user oversight

## The Vision: Goose-Powered Intelligence


Integrating **Goose** will make it so that we don't have to maintain our own AI agent. That's why we created _Kaiak_ which exposes Goose over JSON-RPC:

### Why Kaiak Exists

Since the VSCode extensions are written in TypeScript and Goose is written in Rust, we need a language-agnostic communication layer. Kaiak serves this critical role by:

#### JSON-RPC Interface Design
```rust
// Example API structure
KaiakRequest<GenerateFixRequest> → Goose Agent → KaiakResponse<FixSuggestion>
```

#### Core Capabilities
- **Type-Safe Communication**: Rust's type system ensures API contract integrity
- **Session Management**: Persistent agent sessions across multiple fix attempts
- **Flexible Transport**: Supports stdio and Unix domain sockets
- **Metadata Tracking**: Comprehensive request/response lifecycle monitoring

#### Current API Methods
- `kaiak/generate_fix`: Primary method for AI-powered code fixes
- `kaiak/delete_session`: Session lifecycle management
- *Extensible for future methods as needed*

### Integration Architecture

```
┌─────────────────────┐    JSON-RPC     ┌──────────────┐    Direct API    ┌─────────────┐
│   VSCode Extension  │ ◄──────────────► │    Kaiak     │ ◄───────────────► │    Goose    │
│   (TypeScript)      │                  │  JSON-RPC    │                   │   Agent     │
│                     │                  │   Server     │                   │  (Rust)     │
└─────────────────────┘                  └──────────────┘                   └─────────────┘
```

## Conclusion

Kaiak represents a crucial architectural evolution that will:

1. **Modernize the AI Stack**: Replace Python/JavaScript complexity with performant Rust
2. **Improve User Experience**: Faster, more reliable AI-powered code fixes
3. **Enable Future Innovation**: Access to Goose's advanced agent capabilities
4. **Maintain Compatibility**: Seamless integration with existing VSCode extension ecosystem

This transition positions the editor-extensions project to leverage cutting-edge AI agent technology while maintaining the familiar, productive user experience developers expect from their migration tools.