# Workflows

## UCI Command Processing

### Position Command
```mermaid
sequenceDiagram
    UCI→>Board: from_fen(fen)
    Note over Board: Parse piece placement
    Note over Board: Parse castling rights
    Note over Board: Parse en passant
    UCI→>Board: make_move() for each move
    Note over Board: Update state, store undo info
```

### Go Command
```mermaid
sequenceDiagram
    UCI→>Search: iterative_deepening(params)
    loop Depth Iteration
        Search→>MoveGen: generate_legal_moves()
        MoveGen-->>Search: moves
        Search→>Search: order_moves()
        loop Alpha-Beta
            Search→>Board: make_move()
            Search→>Eval: evaluate()
            Eval→>Personality: personality_score()
            Search→>Board: unmake_move()
        end
    end
    Search-->>UCI: bestmove
```

## Move Generation Pipeline

```mermaid
graph LR
    A[Board State] --> B[generate_pseudo_legal_moves]
    B --> C[Filter illegal moves]
    C --> D[is_in_check test]
    D --> E[Legal Moves]
```

1. Generate pseudo-legal moves from piece positions
2. Filter captures that leave king in check
3. Filter quiet moves that leave king in check
4. Add special moves (castling, en passant, promotions)

## Evaluation Pipeline

```mermaid
graph TB
    A[Board] --> B[material_balance]
    A --> C[piece_mobility]
    A --> D[pawn_structure]
    A --> E[king_safety]
    A --> F[piece_square_score]
    B --> G[Sum]
    C --> G
    D --> G
    E --> G
    F --> G
    G --> H[personality_score]
    H --> I[Final Score]
```

## Search Flow

```mermaid
graph TD
    A[Root] --> B{Depth > 0?}
    B -->|No| C[Quiescence]
    B -->|Yes| D[Generate moves]
    D --> E[Order moves]
    E --> F[Loop moves]
    F --> G[Make move]
    G --> H[Recursive search]
    H --> I{Beta cutoff?}
    I -->|Yes| J[Store in TT]
    I -->|No| K[Update best]
    G --> L[Unmake move]
    F --> M[Store TT entry]
    C --> N[Evaluate]
    N --> O[Stand pat]
```

## Time Management

```mermaid
graph LR
    A[Start] --> B[Calculate budget]
    B --> C{mode?}
    C --> D[Depth mode]
    C --> E[Movetime mode]
    C --> F[Infinite]
    D --> G[Search]
    E --> G
    F --> G
    G --> H{Time check}
    H -->|OK| I[Continue]
    H -->|Exhausted| J[Stop]
```

## FEN Round-Trip

1. Parse FEN string into board state
2. Generate FEN from board state
3. Compare for equality
4. Used in tests to verify board correctness
