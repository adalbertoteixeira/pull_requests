# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a Rust-based Git pull request automation tool designed for JavaScript repositories. It helps developers create consistent commit messages, extract metadata from branch names, analyze changed files to suggest commit types, and generate PR templates.

## Common Development Commands

### Build and Run
- **Development build**: `cargo build`
- **Release build**: `cargo build --release`
- **Run the tool**: `cargo run -- commit` or `cargo run -- ticket`
- **Install locally**: Copy `./target/release/pull_requests` to your target repository

### Code Quality
- **Format code**: `cargo fmt`
- **Lint**: `cargo clippy`
- **Type check**: `cargo check`

### Testing
Currently no tests exist (see TODO in README.md). When implementing tests, use standard Rust testing conventions with `cargo test`.

## Architecture Overview

The codebase follows a modular design with clear separation of concerns:

1. **Entry Point Flow**: `main.rs` → dispatches to either `commit` or `ticket` subcommands
2. **Commit Workflow**: 
   - `branch_utils.rs` extracts branch metadata and analyzes changed files
   - `prompts.rs` handles all user interactions
   - `commit.rs` orchestrates the commit process
   - `storage.rs` persists configurations and failed commits
3. **External Integration**: `ticket.rs` handles ClickUp API integration

### Key Architectural Decisions

- **Interactive CLI**: Uses `inquire` crate for rich terminal prompts with editor support
- **File-based State**: YAML files store configuration in `~/.config/commit_tool.yaml` and `.pull_requests/` directory
- **Git Integration**: Direct shell command execution for git operations
- **Retry Mechanism**: Failed commits are saved and automatically suggested on next run
- **Regex-based Analysis**: Uses compiled regexes to categorize files and extract branch metadata

### Important Patterns

- **Branch Name Format**: Expects branches like `feature/ISSUE-123-description` to extract issue ID
- **Commit Type Detection**: Analyzes changed files to suggest types (feat, fix, test, docs, build, ci)
- **PR Template Generation**: Builds risk assessment and testing instructions based on file changes

## Development Notes

- The tool is designed for JavaScript repositories but core logic is language-agnostic
- ClickUp integration requires async/await (only async code in the project)
- All git operations use shell commands via `std::process::Command`
- Error handling uses `Result` types throughout
- Configuration files use YAML format for human readability