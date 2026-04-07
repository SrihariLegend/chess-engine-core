# Contributing

## Development Setup

```bash
cargo build --release
cargo test
```

## Code Style

- Use `cargo fmt` before committing
- Use `cargo clippy` to catch common mistakes

## Testing

Run all tests:
```bash
cargo test
```

Run property-based tests:
```bash
cargo test --test properties
```

## Adding New Features

### Adding a New Personality
1. Create `src/personality/your_name.rs` implementing `Personality` trait
2. Add module declaration in `src/personality/mod.rs`
3. Add weight configuration in search/evaluation

### Adding UCI Options
1. Add field to `UciOptions` in `src/uci/mod.rs`
2. Handle in `handle_setoption()`
3. Wire to relevant component

## Key Conventions

- All move generation must produce legal moves (passes `is_in_check` test)
- Evaluation returns score from perspective of side to move
- Search uses iterative deepening with time management
- Transposition table uses generational replacement
