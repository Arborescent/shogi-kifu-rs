# shogi-kifu-rs

[![Github Actions](https://github.com/Arborescent/shogi-kifu-rs/workflows/build/badge.svg)](https://github.com/Arborescent/shogi-kifu-rs/actions?workflow=build)

A Shogi game record serialization/deserialization library in CSA format.

CSA format is a plaintext format for recording Shogi games. This library supports parsing CSA-formatted strings as well as composing CSA-formatted strings from structs.

## Supported Versions

- CSA V2 ([spec](http://www2.computer-shogi.org/protocol/record_v2.html))
- CSA V2.1 ([spec](http://www2.computer-shogi.org/protocol/record_v21.html))
- CSA V2.2 ([spec](http://www2.computer-shogi.org/protocol/record_v22.html))
- CSA V3.0 ([spec](http://www2.computer-shogi.org/protocol/record_v3.html))

Version is automatically detected from the version line in the input.

## Supported Variants

In addition to standard 9x9 shogi, this library supports smaller board variants:

| Variant | Board Size | Description |
|---------|------------|-------------|
| Standard Shogi | 9x9 | Full shogi with all pieces |
| Minishogi | 5x5 | Simplified variant with King, Rook, Bishop, Gold, Silver, Pawn |
| Wild Cat Shogi | 3x5 | Kids variant with King, Wazir (as HI), Fers (as KA), Gold, Pawn |

### Board Representation

Variants use native grid formats with the appropriate number of cells per row:

```
Standard (9x9):     Minishogi (5x5):    Wild Cat (3x5):
P1 ... (9 cells)    P1 ... (5 cells)    P1 ... (3 cells)
P2 ... (9 cells)    P2 ... (5 cells)    P2 ... (3 cells)
...                 ...                 ...
P9 ... (9 cells)    P5 ... (5 cells)    P5 ... (3 cells)
```

Alternatively, piece placements (P+ and P-) can be used for any variant.

This is a fork of [csa-rs](https://github.com/nozaq/csa-rs) by [nozaq](https://github.com/nozaq).

## Usage

### Parsing CSA to Structs

Parse a CSA-formatted string into a `GameRecord` struct.

```rust
use std::time::Duration;
use csa::{parse_csa, Action, Color, GameRecord, MoveRecord, PieceType, Square};

/// Demonstrates parsing a CSA-formatted game record.
///
/// CSA format structure:
/// - V2.2           : Version declaration
/// - N+NAKAHARA     : Black player name (+ = black/sente)
/// - N-YONENAGA     : White player name (- = white/gote)
/// - $EVENT:...     : Game metadata attributes
/// - PI             : Initial position (PI = default "hirate" starting position)
/// - +              : Side to move first (+ = black)
/// - +2726FU        : Move notation: color + from_square + to_square + piece
/// - T12            : Time consumed for the move (12 seconds)
fn parse_example() {
    let csa_str = "\
V2.2
N+NAKAHARA
N-YONENAGA
$EVENT:13th World Computer Shogi Championship
PI
+
+2726FU
T12
";

    // Parse the CSA string - version is auto-detected
    let game = parse_csa(csa_str).expect("failed to parse CSA content");

    // Access player names
    assert_eq!(game.black_player, Some("NAKAHARA".to_string()));
    assert_eq!(game.white_player, Some("YONENAGA".to_string()));

    // Access game metadata
    assert_eq!(game.event, Some("13th World Computer Shogi Championship".to_string()));

    // Access move records
    // Move: Black moved pawn from 27 (file 2, rank 7) to 26 (file 2, rank 6)
    assert_eq!(game.moves[0], MoveRecord {
        action: Action::Move(
            Color::Black,
            Square::new(2, 7),  // from: file 2, rank 7
            Square::new(2, 6),  // to: file 2, rank 6
            PieceType::Pawn,
        ),
        time: Some(Duration::from_secs(12)),
    });
}
```

### Composing Structs to CSA

Build a `GameRecord` struct and serialize it to CSA format.

```rust
use std::time::Duration;
use csa::{Action, Color, GameRecord, MoveRecord, PieceType, Square};

/// Demonstrates building a game record and serializing to CSA format.
///
/// The resulting CSA string will include:
/// - Version line (V2.2)
/// - Player names
/// - Event metadata
/// - Initial position
/// - Move records with timing
/// - Game-ending action (resignation)
fn compose_example() {
    // Create a new game record with default initial position
    let mut game = GameRecord::default();

    // Set player names
    game.black_player = Some("NAKAHARA".to_string());
    game.white_player = Some("YONENAGA".to_string());

    // Set game metadata
    game.event = Some("13th World Computer Shogi Championship".to_string());

    // Add a move: Black pawn from 27 to 26, took 5 seconds
    game.moves.push(MoveRecord {
        action: Action::Move(
            Color::Black,
            Square::new(2, 7),  // from square
            Square::new(2, 6),  // to square
            PieceType::Pawn,
        ),
        time: Some(Duration::from_secs(5)),
    });

    // Add game-ending action: resignation (TORYO)
    game.moves.push(MoveRecord {
        action: Action::Toryo,
        time: None,
    });

    // Serialize to CSA format
    let csa_output = game.to_string();

    let expected = "\
V2.2
N+NAKAHARA
N-YONENAGA
$EVENT:13th World Computer Shogi Championship
PI
+
+2726FU
T5
%TORYO
";

    assert_eq!(csa_output, expected);
}
```

## License

`shogi-kifu-rs` is licensed under the MIT license. Please read the [LICENSE](LICENSE) file in this repository for more information.
