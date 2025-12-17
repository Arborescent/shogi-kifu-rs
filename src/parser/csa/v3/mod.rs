//! CSA V3.0 format parser
//!
//! TODO: This is a stub. Full V3.0 implementation needed with:
//! - Encoding declaration
//! - New $TIME format (Fischer)
//! - Program-readable comments ('*, '**)
//! - Millisecond time support
//! - New attributes ($MAX_MOVES, $JISHOGI, $NOTE)
//! - MAX_MOVES action (requires value.rs update)

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
        write!(f, "CSA V3.0 parse error: {}", self.0)
    }
}

impl std::error::Error for ParseError {}

#[derive(Parser)]
#[grammar = "parser/csa/v3/grammar.pest"]
struct CsaParser;

type Grid = [[Option<(Color, PieceType)>; 9]; 9];

pub fn parse(input: &str) -> Result<GameRecord, ParseError> {
    let pairs = CsaParser::parse(Rule::game_record, input)
        .map_err(|e| ParseError(e.to_string()))?;

    let mut record = GameRecord::default();

    for pair in pairs {
        if pair.as_rule() == Rule::game_record {
            for inner in pair.into_inner() {
                match inner.as_rule() {
                    Rule::black_player => record.black_player = parse_player_name(inner),
                    Rule::white_player => record.white_player = parse_player_name(inner),
                    Rule::game_attr => parse_game_attr(inner, &mut record),
                    Rule::position => record.start_pos = parse_position(inner),
                    Rule::side_to_move => record.start_pos.side_to_move = parse_side_to_move(inner),
                    Rule::move_records => record.moves = parse_move_records(inner),
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
            if !name.is_empty() { return Some(name.to_string()); }
        }
    }
    None
}

fn parse_game_attr(pair: pest::iterators::Pair<Rule>, record: &mut GameRecord) {
    let mut key = String::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::attr_key => key = inner.as_str().to_string(),
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
                        // TODO: Handle time_control for $TIME format
                        Rule::attr_text => {
                            let text = value_inner.as_str().to_string();
                            match key.as_str() {
                                "EVENT" => record.event = Some(text),
                                "SITE" => record.site = Some(text),
                                "OPENING" => record.opening = Some(text),
                                // TODO: Handle MAX_MOVES, JISHOGI, NOTE
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

    date_str.and_then(|d| {
        let date_parts: Vec<&str> = d.split('/').collect();
        if date_parts.len() != 3 { return None; }

        let year: i32 = date_parts[0].parse().ok()?;
        let month: u8 = date_parts[1].parse().ok()?;
        let day: u8 = date_parts[2].parse().ok()?;
        let month = Month::try_from(month).ok()?;
        let date = NativeDate::from_calendar_date(year, month, day).ok()?;

        let time = time_str.and_then(|t| {
            let parts: Vec<&str> = t.split(':').collect();
            if parts.len() != 3 { return None; }
            let hour: u8 = parts[0].parse().ok()?;
            let minute: u8 = parts[1].parse().ok()?;
            let second: u8 = parts[2].parse().ok()?;
            NativeTime::from_hms(hour, minute, second).ok()
        });

        Some(Time { date, time })
    })
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

fn parse_position(pair: pest::iterators::Pair<Rule>) -> Position {
    let mut pos = Position::default();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::handicap => pos.drop_pieces = parse_handicap(inner),
            Rule::grid => pos.bulk = Some(parse_grid(inner)),
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
            Rule::grid_row1 => Some(0), Rule::grid_row2 => Some(1), Rule::grid_row3 => Some(2),
            Rule::grid_row4 => Some(3), Rule::grid_row5 => Some(4), Rule::grid_row6 => Some(5),
            Rule::grid_row7 => Some(6), Rule::grid_row8 => Some(7), Rule::grid_row9 => Some(8),
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

fn parse_grid_cell(pair: pest::iterators::Pair<Rule>) -> Option<(Color, PieceType)> {
    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::grid_piece => {
                let mut color = Color::Black;
                let mut piece = PieceType::Pawn;
                for p in inner.into_inner() {
                    match p.as_rule() {
                        Rule::color => color = parse_color(p.as_str()),
                        Rule::piece_type => piece = parse_piece_type(p.as_str()),
                        _ => {}
                    }
                }
                return Some((color, piece));
            }
            Rule::grid_empty => return None,
            _ => {}
        }
    }
    None
}

fn parse_piece_placements(pair: pest::iterators::Pair<Rule>) -> Vec<(Color, Square, PieceType)> {
    let mut placements = Vec::new();

    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::piece_placement {
            let mut color = Color::Black;
            for p in inner.into_inner() {
                match p.as_rule() {
                    Rule::color => color = parse_color(p.as_str()),
                    Rule::placement_piece => {
                        let mut sq = Square::new(0, 0);
                        let mut pt = PieceType::Pawn;
                        for pp in p.into_inner() {
                            match pp.as_rule() {
                                Rule::square => sq = parse_square(pp.as_str()),
                                Rule::piece_type => pt = parse_piece_type(pp.as_str()),
                                _ => {}
                            }
                        }
                        placements.push((color, sq, pt));
                    }
                    _ => {}
                }
            }
        }
    }

    placements
}

fn parse_side_to_move(pair: pest::iterators::Pair<Rule>) -> Color {
    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::color { return parse_color(inner.as_str()); }
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
                    moves.push(MoveRecord { action, time: Some(time) });
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
    let mut sq_count = 0;

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::color => color = parse_color(inner.as_str()),
            Rule::square => {
                if sq_count == 0 { from = parse_square(inner.as_str()); }
                else { to = parse_square(inner.as_str()); }
                sq_count += 1;
            }
            Rule::piece_type => piece = parse_piece_type(inner.as_str()),
            _ => {}
        }
    }

    Action::Move(color, from, to, piece)
}

// V3.0 special moves (added MAX_MOVES, removed MATTA)
// Note: MAX_MOVES maps to Error until value.rs is updated
fn parse_special_move(s: &str) -> Action {
    if s.contains("TORYO") { Action::Toryo }
    else if s.contains("CHUDAN") { Action::Chudan }
    else if s.contains("SENNICHITE") { Action::Sennichite }
    else if s.contains("TIME_UP") { Action::TimeUp }
    else if s.contains("ILLEGAL_MOVE") { Action::IllegalMove }
    else if s.contains("+ILLEGAL_ACTION") { Action::IllegalAction(Color::Black) }
    else if s.contains("-ILLEGAL_ACTION") { Action::IllegalAction(Color::White) }
    else if s.contains("JISHOGI") { Action::Jishogi }
    else if s.contains("KACHI") { Action::Kachi }
    else if s.contains("HIKIWAKE") { Action::Hikiwake }
    else if s.contains("MAX_MOVES") { Action::Error } // TODO: Add MaxMoves to Action enum
    else if s.contains("TSUMI") { Action::Tsumi }
    else if s.contains("FUZUMI") { Action::Fuzumi }
    else { Action::Error }
}

// V3.0 supports millisecond time
fn parse_time_consumed(pair: pest::iterators::Pair<Rule>) -> Duration {
    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::seconds_consumed {
            let s = inner.as_str();
            if let Some(dot_pos) = s.find('.') {
                let secs: u64 = s[..dot_pos].parse().unwrap_or(0);
                let frac_str = &s[dot_pos + 1..];
                let millis: u64 = match frac_str.len() {
                    1 => frac_str.parse::<u64>().unwrap_or(0) * 100,
                    2 => frac_str.parse::<u64>().unwrap_or(0) * 10,
                    3 => frac_str.parse::<u64>().unwrap_or(0),
                    _ => 0,
                };
                return Duration::from_millis(secs * 1000 + millis);
            } else {
                return Duration::from_secs(s.parse().unwrap_or(0));
            }
        }
    }
    Duration::from_secs(0)
}

fn parse_color(s: &str) -> Color {
    match s { "+" => Color::Black, "-" => Color::White, _ => Color::Black }
}

fn parse_square(s: &str) -> Square {
    let chars: Vec<char> = s.chars().collect();
    Square::new(
        chars[0].to_digit(10).unwrap_or(0) as u8,
        chars[1].to_digit(10).unwrap_or(0) as u8,
    )
}

fn parse_piece_type(s: &str) -> PieceType {
    match s {
        "FU" => PieceType::Pawn, "KY" => PieceType::Lance, "KE" => PieceType::Knight,
        "GI" => PieceType::Silver, "KI" => PieceType::Gold, "KA" => PieceType::Bishop,
        "HI" => PieceType::Rook, "OU" => PieceType::King, "TO" => PieceType::ProPawn,
        "NY" => PieceType::ProLance, "NK" => PieceType::ProKnight, "NG" => PieceType::ProSilver,
        "UM" => PieceType::Horse, "RY" => PieceType::Dragon, "AL" => PieceType::All,
        _ => PieceType::Pawn,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple() {
        let csa = "V3.0\nPI\n+\n+2726FU\n";
        let result = parse(csa);
        assert!(result.is_ok(), "Failed: {:?}", result);
    }

    #[test]
    fn test_parse_with_milliseconds() {
        let csa = "V3.0\nPI\n+\n+2726FU\nT15.123\n";
        let result = parse(csa);
        assert!(result.is_ok(), "Failed: {:?}", result);
        let record = result.unwrap();
        assert_eq!(record.moves[0].time, Some(Duration::from_millis(15123)));
    }
}
