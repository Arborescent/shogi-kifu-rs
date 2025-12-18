//! CSA V2.2 format parser

use pest::Parser;
use pest_derive::Parser;
use std::convert::TryFrom;
use std::time::Duration;
use time::{Date as NativeDate, Month, Time as NativeTime};

use crate::value::*;

#[derive(Debug)]
pub struct ParseError(pub String);

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CSA V2.2 parse error: {}", self.0)
    }
}

impl std::error::Error for ParseError {}

#[derive(Parser)]
#[grammar = "parser/csa/v2_2/grammar.pest"]
struct CsaParser;

type Grid = [[Option<(Color, PieceType)>; 9]; 9];
type MinishogiGrid = [[Option<(Color, PieceType)>; 5]; 5];
type WildcatGrid = [[Option<(Color, PieceType)>; 3]; 5];

pub fn parse(input: &str) -> Result<GameRecord, ParseError> {
    let pairs = CsaParser::parse(Rule::game_record, input)
        .map_err(|e| ParseError(e.to_string()))?;

    let mut record = GameRecord::default();

    for pair in pairs {
        if pair.as_rule() == Rule::game_record {
            for inner in pair.into_inner() {
                match inner.as_rule() {
                    Rule::black_player => {
                        record.black_player = parse_player_name(inner);
                    }
                    Rule::white_player => {
                        record.white_player = parse_player_name(inner);
                    }
                    Rule::game_attr => {
                        parse_game_attr(inner, &mut record);
                    }
                    Rule::position => {
                        record.start_pos = parse_position(inner);
                    }
                    Rule::side_to_move => {
                        record.start_pos.side_to_move = parse_side_to_move(inner);
                    }
                    Rule::move_records => {
                        record.moves = parse_move_records(inner);
                    }
                    Rule::final_move => {
                        let action = parse_move_record_action(inner);
                        record.moves.push(MoveRecord { action, time: None });
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(record)
}

fn parse_player_name(pair: pest::iterators::Pair<Rule>) -> Option<String> {
    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::player_name {
            let name = inner.as_str();
            if !name.is_empty() {
                return Some(name.to_string());
            }
        }
    }
    None
}

fn parse_game_attr(pair: pest::iterators::Pair<Rule>, record: &mut GameRecord) {
    let mut key = String::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::attr_key => {
                key = inner.as_str().to_string();
            }
            Rule::attr_value => {
                for value_inner in inner.into_inner() {
                    match value_inner.as_rule() {
                        Rule::datetime => {
                            let time = parse_datetime(value_inner);
                            match key.as_str() {
                                "START_TIME" => record.start_time = time,
                                "END_TIME" => record.end_time = time,
                                _ => {}
                            }
                        }
                        Rule::timelimit => {
                            if key == "TIME_LIMIT" {
                                record.time_limit = Some(parse_timelimit(value_inner));
                            }
                        }
                        Rule::attr_text => {
                            let text = value_inner.as_str().to_string();
                            match key.as_str() {
                                "EVENT" => record.event = Some(text),
                                "SITE" => record.site = Some(text),
                                "OPENING" => record.opening = Some(text),
                                "START_TIME" => {
                                    record.start_time = try_parse_datetime_str(&text);
                                }
                                "END_TIME" => {
                                    record.end_time = try_parse_datetime_str(&text);
                                }
                                "TIME_LIMIT" => {
                                    record.time_limit = try_parse_timelimit_str(&text);
                                }
                                _ => {}
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
}

fn parse_datetime(pair: pest::iterators::Pair<Rule>) -> Option<Time> {
    let mut date_str = None;
    let mut time_str = None;

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::date => date_str = Some(inner.as_str()),
            Rule::time => time_str = Some(inner.as_str()),
            _ => {}
        }
    }

    date_str.and_then(|d| parse_datetime_parts(d, time_str))
}

fn parse_datetime_parts(date_str: &str, time_str: Option<&str>) -> Option<Time> {
    let date_parts: Vec<&str> = date_str.split('/').collect();
    if date_parts.len() != 3 {
        return None;
    }

    let year: i32 = date_parts[0].parse().ok()?;
    let month: u8 = date_parts[1].parse().ok()?;
    let day: u8 = date_parts[2].parse().ok()?;
    let month = Month::try_from(month).ok()?;
    let date = NativeDate::from_calendar_date(year, month, day).ok()?;

    let time = if let Some(time_s) = time_str {
        let time_parts: Vec<&str> = time_s.split(':').collect();
        if time_parts.len() == 3 {
            let hour: u8 = time_parts[0].parse().ok()?;
            let minute: u8 = time_parts[1].parse().ok()?;
            let second: u8 = time_parts[2].parse().ok()?;
            Some(NativeTime::from_hms(hour, minute, second).ok()?)
        } else {
            None
        }
    } else {
        None
    };

    Some(Time { date, time })
}

fn try_parse_datetime_str(s: &str) -> Option<Time> {
    let parts: Vec<&str> = s.split(' ').collect();
    if parts.is_empty() {
        return None;
    }
    parse_datetime_parts(parts[0], parts.get(1).copied())
}

fn parse_timelimit(pair: pest::iterators::Pair<Rule>) -> TimeLimit {
    let mut hours: u64 = 0;
    let mut minutes: u64 = 0;
    let mut byoyomi: u64 = 0;

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::timelimit_hours => hours = inner.as_str().parse().unwrap_or(0),
            Rule::timelimit_minutes => minutes = inner.as_str().parse().unwrap_or(0),
            Rule::timelimit_byoyomi => byoyomi = inner.as_str().parse().unwrap_or(0),
            _ => {}
        }
    }

    TimeLimit {
        main_time: Duration::from_secs(hours * 3600 + minutes * 60),
        byoyomi: Duration::from_secs(byoyomi),
    }
}

fn try_parse_timelimit_str(s: &str) -> Option<TimeLimit> {
    let parts: Vec<&str> = s.split('+').collect();
    if parts.len() != 2 {
        return None;
    }

    let time_parts: Vec<&str> = parts[0].split(':').collect();
    if time_parts.len() != 2 {
        return None;
    }

    let hours: u64 = time_parts[0].parse().ok()?;
    let minutes: u64 = time_parts[1].parse().ok()?;
    let byoyomi: u64 = parts[1].parse().ok()?;

    Some(TimeLimit {
        main_time: Duration::from_secs(hours * 3600 + minutes * 60),
        byoyomi: Duration::from_secs(byoyomi),
    })
}

fn parse_position(pair: pest::iterators::Pair<Rule>) -> Position {
    let mut pos = Position::default();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::handicap => pos.drop_pieces = parse_handicap(inner),
            Rule::grid => pos.bulk = Some(parse_grid(inner)),
            Rule::minishogi_grid => pos.minishogi_bulk = Some(parse_minishogi_grid(inner)),
            Rule::wildcat_grid => pos.wildcat_bulk = Some(parse_wildcat_grid(inner)),
            Rule::piece_placement_lines => pos.add_pieces = parse_piece_placements(inner),
            _ => {}
        }
    }

    pos
}

fn parse_handicap(pair: pest::iterators::Pair<Rule>) -> Vec<(Square, PieceType)> {
    let mut pieces = Vec::new();

    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::handicap_piece {
            let mut square = Square::new(0, 0);
            let mut piece_type = PieceType::Pawn;

            for piece_inner in inner.into_inner() {
                match piece_inner.as_rule() {
                    Rule::square => square = parse_square(piece_inner.as_str()),
                    Rule::piece_type => piece_type = parse_piece_type(piece_inner.as_str()),
                    _ => {}
                }
            }
            pieces.push((square, piece_type));
        }
    }

    pieces
}

fn parse_grid(pair: pest::iterators::Pair<Rule>) -> Grid {
    let mut grid: Grid = [[None; 9]; 9];

    for inner in pair.into_inner() {
        let row_num = match inner.as_rule() {
            Rule::grid_row1 => Some(0),
            Rule::grid_row2 => Some(1),
            Rule::grid_row3 => Some(2),
            Rule::grid_row4 => Some(3),
            Rule::grid_row5 => Some(4),
            Rule::grid_row6 => Some(5),
            Rule::grid_row7 => Some(6),
            Rule::grid_row8 => Some(7),
            Rule::grid_row9 => Some(8),
            _ => None,
        };

        if let Some(row_idx) = row_num {
            let mut col = 0;
            for cell in inner.into_inner() {
                if cell.as_rule() == Rule::grid_cell && col < 9 {
                    grid[row_idx][col] = parse_grid_cell(cell);
                    col += 1;
                }
            }
        }
    }

    grid
}

fn parse_minishogi_grid(pair: pest::iterators::Pair<Rule>) -> MinishogiGrid {
    let mut grid: MinishogiGrid = [[None; 5]; 5];

    for inner in pair.into_inner() {
        let row_num = match inner.as_rule() {
            Rule::mini_row1 => Some(0),
            Rule::mini_row2 => Some(1),
            Rule::mini_row3 => Some(2),
            Rule::mini_row4 => Some(3),
            Rule::mini_row5 => Some(4),
            _ => None,
        };

        if let Some(row_idx) = row_num {
            let mut col = 0;
            for cell in inner.into_inner() {
                if cell.as_rule() == Rule::grid_cell && col < 5 {
                    grid[row_idx][col] = parse_grid_cell(cell);
                    col += 1;
                }
            }
        }
    }

    grid
}

fn parse_wildcat_grid(pair: pest::iterators::Pair<Rule>) -> WildcatGrid {
    let mut grid: WildcatGrid = [[None; 3]; 5];

    for inner in pair.into_inner() {
        let row_num = match inner.as_rule() {
            Rule::wildcat_row1 => Some(0),
            Rule::wildcat_row2 => Some(1),
            Rule::wildcat_row3 => Some(2),
            Rule::wildcat_row4 => Some(3),
            Rule::wildcat_row5 => Some(4),
            _ => None,
        };

        if let Some(row_idx) = row_num {
            let mut col = 0;
            for cell in inner.into_inner() {
                if cell.as_rule() == Rule::grid_cell && col < 3 {
                    grid[row_idx][col] = parse_grid_cell(cell);
                    col += 1;
                }
            }
        }
    }

    grid
}

fn parse_grid_cell(pair: pest::iterators::Pair<Rule>) -> Option<(Color, PieceType)> {
    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::grid_piece => return Some(parse_grid_piece(inner)),
            Rule::grid_empty => return None,
            _ => {}
        }
    }
    None
}

fn parse_grid_piece(pair: pest::iterators::Pair<Rule>) -> (Color, PieceType) {
    let mut color = Color::Black;
    let mut piece = PieceType::Pawn;

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::color => color = parse_color(inner.as_str()),
            Rule::piece_type => piece = parse_piece_type(inner.as_str()),
            _ => {}
        }
    }

    (color, piece)
}

fn parse_piece_placements(pair: pest::iterators::Pair<Rule>) -> Vec<(Color, Square, PieceType)> {
    let mut placements = Vec::new();

    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::piece_placement {
            let mut color = Color::Black;
            let pieces = parse_single_placement(inner, &mut color);
            for (sq, pt) in pieces {
                placements.push((color, sq, pt));
            }
        }
    }

    placements
}

fn parse_single_placement(
    pair: pest::iterators::Pair<Rule>,
    color_out: &mut Color,
) -> Vec<(Square, PieceType)> {
    let mut pieces = Vec::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::color => *color_out = parse_color(inner.as_str()),
            Rule::placement_piece => {
                let mut square = Square::new(0, 0);
                let mut piece_type = PieceType::Pawn;

                for piece_inner in inner.into_inner() {
                    match piece_inner.as_rule() {
                        Rule::square => square = parse_square(piece_inner.as_str()),
                        Rule::piece_type => piece_type = parse_piece_type(piece_inner.as_str()),
                        _ => {}
                    }
                }
                pieces.push((square, piece_type));
            }
            _ => {}
        }
    }

    pieces
}

fn parse_side_to_move(pair: pest::iterators::Pair<Rule>) -> Color {
    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::color {
            return parse_color(inner.as_str());
        }
    }
    Color::Black
}

fn parse_move_records(pair: pest::iterators::Pair<Rule>) -> Vec<MoveRecord> {
    let mut moves = Vec::new();
    let mut pending_action: Option<Action> = None;

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::move_record => {
                if let Some(action) = pending_action.take() {
                    moves.push(MoveRecord { action, time: None });
                }
                pending_action = Some(parse_move_record_action(inner));
            }
            Rule::time_consumed => {
                if let Some(action) = pending_action.take() {
                    let time = parse_time_consumed(inner);
                    moves.push(MoveRecord {
                        action,
                        time: Some(time),
                    });
                }
            }
            _ => {}
        }
    }

    if let Some(action) = pending_action {
        moves.push(MoveRecord { action, time: None });
    }

    moves
}

fn parse_move_record_action(pair: pest::iterators::Pair<Rule>) -> Action {
    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::normal_move => return parse_normal_move(inner),
            Rule::special_move => return parse_special_move(inner.as_str()),
            _ => {}
        }
    }
    Action::Error
}

fn parse_normal_move(pair: pest::iterators::Pair<Rule>) -> Action {
    let mut color = Color::Black;
    let mut from = Square::new(0, 0);
    let mut to = Square::new(0, 0);
    let mut piece = PieceType::Pawn;
    let mut square_count = 0;

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::color => color = parse_color(inner.as_str()),
            Rule::square => {
                if square_count == 0 {
                    from = parse_square(inner.as_str());
                } else {
                    to = parse_square(inner.as_str());
                }
                square_count += 1;
            }
            Rule::piece_type => piece = parse_piece_type(inner.as_str()),
            _ => {}
        }
    }

    Action::Move(color, from, to, piece)
}

fn parse_special_move(s: &str) -> Action {
    if s.contains("TORYO") {
        Action::Toryo
    } else if s.contains("CHUDAN") {
        Action::Chudan
    } else if s.contains("SENNICHITE") {
        Action::Sennichite
    } else if s.contains("TIME_UP") {
        Action::TimeUp
    } else if s.contains("ILLEGAL_MOVE") {
        Action::IllegalMove
    } else if s.contains("+ILLEGAL_ACTION") {
        Action::IllegalAction(Color::Black)
    } else if s.contains("-ILLEGAL_ACTION") {
        Action::IllegalAction(Color::White)
    } else if s.contains("JISHOGI") {
        Action::Jishogi
    } else if s.contains("KACHI") {
        Action::Kachi
    } else if s.contains("HIKIWAKE") {
        Action::Hikiwake
    } else if s.contains("MATTA") {
        Action::Matta
    } else if s.contains("TSUMI") {
        Action::Tsumi
    } else if s.contains("FUZUMI") {
        Action::Fuzumi
    } else {
        Action::Error
    }
}

fn parse_time_consumed(pair: pest::iterators::Pair<Rule>) -> Duration {
    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::seconds_consumed {
            let secs: u64 = inner.as_str().parse().unwrap_or(0);
            return Duration::from_secs(secs);
        }
    }
    Duration::from_secs(0)
}

fn parse_color(s: &str) -> Color {
    match s {
        "+" => Color::Black,
        "-" => Color::White,
        _ => Color::Black,
    }
}

fn parse_square(s: &str) -> Square {
    let chars: Vec<char> = s.chars().collect();
    let file = chars[0].to_digit(10).unwrap_or(0) as u8;
    let rank = chars[1].to_digit(10).unwrap_or(0) as u8;
    Square::new(file, rank)
}

fn parse_piece_type(s: &str) -> PieceType {
    match s {
        "FU" => PieceType::Pawn,
        "KY" => PieceType::Lance,
        "KE" => PieceType::Knight,
        "GI" => PieceType::Silver,
        "KI" => PieceType::Gold,
        "KA" => PieceType::Bishop,
        "HI" => PieceType::Rook,
        "OU" => PieceType::King,
        "TO" => PieceType::ProPawn,
        "NY" => PieceType::ProLance,
        "NK" => PieceType::ProKnight,
        "NG" => PieceType::ProSilver,
        "UM" => PieceType::Horse,
        "RY" => PieceType::Dragon,
        "AL" => PieceType::All,
        _ => PieceType::Pawn,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple() {
        let csa = "V2.2\nPI\n+\n+2726FU\n";
        let result = parse(csa);
        assert!(result.is_ok(), "Failed: {:?}", result);
    }

    #[test]
    fn test_parse_with_metadata() {
        let csa = concat!(
            "V2.2\n",
            "N+NAKAHARA\n",
            "N-YONENAGA\n",
            "$EVENT:Test\n",
            "PI\n",
            "+\n",
            "+2726FU\n",
            "T12\n",
            "%TORYO\n",
        );
        let result = parse(csa);
        assert!(result.is_ok(), "Failed: {:?}", result);

        let record = result.unwrap();
        assert_eq!(record.black_player, Some("NAKAHARA".to_string()));
        assert_eq!(record.white_player, Some("YONENAGA".to_string()));
        assert_eq!(record.moves.len(), 2);
    }

    /// Test minishogi-style position using piece placements.
    ///
    /// Minishogi is a 5x5 variant. Since the grid format is 9x9 only,
    /// we use PI (to clear the standard position) with piece placements
    /// to set up the minishogi starting position:
    ///
    /// ```text
    ///    5   4   3   2   1
    /// 1 -OU -KI -GI -KA -HI  (white back rank)
    /// 2  .   .   .   .  -FU  (white pawn)
    /// 3  .   .   .   .   .
    /// 4 +FU  .   .   .   .   (black pawn)
    /// 5 +HI +KA +GI +KI +OU  (black back rank)
    /// ```
    #[test]
    fn test_minishogi_piece_placements() {
        let csa = concat!(
            "V2.2\n",
            "N+Sente\n",
            "N-Gote\n",
            "$EVENT:Minishogi Game\n",
            "PI\n",
            "P-51OU41KI31GI21KA11HI12FU\n",
            "P+55HI45KA35GI25KI15OU54FU\n",
            "+\n",
            "+5453FU\n",
            "T5\n",
            "-1213FU\n",
            "T3\n",
            "%TORYO\n",
        );

        let result = parse(csa);
        assert!(result.is_ok(), "Failed to parse minishogi: {:?}", result);

        let record = result.unwrap();
        assert_eq!(record.black_player, Some("Sente".to_string()));
        assert_eq!(record.white_player, Some("Gote".to_string()));
        assert_eq!(record.event, Some("Minishogi Game".to_string()));

        // Check piece placements were parsed
        // White pieces: King(5,1), Gold(4,1), Silver(3,1), Bishop(2,1), Rook(1,1), Pawn(1,2)
        // Black pieces: Rook(5,5), Bishop(4,5), Silver(3,5), Gold(2,5), King(1,5), Pawn(5,4)
        assert_eq!(record.start_pos.add_pieces.len(), 12);

        // Verify some specific pieces
        assert!(record.start_pos.add_pieces.contains(&(
            Color::White,
            Square::new(5, 1),
            PieceType::King
        )));
        assert!(record.start_pos.add_pieces.contains(&(
            Color::Black,
            Square::new(1, 5),
            PieceType::King
        )));
        assert!(record.start_pos.add_pieces.contains(&(
            Color::Black,
            Square::new(5, 4),
            PieceType::Pawn
        )));

        // Check moves
        assert_eq!(record.moves.len(), 3);

        // First move: Black pawn 54 -> 53
        assert_eq!(
            record.moves[0].action,
            Action::Move(Color::Black, Square::new(5, 4), Square::new(5, 3), PieceType::Pawn)
        );
        assert_eq!(record.moves[0].time, Some(Duration::from_secs(5)));

        // Second move: White pawn 12 -> 13
        assert_eq!(
            record.moves[1].action,
            Action::Move(Color::White, Square::new(1, 2), Square::new(1, 3), PieceType::Pawn)
        );

        // Third: resignation
        assert_eq!(record.moves[2].action, Action::Toryo);
    }

    /// Test minishogi with drops (pieces captured and dropped back).
    #[test]
    fn test_minishogi_with_drops() {
        let csa = concat!(
            "V2.2\n",
            "$EVENT:Minishogi Drop Test\n",
            "PI\n",
            "P-51OU41KI31GI21KA11HI12FU\n",
            "P+55HI45KA35GI25KI15OU54FU\n",
            "+\n",
            "+5453FU\n",
            "-1213FU\n",
            "+5352FU\n",
            "-0053FU\n",
            "%TORYO\n",
        );

        let result = parse(csa);
        assert!(result.is_ok(), "Failed to parse minishogi with drops: {:?}", result);

        let record = result.unwrap();

        // Check drop move (from square 00 means drop)
        // Move 4: White drops pawn at 53
        assert_eq!(
            record.moves[3].action,
            Action::Move(Color::White, Square::new(0, 0), Square::new(5, 3), PieceType::Pawn)
        );
    }

    /// Test minishogi with native 5x5 grid format.
    ///
    /// Minishogi starting position:
    /// ```text
    ///    5   4   3   2   1
    /// 1 -HI -KA -GI -KI -OU  (white back rank)
    /// 2  .   .   .   .  -FU  (white pawn)
    /// 3  .   .   .   .   .   (empty)
    /// 4 +FU  .   .   .   .   (black pawn)
    /// 5 +OU +KI +GI +KA +HI  (black back rank)
    /// ```
    #[test]
    fn test_minishogi_grid_format() {
        let csa = concat!(
            "V2.2\n",
            "N+Sente\n",
            "N-Gote\n",
            "$EVENT:Minishogi Grid Test\n",
            "P1-HI-KA-GI-KI-OU\n",
            "P2 *  *  *  * -FU\n",
            "P3 *  *  *  *  * \n",
            "P4+FU *  *  *  * \n",
            "P5+OU+KI+GI+KA+HI\n",
            "+\n",
            "+5453FU\n",
            "T5\n",
            "-1213FU\n",
            "T3\n",
            "%TORYO\n",
        );

        let result = parse(csa);
        assert!(result.is_ok(), "Failed to parse minishogi grid: {:?}", result);

        let record = result.unwrap();
        assert_eq!(record.black_player, Some("Sente".to_string()));
        assert_eq!(record.white_player, Some("Gote".to_string()));

        // Check that minishogi_bulk was set
        assert!(record.start_pos.minishogi_bulk.is_some());
        assert!(record.start_pos.bulk.is_none());

        let grid = record.start_pos.minishogi_bulk.unwrap();

        // Check white back rank (row 0 = rank 1)
        // Files are 5,4,3,2,1 from left to right (index 0,1,2,3,4)
        assert_eq!(grid[0][0], Some((Color::White, PieceType::Rook)));    // 51 = -HI
        assert_eq!(grid[0][1], Some((Color::White, PieceType::Bishop)));  // 41 = -KA
        assert_eq!(grid[0][2], Some((Color::White, PieceType::Silver)));  // 31 = -GI
        assert_eq!(grid[0][3], Some((Color::White, PieceType::Gold)));    // 21 = -KI
        assert_eq!(grid[0][4], Some((Color::White, PieceType::King)));    // 11 = -OU

        // Check white pawn (row 1 = rank 2)
        assert_eq!(grid[1][4], Some((Color::White, PieceType::Pawn)));    // 12 = -FU
        assert_eq!(grid[1][0], None);  // Empty

        // Check empty row (row 2 = rank 3)
        for col in 0..5 {
            assert_eq!(grid[2][col], None);
        }

        // Check black pawn (row 3 = rank 4)
        assert_eq!(grid[3][0], Some((Color::Black, PieceType::Pawn)));    // 54 = +FU

        // Check black back rank (row 4 = rank 5)
        assert_eq!(grid[4][0], Some((Color::Black, PieceType::King)));    // 55 = +OU
        assert_eq!(grid[4][4], Some((Color::Black, PieceType::Rook)));    // 15 = +HI

        // Check moves
        assert_eq!(record.moves.len(), 3);
        assert_eq!(
            record.moves[0].action,
            Action::Move(Color::Black, Square::new(5, 4), Square::new(5, 3), PieceType::Pawn)
        );
    }

    /// Test minishogi grid round-trip (parse -> serialize -> parse).
    #[test]
    fn test_minishogi_grid_roundtrip() {
        let csa = concat!(
            "V2.2\n",
            "P1-HI-KA-GI-KI-OU\n",
            "P2 *  *  *  * -FU\n",
            "P3 *  *  *  *  * \n",
            "P4+FU *  *  *  * \n",
            "P5+OU+KI+GI+KA+HI\n",
            "+\n",
        );

        let result = parse(csa);
        assert!(result.is_ok(), "Failed to parse: {:?}", result);

        let record = result.unwrap();
        let serialized = record.to_string();

        // Parse the serialized output
        let result2 = parse(&serialized);
        assert!(result2.is_ok(), "Failed to re-parse: {:?}", result2);

        let record2 = result2.unwrap();
        assert_eq!(record.start_pos.minishogi_bulk, record2.start_pos.minishogi_bulk);
    }

    /// Test Wild Cat Shogi with native 3x5 grid format.
    ///
    /// Wild Cat Shogi starting position (3 files x 5 ranks):
    /// ```text
    ///    3   2   1
    /// 1 -KA -OU -HI  (white/gote back rank: Fers, King, Wazir)
    /// 2 -FU  .  -FU  (white pawns)
    /// 3  .   .   .   (empty)
    /// 4 +FU  .  +FU  (black pawns)
    /// 5 +HI +OU +KA  (black/sente back rank: Wazir, King, Fers)
    /// ```
    ///
    /// Note: In Wild Cat Shogi, Wazir uses "HI" (moves 1 square orthogonally)
    /// and Fers uses "KA" (moves 1 square diagonally).
    #[test]
    fn test_wildcat_grid_format() {
        let csa = concat!(
            "V2.2\n",
            "N+Sente\n",
            "N-Gote\n",
            "$EVENT:Wild Cat Shogi Game\n",
            "P1-KA-OU-HI\n",
            "P2-FU * -FU\n",
            "P3 *  *  * \n",
            "P4+FU * +FU\n",
            "P5+HI+OU+KA\n",
            "+\n",
            "+3433FU\n",
            "T5\n",
            "-1213FU\n",
            "T3\n",
            "%TORYO\n",
        );

        let result = parse(csa);
        assert!(result.is_ok(), "Failed to parse wildcat grid: {:?}", result);

        let record = result.unwrap();
        assert_eq!(record.black_player, Some("Sente".to_string()));
        assert_eq!(record.white_player, Some("Gote".to_string()));

        // Check that wildcat_bulk was set
        assert!(record.start_pos.wildcat_bulk.is_some());
        assert!(record.start_pos.bulk.is_none());
        assert!(record.start_pos.minishogi_bulk.is_none());

        let grid = record.start_pos.wildcat_bulk.unwrap();

        // Check white back rank (row 0 = rank 1)
        // Files are 3,2,1 from left to right (index 0,1,2)
        assert_eq!(grid[0][0], Some((Color::White, PieceType::Bishop)));  // 31 = -KA (Fers)
        assert_eq!(grid[0][1], Some((Color::White, PieceType::King)));    // 21 = -OU
        assert_eq!(grid[0][2], Some((Color::White, PieceType::Rook)));    // 11 = -HI (Wazir)

        // Check white pawns (row 1 = rank 2)
        assert_eq!(grid[1][0], Some((Color::White, PieceType::Pawn)));    // 32 = -FU
        assert_eq!(grid[1][1], None);                                      // Empty
        assert_eq!(grid[1][2], Some((Color::White, PieceType::Pawn)));    // 12 = -FU

        // Check empty row (row 2 = rank 3)
        for col in 0..3 {
            assert_eq!(grid[2][col], None);
        }

        // Check black pawns (row 3 = rank 4)
        assert_eq!(grid[3][0], Some((Color::Black, PieceType::Pawn)));    // 34 = +FU
        assert_eq!(grid[3][2], Some((Color::Black, PieceType::Pawn)));    // 14 = +FU

        // Check black back rank (row 4 = rank 5)
        assert_eq!(grid[4][0], Some((Color::Black, PieceType::Rook)));    // 35 = +HI (Wazir)
        assert_eq!(grid[4][1], Some((Color::Black, PieceType::King)));    // 25 = +OU
        assert_eq!(grid[4][2], Some((Color::Black, PieceType::Bishop)));  // 15 = +KA (Fers)

        // Check moves
        assert_eq!(record.moves.len(), 3);
        assert_eq!(
            record.moves[0].action,
            Action::Move(Color::Black, Square::new(3, 4), Square::new(3, 3), PieceType::Pawn)
        );
    }

    /// Test Wild Cat Shogi with drops.
    #[test]
    fn test_wildcat_with_drops() {
        let csa = concat!(
            "V2.2\n",
            "$EVENT:Wild Cat Drop Test\n",
            "P1-KA-OU-HI\n",
            "P2-FU * -FU\n",
            "P3 *  *  * \n",
            "P4+FU * +FU\n",
            "P5+HI+OU+KA\n",
            "+\n",
            "+3433FU\n",
            "-1213FU\n",
            "+3332TO\n",
            "-0033FU\n",
            "%TORYO\n",
        );

        let result = parse(csa);
        assert!(result.is_ok(), "Failed to parse wildcat with drops: {:?}", result);

        let record = result.unwrap();

        // Check promotion move (pawn promotes to gold = TO)
        assert_eq!(
            record.moves[2].action,
            Action::Move(Color::Black, Square::new(3, 3), Square::new(3, 2), PieceType::ProPawn)
        );

        // Check drop move (from square 00 means drop)
        assert_eq!(
            record.moves[3].action,
            Action::Move(Color::White, Square::new(0, 0), Square::new(3, 3), PieceType::Pawn)
        );
    }

    /// Test Wild Cat Shogi grid round-trip.
    #[test]
    fn test_wildcat_grid_roundtrip() {
        let csa = concat!(
            "V2.2\n",
            "P1-KA-OU-HI\n",
            "P2-FU * -FU\n",
            "P3 *  *  * \n",
            "P4+FU * +FU\n",
            "P5+HI+OU+KA\n",
            "+\n",
        );

        let result = parse(csa);
        assert!(result.is_ok(), "Failed to parse: {:?}", result);

        let record = result.unwrap();
        let serialized = record.to_string();

        // Parse the serialized output
        let result2 = parse(&serialized);
        assert!(result2.is_ok(), "Failed to re-parse: {:?}", result2);

        let record2 = result2.unwrap();
        assert_eq!(record.start_pos.wildcat_bulk, record2.start_pos.wildcat_bulk);
    }
}
