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
        last_move: None, 
        captured_white: Vec::new(),
        captured_black: Vec::new(),
    };
    let mut history = Vec::<ChessMove>::new();
    let mut moves_scroll_offset: f32 = 0.0;

    let mut moves_scroll_offset = 0.0;
    let mut user_scrolled = false;


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
            draw_last_move(game.last_move);
            draw_captured_pieces(&game.captured_white, &game.captured_black, &textures);

            // Panel base
            let panel_x = BOARD_DIM + 10.0;
            let (pw, ph) = (40.0, 40.0);

            // Pause Button
            draw_rectangle(panel_x, 10.0, pw, ph, LIGHTGRAY);
            let (bw, bh) = (pw * 0.2, ph * 0.7);
            let by = 10.0 + (ph - bh) / 2.0;
            draw_rectangle(panel_x + pw * 0.2, by, bw, bh, BLACK);
            draw_rectangle(panel_x + pw * 0.6, by, bw, bh, BLACK);

            if is_mouse_button_pressed(MouseButton::Left) {
                let (mx, my) = mouse_position();
                if mx >= panel_x && mx <= panel_x + pw && my >= 10.0 && my <= 10.0 + ph {
                    state = GameState::Paused;
                } else if let Some((from, to)) = handle_click(&mut game) {
                    if let Some(pc) = game.board.piece_on(from) {
                        let rank = to.get_rank().to_index();
                        if pc == Piece::Pawn && (rank == 0 || rank == 7) {
                            state = GameState::Promotion { from, to };
                        } else {
                            let mv = ChessMove::new(from, to, None);
                            if game.board.legal(mv) {
                                if let Some(captured) = game.board.piece_on(to) {
                                    if game.board.side_to_move() == ChessColor::White {
                                        game.captured_black.push(captured);
                                    } else {
                                        game.captured_white.push(captured);
                                    }
                                }
                                game.board = game.board.make_move_new(mv);
                                history.push(mv);
                                game.last_move = Some(mv);
                                game.ai_moved = false;
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

                    let depth = MAX_DEPTH;

                    if let Some(best_mv) = choose_best_move_ab(&game.board, depth) {
                        if let Some(captured) = game.board.piece_on(best_mv.get_dest()) {
                            if game.board.side_to_move() == ChessColor::White {
                                game.captured_black.push(captured);
                            } else {
                                game.captured_white.push(captured);
                            }
                        }
                        game.board = game.board.make_move_new(best_mv);
                        history.push(best_mv);
                        game.last_move = Some(best_mv);
                        game.ai_moved = true;
                    }
                }
            }

            // ------ Pause Button ------

            let pause_button_x = BOARD_DIM + 10.0;
            let pause_button_y = 10.0;
            let pause_button_width = 40.0;
            let pause_button_height = 40.0;

            // Background under pause button
            draw_rectangle(pause_button_x, pause_button_y, pause_button_width, pause_button_height, LIGHTGRAY);

            // Pause button "bars"
            let bar_width = pause_button_width * 0.2;
            let bar_height = pause_button_height * 0.7;
            let bar_y = pause_button_y + (pause_button_height - bar_height) / 2.0;
            draw_rectangle(pause_button_x + pause_button_width * 0.2, bar_y, bar_width, bar_height, BLACK);
            draw_rectangle(pause_button_x + pause_button_width * 0.6, bar_y, bar_width, bar_height, BLACK);

            // Pause Button click
            if is_mouse_button_pressed(MouseButton::Left) {
                let (mx, my) = mouse_position();
                if mx >= pause_button_x && mx <= pause_button_x + pause_button_width &&
                   my >= pause_button_y && my <= pause_button_y + pause_button_height {
                    state = GameState::Paused;
                }
            }


            // ------ Moves Panel ------

            let panel_x = BOARD_DIM + 10.0;
            let panel_width = 180.0;

            // Start moves label **after** pause button
            let moves_label_y = pause_button_y + pause_button_height + 10.0;
            draw_text("Moves:", panel_x, moves_label_y, 24.0, BLACK);

            // Moves Area
            let moves_area_top = moves_label_y + 30.0;
            let moves_area_height = BOARD_DIM * 0.45;
            let moves_area_bottom = moves_area_top + moves_area_height;

            // Draw background
            draw_rectangle(panel_x, moves_area_top, panel_width, moves_area_height, LIGHTGRAY);

            // Scrolling logic
            let move_line_height = 22.0;
            let total_moves_height = history.len() as f32 * move_line_height;
            let max_scroll = (total_moves_height - moves_area_height).max(0.0);

            let (_, scroll_y) = mouse_wheel();
            moves_scroll_offset -= scroll_y * 20.0;
            moves_scroll_offset = moves_scroll_offset.clamp(-max_scroll, 0.0);

            if !user_scrolled && total_moves_height > moves_area_height {
                moves_scroll_offset = -max_scroll;
            }
            if scroll_y.abs() > 0.0 {
                user_scrolled = true;
            }
            if (moves_scroll_offset + max_scroll).abs() < 5.0 {
                user_scrolled = false;
            }

            // --- Dragging Scrollbar ---
            let mouse = mouse_position();
            let scrollbar_width = 6.0;
            let scrollbar_x = panel_x + panel_width - scrollbar_width;
            let scrollbar_height = moves_area_height * (moves_area_height / total_moves_height).min(moves_area_height);
            let scrollbar_max_offset = moves_area_height - scrollbar_height;
            let scrollbar_y = moves_area_top + (-moves_scroll_offset / max_scroll * scrollbar_max_offset);

            static mut DRAGGING_SCROLL: bool = false;
            static mut DRAG_OFFSET_Y: f32 = 0.0;

            if is_mouse_button_pressed(MouseButton::Left) {
                if mouse.0 >= scrollbar_x && mouse.0 <= scrollbar_x + scrollbar_width &&
                   mouse.1 >= scrollbar_y && mouse.1 <= scrollbar_y + scrollbar_height {
                    unsafe {
                        DRAGGING_SCROLL = true;
                        DRAG_OFFSET_Y = mouse.1 - scrollbar_y;
                    }
                }
            }
            if is_mouse_button_down(MouseButton::Left) {
                unsafe {
                    if DRAGGING_SCROLL {
                        let mut new_scrollbar_y = mouse.1 - DRAG_OFFSET_Y;
                        new_scrollbar_y = new_scrollbar_y.clamp(moves_area_top, moves_area_bottom - scrollbar_height);
                        moves_scroll_offset = -(new_scrollbar_y - moves_area_top) / scrollbar_max_offset * max_scroll;
                    }
                }
            } else {
                unsafe { DRAGGING_SCROLL = false; }
            }

            // Draw each move
            let vertical_padding = 8.0;
            for (i, mv) in history.iter().enumerate() {
                let y = moves_area_top + vertical_padding + (i as f32) * move_line_height + moves_scroll_offset;
                if y > moves_area_top - move_line_height && y < moves_area_bottom {
                    draw_text(&format!("{:2}. {}", i + 1, mv), panel_x + 5.0, y, 20.0, BLACK);
                }
            }

            // Draw scrollbar
            if max_scroll > 0.0 {
                let hovered = mouse.0 >= scrollbar_x && mouse.0 <= scrollbar_x + scrollbar_width &&
                              mouse.1 >= moves_area_top && mouse.1 <= moves_area_bottom;
                draw_rectangle(
                    scrollbar_x,
                    moves_area_top + (-moves_scroll_offset / max_scroll) * scrollbar_max_offset,
                    scrollbar_width,
                    scrollbar_height,
                    if hovered { GRAY } else { DARKGRAY },
                );
            }

            // ------ End Moves Panel ------

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
    last_move: Option<ChessMove>,         
    captured_white: Vec<Piece>,
    captured_black: Vec<Piece>,         
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
    let labels = ["Resume", "Restart", "Undo", "Exit"];
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
                        game.last_move = None;
                        game.captured_white.clear();  // <<< ADD THIS
                        game.captured_black.clear();  // <<< AND THIS
                        *state = GameState::Playing;
                    }
                    "Undo" => {
                        if let Some(_) = history.pop() { // undo AI move
                            if let Some(_) = history.pop() { // undo player move
                                game.board = Board::default();
                                for &mv in history.iter() {
                                    game.board = game.board.make_move_new(mv);
                                }
                                game.selected_square = None;
                                game.ai_moved = false;
                                game.last_move = history.last().copied();
                                rebuild_captured_pieces(&history, &mut game.captured_white, &mut game.captured_black);
                            }
                        }
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

fn draw_last_move(last_move: Option<ChessMove>) {
    if let Some(mv) = last_move {
        let (from, to) = (mv.get_source(), mv.get_dest());
        for &sq in &[from, to] {
            let x = sq.get_file().to_index() as f32 * TILE_SIZE;
            let y = (7 - sq.get_rank().to_index()) as f32 * TILE_SIZE;
            draw_rectangle_lines(x, y, TILE_SIZE, TILE_SIZE, 4.0, YELLOW);
        }
    }
}

fn draw_captured_pieces(
    captured_white: &Vec<Piece>,
    captured_black: &Vec<Piece>,
    textures: &HashMap<PieceKey, Texture2D>,
) {
    let panel_x = BOARD_DIM + 10.0;
    let icon_size = 30.0;
    let spacing = 5.0;
    let per_row = 4;

    // Define bottom area starting point
    let mut y_start = BOARD_DIM - 10.0; // Start from very bottom

    // First draw captured White pieces (captured by Black)
    let mut x = panel_x;
    let mut y = y_start - icon_size; // go up
    for (i, &piece) in captured_white.iter().enumerate() {
        let key = match piece {
            Piece::Pawn => PieceKey::PawnWhite,
            Piece::Knight => PieceKey::KnightWhite,
            Piece::Bishop => PieceKey::BishopWhite,
            Piece::Rook => PieceKey::RookWhite,
            Piece::Queen => PieceKey::QueenWhite,
            Piece::King => PieceKey::KingWhite,
        };

        draw_texture_ex(
            &textures[&key],
            x,
            y,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(icon_size, icon_size)),
                ..Default::default()
            },
        );

        if (i + 1) % per_row == 0 {
            x = panel_x;
            y -= icon_size + spacing;
        } else {
            x += icon_size + spacing;
        }
    }

    // Then draw captured Black pieces (captured by White)
    // start higher so it's separate
    let captured_white_rows = (captured_white.len() + per_row - 1) / per_row;
    y = y - 20.0; // small gap between white and black captured
    y -= (icon_size + spacing) * captured_white_rows as f32;

    x = panel_x;
    for (i, &piece) in captured_black.iter().enumerate() {
        let key = match piece {
            Piece::Pawn => PieceKey::PawnBlack,
            Piece::Knight => PieceKey::KnightBlack,
            Piece::Bishop => PieceKey::BishopBlack,
            Piece::Rook => PieceKey::RookBlack,
            Piece::Queen => PieceKey::QueenBlack,
            Piece::King => PieceKey::KingBlack,
        };

        draw_texture_ex(
            &textures[&key],
            x,
            y,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(icon_size, icon_size)),
                ..Default::default()
            },
        );

        if (i + 1) % per_row == 0 {
            x = panel_x;
            y -= icon_size + spacing;
        } else {
            x += icon_size + spacing;
        }
    }
}



fn rebuild_captured_pieces(
    history: &[ChessMove],
    captured_white: &mut Vec<Piece>,
    captured_black: &mut Vec<Piece>,
) {
    let mut board = Board::default();
    captured_white.clear();
    captured_black.clear();

    for &mv in history {
        if let Some(captured) = board.piece_on(mv.get_dest()) {
            if board.side_to_move() == ChessColor::White {
                captured_black.push(captured);
            } else {
                captured_white.push(captured);
            }
        }
        board = board.make_move_new(mv);
    }
}


fn draw_eval_bar(score: i32) {
    let panel_x = BOARD_DIM + 70.0;
    let panel_top = 10.0;
    let panel_height = BOARD_DIM - 20.0;
    let mid_y = panel_top + panel_height / 2.0;
    
    let clamped_score = score.clamp(-2000, 2000) as f32 / 2000.0;
    let bar_y = mid_y - clamped_score * (panel_height / 2.0);

    draw_rectangle(panel_x, panel_top, 20.0, panel_height, GRAY);
    draw_rectangle(panel_x, bar_y, 20.0, 5.0, RED);
}


fn draw_overlay(msg: &str) {
    draw_rectangle(0.0, 0.0, BOARD_DIM + 200.0, BOARD_DIM, BLACK.with_alpha(0.5));
    draw_text_centered(msg, BOARD_DIM/2.0, BOARD_DIM/2.0, 36.0);
}

/*fn evaluate_board(board: &Board, _difficulty: Difficulty) -> i32 {
    let piece_values = [
        (Piece::Pawn, 100),
        (Piece::Knight, 320),
        (Piece::Bishop, 330),
        (Piece::Rook, 500),
        (Piece::Queen, 900),
        (Piece::King, 20000),
    ];

    let mut score = 0;

    // Material Count
    for &(piece, value) in &piece_values {
        let white = (board.pieces(piece) & board.color_combined(ChessColor::White)).popcnt() as i32;
        let black = (board.pieces(piece) & board.color_combined(ChessColor::Black)).popcnt() as i32;
        score += (white - black) * value;
    }

    // Mobility
    let white_moves = match board.null_move() {
        Some(null_board) => MoveGen::new_legal(&null_board).len() as i32,
        None => 0,
    };
    let black_moves = MoveGen::new_legal(board).len() as i32;
    score += 2 * (white_moves - black_moves);

    // Center control
    let center_squares = [Square::D4, Square::D5, Square::E4, Square::E5];
    for &sq in &center_squares {
        if let Some(piece) = board.piece_on(sq) {
            let color = board.color_on(sq).unwrap();
            match color {
                ChessColor::White => score += 30,
                ChessColor::Black => score -= 30,
            }
        }
    }

    // Development bonus
    for sq in ALL_SQUARES {
        if let Some(piece) = board.piece_on(sq) {
            let color = board.color_on(sq).unwrap();
            match (color, piece) {
                (ChessColor::White, Piece::Knight) | (ChessColor::White, Piece::Bishop) => {
                    if sq.get_rank().to_index() > 1 {
                        score += 15;
                    }
                }
                (ChessColor::Black, Piece::Knight) | (ChessColor::Black, Piece::Bishop) => {
                    if sq.get_rank().to_index() < 6 {
                        score -= 15;
                    }
                }
                _ => {}
            }
        }
    }

    // King position bonus: Encourage safer kings (not too close to the middle)
    let black_king_sq = board.king_square(ChessColor::Black);
    let white_king_sq = board.king_square(ChessColor::White);

    let center_files = [3, 4]; // D and E files
    let center_ranks = [3, 4]; // ranks 4 and 5

    if !center_files.contains(&black_king_sq.get_file().to_index())
        || !center_ranks.contains(&black_king_sq.get_rank().to_index()) {
        score += 20; // Black king safer (less central)
    } else {
        score -= 20; // Black king exposed in center
    }

    if !center_files.contains(&white_king_sq.get_file().to_index())
        || !center_ranks.contains(&white_king_sq.get_rank().to_index()) {
        score -= 20; // White king safer
    } else {
        score += 20; // White king exposed
    }

    score
}*/

// Regular negamax_ab: no full evaluation at leaves anymore
fn negamax_ab(board: &Board, depth: i32, mut alpha: i32, beta: i32, color: i32) -> i32 {
    if board.status() != BoardStatus::Ongoing {
        return match board.status() {
            BoardStatus::Checkmate => -color * 1_000_000,
            BoardStatus::Stalemate => 0,
            _ => 0,
        };
    }

    if depth == 0 {
        return color * quiescence_search(board, alpha, beta, color);
    }

    let mut best_score = i32::MIN;
    let mut moves: Vec<ChessMove> = MoveGen::new_legal(board).collect();

    moves.sort_by_key(|mv| {
        let mut priority = 0;
        if board.piece_on(mv.get_dest()).is_some() {
            priority -= 10_000;
        }
        if mv.get_promotion().is_some() {
            priority -= 8000;
        }
        if board.make_move_new(*mv).checkers().popcnt() > 0 {
            priority -= 5000;
        }
        priority
    });

    for mv in moves {
        let next = board.make_move_new(mv);
        let score = -negamax_ab(&next, depth - 1, -beta, -alpha, -color);

        best_score = best_score.max(score);
        alpha = alpha.max(score);
        if alpha >= beta {
            break; // Beta cutoff
        }
    }

    best_score
}

// Quiescence search: only explores capture moves/checks when at depth 0
fn quiescence_search(board: &Board, mut alpha: i32, beta: i32, color: i32) -> i32 {
    if board.status() != BoardStatus::Ongoing {
        return match board.status() {
            BoardStatus::Checkmate => -color * 1_000_000,
            BoardStatus::Stalemate => 0,
            _ => 0,
        };
    }

    let stand_pat = stand_pat(board, color);
    if stand_pat >= beta {
        return beta;
    }
    if alpha < stand_pat {
        alpha = stand_pat;
    }

    let mut captures: Vec<ChessMove> = MoveGen::new_legal(board)
        .filter(|mv| board.piece_on(mv.get_dest()).is_some() || mv.get_promotion().is_some())
        .collect();

    captures.sort_by_key(|mv| {
        let mut priority = 0;
        if board.piece_on(mv.get_dest()).is_some() {
            priority -= 10_000;
        }
        if mv.get_promotion().is_some() {
            priority -= 8000;
        }
        if board.make_move_new(*mv).checkers().popcnt() > 0 {
            priority -= 5000;
        }
        priority
    });

    for mv in captures {
        let next = board.make_move_new(mv);
        let score = -quiescence_search(&next, -beta, -alpha, -color);

        if score >= beta {
            return beta;
        }
        if score > alpha {
            alpha = score;
        }
    }

    alpha
}

fn stand_pat(board: &Board, color: i32) -> i32 {
    let mut score = 0;

    for sq in ALL_SQUARES {
        if let Some(piece) = board.piece_on(sq) {
            let piece_color = board.color_on(sq).unwrap();
            if piece_color == ChessColor::White {
                match piece {
                    Piece::Knight | Piece::Bishop => {
                        if sq.get_rank().to_index() > 1 {
                            score += 10;
                        }
                    }
                    Piece::Rook => {
                        if sq.get_rank().to_index() > 0 {
                            score += 5;
                        }
                    }
                    Piece::King => {
                        if sq.get_file() == chess::File::G || sq.get_file() == chess::File::C {
                            score += 20; // castled king
                        }
                    }
                    _ => {}
                }
            }
            if piece_color == ChessColor::Black {
                match piece {
                    Piece::Knight | Piece::Bishop => {
                        if sq.get_rank().to_index() < 6 {
                            score -= 10;
                        }
                    }
                    Piece::Rook => {
                        if sq.get_rank().to_index() < 7 {
                            score -= 5;
                        }
                    }
                    Piece::King => {
                        if sq.get_file() == chess::File::G || sq.get_file() == chess::File::C {
                            score -= 20;
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    color * score
}

fn choose_best_move_ab(board: &Board, depth: i32) -> Option<ChessMove> {
    let mut moves: Vec<ChessMove> = MoveGen::new_legal(board).collect();

    if moves.is_empty() {
        return None;
    }

    moves.sort_by_key(|mv| {
        let mut priority = 0;
        if board.piece_on(mv.get_dest()).is_some() {
            priority -= 10_000;
        }
        if mv.get_promotion().is_some() {
            priority -= 8000;
        }
        if board.make_move_new(*mv).checkers().popcnt() > 0 {
            priority -= 5000;
        }
        priority
    });

    let mut best_move = None;
    let mut best_score = i32::MIN;

    for mv in moves {
        let next = board.make_move_new(mv);
        let color = if board.side_to_move() == ChessColor::White { 1 } else { -1 };
        let score = -negamax_ab(&next, depth - 1, i32::MIN + 1, i32::MAX, -color);

        if score > best_score {
            best_score = score;
            best_move = Some(mv);
        }
    }

    best_move
}
