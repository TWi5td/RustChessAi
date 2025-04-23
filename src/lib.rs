use std::collections::HashMap;
use std::time::Instant;

use chess::{BitBoard, Board, ChessMove, Color as ChessColor, MoveGen, Piece, Square, BoardStatus, ALL_SQUARES};
use macroquad::prelude::*;
use ::rand::seq::SliceRandom;
use ::rand::{thread_rng, Rng};

const TILE_SIZE: f32      = 80.0;
const BOARD_DIM: f32      = TILE_SIZE * 8.0;
const MAX_DEPTH: i32      = 5;
const TIME_LIMIT_MS: u128 = 500;

const PROMO_PIECES: [Piece; 4] = [
    Piece::Queen,
    Piece::Rook,
    Piece::Bishop,
    Piece::Knight,
];

pub fn window_conf() -> Conf {
    Conf {
        window_title: "Chess AI".to_string(),
        window_width:  (BOARD_DIM + 200.0) as i32,
        window_height: BOARD_DIM as i32,
        ..Default::default()
    }
}

pub async fn run_app() {
    let mut textures = HashMap::new();
    let assets = [
        (PieceKey::PawnWhite,   "assets/white-pawn.png"),
        (PieceKey::KnightWhite, "assets/white-knight.png"),
        (PieceKey::BishopWhite, "assets/white-bishop.png"),
        (PieceKey::RookWhite,   "assets/white-rook.png"),
        (PieceKey::QueenWhite,  "assets/white-queen.png"),
        (PieceKey::KingWhite,   "assets/white-king.png"),
        (PieceKey::PawnBlack,   "assets/black-pawn.png"),
        (PieceKey::KnightBlack, "assets/black-knight.png"),
        (PieceKey::BishopBlack, "assets/black-bishop.png"),
        (PieceKey::RookBlack,   "assets/black-rook.png"),
        (PieceKey::QueenBlack,  "assets/black-queen.png"),
        (PieceKey::KingBlack,   "assets/black-king.png"),
    ];
    for &(key, path) in &assets {
        let t = load_texture(path).await.unwrap();
        t.set_filter(FilterMode::Nearest);
        textures.insert(key, t);
    }

    let mut state = GameState::Menu;
    let mut game = ChessGame { 
        board: Board::default(), 
        selected_square: None, 
        ai_moved: false,
        difficulty: Difficulty::Medium,
    };
    let mut history = Vec::<ChessMove>::new();

    loop {
        clear_background(WHITE);

        match state {
            GameState::Menu => {
                draw_menu();
                draw_difficulty_selection(&mut game.difficulty);
                if is_key_pressed(KeyCode::Enter) {
                    state = GameState::Playing;
                }
            }

            GameState::Playing => {
                draw_board();
                draw_pieces(&game.board, &textures);
                highlight_selection(game.selected_square);
                if let Some(sq) = game.selected_square {
                    draw_legal_moves(sq, &game.board);
                }
                draw_game_status(&game.board);

                let panel_x = BOARD_DIM + 10.0;
                let (pw, ph) = (40.0, 40.0);
                draw_rectangle(panel_x, 10.0, pw, ph, LIGHTGRAY);
                let (bw, bh) = (pw*0.2, ph*0.7);
                let by = 10.0 + (ph-bh)/2.0;
                draw_rectangle(panel_x + pw*0.2, by, bw, bh, BLACK);
                draw_rectangle(panel_x + pw*0.6, by, bw, bh, BLACK);

                if is_mouse_button_pressed(MouseButton::Left) {
                    let (mx, my) = mouse_position();
                    if mx >= panel_x && mx <= panel_x + pw && my >= 10.0 && my <= 10.0 + ph {
                        state = GameState::Paused;
                    }
                    else if let Some((from, to)) = handle_click(&mut game) {
                        if let Some(pc) = game.board.piece_on(from) {
                            let rank = to.get_rank().to_index();
                            if pc == Piece::Pawn && (rank == 0 || rank == 7) {
                                state = GameState::Promotion { from, to };
                            } else {
                                let mv = ChessMove::new(from, to, None);
                                if game.board.legal(mv) {
                                    println!("Making move: {}", mv);
                                    game.board = game.board.make_move_new(mv);
                                    history.push(mv);
                                    game.ai_moved = false;
                                } else {
                                    println!("Illegal move attempted: {}", mv);
                                    game.selected_square = None;
                                }
                            }
                        } else {
                            println!("No piece at source square: {}", from);
                            game.selected_square = None;
                        }
                    }
                }

                if is_key_pressed(KeyCode::P) || is_key_pressed(KeyCode::Escape) {
                    state = GameState::Paused;
                }

                if game.board.side_to_move() == ChessColor::Black && !game.ai_moved {
                    if game.board.status() != BoardStatus::Ongoing {
                        state = GameState::GameOver;
                    } else {
                        let banned = history.iter().rev().next().map(|last| {
                            ChessMove::new(last.get_dest(), last.get_source(), last.get_promotion())
                        });
                        let imb = evaluate_board(&game.board).abs();
                        let depth = if imb >= 900 { MAX_DEPTH + 2 } else { MAX_DEPTH };
                        if let Some(best_mv) = choose_best_move_ab(&game.board, &history, depth, banned, game.difficulty) {
                            game.board = game.board.make_move_new(best_mv);
                            history.push(best_mv);
                            game.ai_moved = true;
                        }
                    }
                }

                let mut my = 10.0 + ph + 30.0;
                draw_text("Moves:", panel_x, my, 24.0, BLACK);
                my += 30.0;
                for (i, mv) in history.iter().enumerate() {
                    draw_text(&format!("{:2}. {}", i+1, mv), panel_x, my, 20.0, BLACK);
                    my += 22.0;
                }

                if game.board.status() != BoardStatus::Ongoing {
                    state = GameState::GameOver;
                }
            }

            GameState::Promotion { from, to } => {
                draw_board();
                draw_pieces(&game.board, &textures);
                draw_promotion_ui(from, to, &textures, &mut state, &mut game, &mut history);
            }

            GameState::Paused => {
                draw_board();
                draw_pieces(&game.board, &textures);
                draw_pause_menu(&mut state, &mut game, &mut history);
            }

            GameState::GameOver => {
                draw_board();
                draw_pieces(&game.board, &textures);
                draw_game_over_ui(&mut state, &mut game, &mut history);
            }
        }

        next_frame().await;
    }
}

enum GameState {
    Menu,
    Playing,
    Paused,
    Promotion { from: Square, to: Square },
    GameOver,
}

#[derive(Clone, Copy)] // Add Copy and Clone
enum Difficulty {
    Easy,
    Medium,
    Hard,
}

#[derive(Clone,Copy,PartialEq,Eq,Hash)]
enum PieceKey {
    PawnWhite, KnightWhite, BishopWhite, RookWhite, QueenWhite, KingWhite,
    PawnBlack, KnightBlack, BishopBlack, RookBlack, QueenBlack, KingBlack,
}

struct ChessGame {
    board: Board,
    selected_square: Option<Square>,
    ai_moved: bool,
    difficulty: Difficulty,
}

fn draw_text_centered(text: &str, x: f32, y: f32, size: f32) {
    let d = measure_text(text, None, size as u16, 1.0);
    draw_text(text, x - d.width/2.0, y, size, BLACK);
}

fn draw_menu() {
    draw_rectangle(0.0, 0.0, BOARD_DIM+200.0, BOARD_DIM, WHITE);
    draw_text_centered("Chess AI", BOARD_DIM/2.0, BOARD_DIM/2.0 - 20.0, 48.0);
    draw_text_centered("Press Enter to Start", BOARD_DIM/2.0, BOARD_DIM/2.0 + 20.0, 24.0);
}

fn draw_difficulty_selection(difficulty: &mut Difficulty) {
    let cx = BOARD_DIM / 2.0;
    let y = BOARD_DIM / 2.0 + 60.0;
    draw_text_centered("Use 1-3 to select difficulty:", cx, y, 20.0);
    draw_text_centered(
        match difficulty {
            Difficulty::Easy => "1: Easy (selected)",
            Difficulty::Medium => "2: Medium (selected)",
            Difficulty::Hard => "3: Hard (selected)",
        },
        cx,
        y + 30.0,
        20.0,
    );

    if is_key_pressed(KeyCode::Key1) {
        *difficulty = Difficulty::Easy;
    }
    if is_key_pressed(KeyCode::Key2) {
        *difficulty = Difficulty::Medium;
    }
    if is_key_pressed(KeyCode::Key3) {
        *difficulty = Difficulty::Hard;
    }
}

fn draw_board() {
    for r in 0..8 {
        for f in 0..8 {
            let c = if (r+f)%2==0 { LIGHTGRAY } else { DARKGRAY };
            draw_rectangle(f as f32*TILE_SIZE, (7-r) as f32*TILE_SIZE, TILE_SIZE, TILE_SIZE, c);
        }
    }
}

fn draw_pieces(board: &Board, texs: &HashMap<PieceKey,Texture2D>) {
    for &sq in ALL_SQUARES.iter() {
        if let Some(pc)=board.piece_on(sq) {
            let clr = board.color_on(sq).unwrap();
            #[allow(unreachable_patterns)]
            let key = match (clr,pc) {
                (ChessColor::White,Piece::Pawn)   => PieceKey::PawnWhite,
                (ChessColor::White,Piece::Knight) => PieceKey::KnightWhite,
                (ChessColor::White,Piece::Bishop) => PieceKey::BishopWhite,
                (ChessColor::White,Piece::Rook)   => PieceKey::RookWhite,
                (ChessColor::White,Piece::Queen)  => PieceKey::QueenWhite,
                (ChessColor::White,Piece::King)   => PieceKey::KingWhite,
                (ChessColor::Black,Piece::Pawn)   => PieceKey::PawnBlack,
                (ChessColor::Black,Piece::Knight) => PieceKey::KnightBlack,
                (ChessColor::Black,Piece::Bishop) => PieceKey::BishopBlack,
                (ChessColor::Black,Piece::Rook)   => PieceKey::RookBlack,
                (ChessColor::Black,Piece::Queen)  => PieceKey::QueenBlack,
                (ChessColor::Black,Piece::King)   => PieceKey::KingBlack,
                _ => continue,
            };
            let x = sq.get_file().to_index() as f32 * TILE_SIZE;
            let y = (7 - sq.get_rank().to_index()) as f32 * TILE_SIZE;
            draw_texture_ex(&texs[&key], x, y, WHITE, DrawTextureParams {
                dest_size: Some(vec2(TILE_SIZE,TILE_SIZE)), ..Default::default()
            });
        }
    }
}

fn highlight_selection(sel: Option<Square>) {
    if let Some(sq)=sel {
        let x = sq.get_file().to_index() as f32 * TILE_SIZE;
        let y = (7 - sq.get_rank().to_index()) as f32 * TILE_SIZE;
        draw_rectangle_lines(x,y,TILE_SIZE,TILE_SIZE,3.0,RED);
    }
}

fn draw_legal_moves(sq: Square, board: &Board) {
    for mv in MoveGen::new_legal(board) {
        if mv.get_source()==sq {
            let d  = mv.get_dest();
            let cx = d.get_file().to_index() as f32*TILE_SIZE + TILE_SIZE/2.0;
            let cy = (7 - d.get_rank().to_index()) as f32*TILE_SIZE + TILE_SIZE/2.0;
            draw_circle(cx, cy, TILE_SIZE*0.1, Color::new(0.,0.8,0.,0.6));
        }
    }
}

fn draw_game_status(board: &Board) {
    if board.status()==BoardStatus::Ongoing && board.checkers().popcnt()>0 {
        draw_text_centered("Check!", BOARD_DIM/2.0, 20.0, 24.0);
    }
}

fn handle_click(game: &mut ChessGame) -> Option<(Square, Square)> {
    let (mx, my) = mouse_position();
    let file     = (mx / TILE_SIZE).floor() as usize;
    let rank_vis = (my / TILE_SIZE).floor() as usize;
    if file < 8 && rank_vis < 8 {
        let rank = 7 - rank_vis;
        let sq = Square::make_square(
            chess::Rank::from_index(rank),
            chess::File::from_index(file),
        );
        let side = game.board.side_to_move();
        if let Some(from) = game.selected_square {
            if game.board.piece_on(sq)
                .map_or(false, |_| game.board.color_on(sq).unwrap() == side)
            {
                game.selected_square = Some(sq);
                return None;
            }
            game.selected_square = None;
            return Some((from, sq));
        } else if game.board.piece_on(sq)
            .map_or(false, |_| game.board.color_on(sq).unwrap() == side)
        {
            game.selected_square = Some(sq);
        }
    }
    None
}

fn draw_promotion_ui(
    from: Square,
    to: Square,
    textures: &HashMap<PieceKey, Texture2D>,
    state: &mut GameState,
    game: &mut ChessGame,
    history: &mut Vec<ChessMove>,
) {
    let cx = BOARD_DIM / 2.0;
    let cy = BOARD_DIM / 2.0;
    let sz = TILE_SIZE;
    for (i, &piece) in PROMO_PIECES.iter().enumerate() {
        let x = cx + (i as f32 - 1.5) * (sz + 10.0);
        let y = cy - sz / 2.0;
        let key = match (game.board.side_to_move(), piece) {
            (ChessColor::White, Piece::Queen)  => PieceKey::QueenWhite,
            (ChessColor::White, Piece::Rook)   => PieceKey::RookWhite,
            (ChessColor::White, Piece::Bishop) => PieceKey::BishopWhite,
            (ChessColor::White, Piece::Knight) => PieceKey::KnightWhite,
            (ChessColor::Black, Piece::Queen)  => PieceKey::QueenBlack,
            (ChessColor::Black, Piece::Rook)   => PieceKey::RookBlack,
            (ChessColor::Black, Piece::Bishop) => PieceKey::BishopBlack,
            (ChessColor::Black, Piece::Knight) => PieceKey::KnightBlack,
            _ => continue,
        };
        draw_texture_ex(
            &textures[&key],
            x, y,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(sz, sz)),
                ..Default::default()
            },
        );
        if is_mouse_button_pressed(MouseButton::Left) {
            let (mx, my) = mouse_position();
            if mx >= x && mx <= x + sz && my >= y && my <= y + sz {
                let mv = ChessMove::new(from, to, Some(piece));
                game.board = game.board.make_move_new(mv);
                history.push(mv);
                game.ai_moved = false;
                *state = GameState::Playing;
                break;
            }
        }
    }
}

fn draw_pause_menu(
    state: &mut GameState,
    game: &mut ChessGame,
    history: &mut Vec<ChessMove>,
) {
    draw_rectangle(0.0, 0.0, BOARD_DIM + 200.0, BOARD_DIM, BLACK.with_alpha(0.5));
    let bw = 160.0;
    let bh = 50.0;
    let cx = (BOARD_DIM + 200.0) / 2.0;
    let labels = ["Resume", "Restart", "Exit"];
    let start_y = BOARD_DIM / 2.0 - (labels.len() as f32 * (bh + 10.0)) / 2.0;

    for (i, &lbl) in labels.iter().enumerate() {
        let x = cx - bw / 2.0;
        let y = start_y + i as f32 * (bh + 10.0);
        draw_rectangle(x, y, bw, bh, LIGHTGRAY);
        draw_text_centered(lbl, x + bw / 2.0, y + bh / 2.0 + 8.0, 24.0);

        if is_mouse_button_pressed(MouseButton::Left) {
            let (mx, my) = mouse_position();
            if mx >= x && mx <= x + bw && my >= y && my <= y + bh {
                match lbl {
                    "Resume" => *state = GameState::Playing,
                    "Restart" => {
                        game.board = Board::default();
                        history.clear();
                        game.selected_square = None;
                        game.ai_moved = false;
                        *state = GameState::Playing;
                    }
                    "Exit" => std::process::exit(0),
                    _ => {}
                }
            }
        }
    }
}

fn draw_game_over_ui(
    state: &mut GameState,
    game: &mut ChessGame,
    history: &mut Vec<ChessMove>,
) {
    let msg = match game.board.status() {
        BoardStatus::Checkmate => {
            if game.board.side_to_move() == ChessColor::White {
                "You were checkmated!"
            } else {
                "Opponent was checkmated!"
            }
        }
        BoardStatus::Stalemate => "Stalemate",
        _ => "Game Over",
    };

    draw_overlay(msg);

    let bw = 120.0;
    let bh = 40.0;
    let rx = BOARD_DIM / 2.0 - bw - 10.0;
    let ex = BOARD_DIM / 2.0 + 10.0;
    let y  = BOARD_DIM / 2.0 + 10.0;

    draw_rectangle(rx, y, bw, bh, LIGHTGRAY);
    draw_text_centered("Restart", rx + bw/2.0, y + bh/2.0 + 5.0, 24.0);

    draw_rectangle(ex, y, bw, bh, LIGHTGRAY);
    draw_text_centered("Exit", ex + bw/2.0, y + bh/2.0 + 5.0, 24.0);

    if is_mouse_button_pressed(MouseButton::Left) {
        let (mx, my) = mouse_position();
        if mx >= rx && mx <= rx + bw && my >= y && my <= y + bh {
            game.board = Board::default();
            history.clear();
            game.selected_square = None;
            game.ai_moved = false;
            *state = GameState::Playing;
        }
        if mx >= ex && mx <= ex + bw && my >= y && my <= y + bh {
            std::process::exit(0);
        }
    }
}

fn draw_overlay(msg: &str) {
    draw_rectangle(0.0, 0.0, BOARD_DIM + 200.0, BOARD_DIM, BLACK.with_alpha(0.5));
    draw_text_centered(msg, BOARD_DIM/2.0, BOARD_DIM/2.0, 36.0);
}

fn evaluate_board(board: &Board) -> i32 {
    let piece_values = [
        (Piece::Pawn, 100),
        (Piece::Knight, 320),
        (Piece::Bishop, 330),
        (Piece::Rook, 500),
        (Piece::Queen, 900),
        (Piece::King, 20000),
    ];

    let mut score = 0;

    for &(piece, value) in &piece_values {
        let white = (board.pieces(piece) & board.color_combined(ChessColor::White)).popcnt() as i32;
        let black = (board.pieces(piece) & board.color_combined(ChessColor::Black)).popcnt() as i32;
        score += (white - black) * value;
    }

    let white_moves = match board.null_move() {
        Some(null_board) => MoveGen::new_legal(&null_board).len() as i32,
        None => 0,
    };
    let black_moves = MoveGen::new_legal(board).len() as i32;
    score += 5 * (white_moves - black_moves);

    let center_squares = [Square::D4, Square::D5, Square::E4, Square::E5];
    let white = board.color_combined(ChessColor::White);
    let black = board.color_combined(ChessColor::Black);

    for sq in center_squares {
        let sq_bb = BitBoard::from_square(sq);
        if (white & sq_bb).popcnt() > 0 {
            if let Some(piece) = board.piece_on(sq) {
                let bonus = match piece {
                    Piece::Pawn => 10,
                    Piece::Knight | Piece::Bishop => 15,
                    Piece::Queen => 5,
                    _ => 0,
                };
                score += bonus;
            }
        }
        if (black & sq_bb).popcnt() > 0 {
            if let Some(piece) = board.piece_on(sq) {
                let bonus = match piece {
                    Piece::Pawn => 10,
                    Piece::Knight | Piece::Bishop => 15,
                    Piece::Queen => 5,
                    _ => 0,
                };
                score -= bonus;
            }
        }
    }

    score
}

fn negamax_ab(
    board: &Board,
    depth: i32,
    mut alpha: i32,
    beta: i32,
    color: i32,
    start: Instant,
) -> i32 {
    if start.elapsed().as_millis() > TIME_LIMIT_MS || board.status() != BoardStatus::Ongoing {
        return color * evaluate_board(board);
    }

    if depth == 0 {
        return color * evaluate_board(board);
    }

    let mut best = i32::MIN;
    let mut moves: Vec<ChessMove> = MoveGen::new_legal(board).collect();

    moves.sort_by_key(|mv| {
        let mut score = 0;
        if board.piece_on(mv.get_dest()).is_some() {
            score -= 1000;
        }
        if mv.get_promotion().is_some() {
            score -= 800;
        }
        score
    });

    for mv in moves {
        let next = board.make_move_new(mv);
        let is_capture = board.piece_on(mv.get_dest()).is_some();
        let is_promo = mv.get_promotion().is_some();
        let is_check = next.checkers().popcnt() > 0;

        let extension = if is_check || is_capture || is_promo { 1 } else { 0 };
        let score = -negamax_ab(&next, depth - 1 + extension, -beta, -alpha, -color, start);

        best = best.max(score);
        alpha = alpha.max(score);
        if alpha >= beta {
            break;
        }
    }

    best
}

fn choose_best_move_ab(
    board: &Board,
    _history: &[ChessMove],
    depth: i32,
    banned: Option<ChessMove>,
    difficulty: Difficulty,
) -> Option<ChessMove> {
    let start = Instant::now();
    let mut moves: Vec<_> = MoveGen::new_legal(board)
        .filter(|&mv| Some(mv) != banned)
        .collect();

    if moves.is_empty() {
        return None;
    }

    let (effective_depth, random_chance) = match difficulty {
        Difficulty::Easy => (2, 0.7),
        Difficulty::Medium => (depth, 0.0),
        Difficulty::Hard => (depth + 2, 0.0),
    };

    if matches!(difficulty, Difficulty::Easy) && thread_rng().gen::<f32>() < random_chance {
        return Some(*moves.choose(&mut thread_rng()).unwrap());
    }

    let mut best_move = None;
    let mut best_score = i32::MIN;

    moves.sort_by_key(|mv| {
        let mut score = 0;
        if board.piece_on(mv.get_dest()).is_some() {
            score -= 1000;
        }
        if mv.get_promotion().is_some() {
            score -= 800;
        }
        score
    });

    for mv in moves {
        let next = board.make_move_new(mv);
        let is_capture = board.piece_on(mv.get_dest()).is_some();
        let is_promo = mv.get_promotion().is_some();
        let is_check = next.checkers().popcnt() > 0;

        let extension = if is_check || is_capture || is_promo { 1 } else { 0 };

        let score = -negamax_ab(
            &next,
            effective_depth - 1 + extension,
            i32::MIN + 1,
            i32::MAX,
            if board.side_to_move() == ChessColor::White { 1 } else { -1 },
            start,
        );

        if score > best_score {
            best_score = score;
            best_move = Some(mv);
        }

        if start.elapsed().as_millis() > TIME_LIMIT_MS {
            break;
        }
    }

    best_move
}