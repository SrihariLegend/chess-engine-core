# Documentation Index

## Overview

This is a **Rust chess engine** with UCI interface, alpha-beta search, bitboard-based board representation, and modular personality/narrative systems.

## File Guide

| File | Purpose | When to Use |
|------|---------|-------------|
| `codebase_info.md` | Basic project info, tech stack | Getting oriented |
| `architecture.md` | System diagrams, component relationships | Understanding design |
| `components.md` | Detailed module documentation | Finding specific code |
| `interfaces.md` | Public API signatures | Using the engine |
| `data_models.md` | Type definitions, structs | Understanding data |
| `workflows.md` | Process flows, sequences | Understanding operations |
| `dependencies.md` | External deps, module graph | Integration work |
| `review_notes.md` | Gaps and inconsistencies | Improving docs |

## Quick Reference

- **Entry point**: `src/main.rs` → `main()`
- **UCI handler**: `src/uci/mod.rs` → `UciHandler::run()`
- **Board**: `src/board/mod.rs` → `Board`
- **Search**: `src/search/mod.rs` → `SearchState::search()`
- **Evaluation**: `src/eval/mod.rs` → `evaluate()`

## Common Tasks

| Task | File |
|------|------|
| Add new UCI command | `src/uci/mod.rs` |
| Modify evaluation | `src/eval/mod.rs` |
| Add personality | `src/personality/` |
| Add narrative theme | `src/narrative/` |
| Fix move generation | `src/movegen/mod.rs` |
| Improve search | `src/search/mod.rs` |

## For AI Assistants

Add this file to context for questions about the chess engine. The other files contain detailed information for specific queries.
