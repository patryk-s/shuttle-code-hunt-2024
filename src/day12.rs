use std::{fmt::Display, sync::Arc};

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use tokio::sync::RwLock;

pub fn router() -> Router {
    let board = Arc::new(RwLock::new(Board::default()));

    Router::new()
        .route("/12/board", get(get_board))
        .route("/12/reset", post(reset_board))
        .route("/12/place/:team/:column", post(place))
        .with_state(board.clone())
}

#[axum::debug_handler]
async fn get_board(State(board): State<Arc<RwLock<Board>>>) -> impl IntoResponse {
    board.read().await.to_string()
}

#[axum::debug_handler]
async fn reset_board(State(board): State<Arc<RwLock<Board>>>) -> impl IntoResponse {
    eprintln!("resetting");
    let mut board = board.write_owned().await;
    *board = Board::default();
    board.to_string()
}

#[axum::debug_handler]
async fn place(
    State(board): State<Arc<RwLock<Board>>>,
    Path((team, column)): Path<(String, u8)>,
) -> impl IntoResponse {
    let team = match team.as_str() {
        "cookie" => Tile::Cookie,
        "milk" => Tile::Milk,
        _ => return StatusCode::BAD_REQUEST.into_response(),
    };
    if column == 0 || column > 4 {
        return StatusCode::BAD_REQUEST.into_response();
    }
    let column = column - 1;
    let mut board = board.write_owned().await;
    match board.place(team, column as usize) {
        Ok(_) => board.to_string().into_response(),
        Err(_) => (StatusCode::SERVICE_UNAVAILABLE, board.to_string()).into_response(),
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
enum Tile {
    Empty,
    Cookie,
    Milk,
}

impl Display for Tile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Empty => "⬛",
            Self::Cookie => "🍪",
            Self::Milk => "🥛",
        };
        write!(f, "{s}")
    }
}

#[derive(Debug)]
struct Board {
    winner: Option<Tile>,
    state: [Tile; 16],
}

impl Board {
    fn place(&mut self, tile: Tile, column: usize) -> Result<(), String> {
        // let current_state = self.state;
        if self.state[column] != Tile::Empty {
            return Err("tile not empty".to_string());
        }
        if self.winner.is_some() {
            return Err("game already over".to_string());
        }
        for i in (0..4).rev() {
            let spot = column + i * 4;
            if self.state[spot] == Tile::Empty {
                self.state[spot] = tile;
                break;
            }
        }
        self.check_winner();
        Ok(())
    }

    fn check_winner(&mut self) {
        eprintln!("checking winner");
        // check first diagonal
        let tile = self.state[0];
        if tile != Tile::Empty
            && tile == self.state[5]
            && tile == self.state[10]
            && tile == self.state[15]
        {
            self.winner = Some(tile);
            return;
        }

        // check second diagonal
        let tile = self.state[3];
        if tile != Tile::Empty
            && tile == self.state[6]
            && tile == self.state[9]
            && tile == self.state[12]
        {
            self.winner = Some(tile);
            return;
        }

        // check horizontal, starting from bottom row
        for i in (0..4).rev() {
            let tile = self.state[i * 4];
            if tile != Tile::Empty
                && tile == self.state[i * 4 + 1]
                && tile == self.state[i * 4 + 2]
                && tile == self.state[i * 4 + 3]
            {
                self.winner = Some(tile);
                return;
            }
        }

        // check vertical
        for i in 0..4 {
            let tile = self.state[i];
            #[allow(clippy::identity_op)] // allow the '4 * 1' below, to make it more clear
            if tile != Tile::Empty
                && tile == self.state[i + 4 * 1]
                && tile == self.state[i + 4 * 2]
                && tile == self.state[i + 4 * 3]
            {
                self.winner = Some(tile);
                return;
            }
        }

        // check board full, no winner
        if !self.state.contains(&Tile::Empty) {
            // "Empty" winner means "no winner" (see Display impl for Board)
            self.winner = Some(Tile::Empty);
        }
    }
}

impl Default for Board {
    fn default() -> Self {
        let state = [Tile::Empty; 16];
        Self {
            winner: None,
            state,
        }
    }
}

impl Display for Board {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut x = 0_usize;
        for _ in 0..4 {
            writeln!(
                f,
                "⬜{}{}{}{}⬜",
                self.state[x],
                self.state[x + 1],
                self.state[x + 2],
                self.state[x + 3]
            )?;
            x += 4;
        }
        writeln!(f, "⬜⬜⬜⬜⬜⬜")?;
        if let Some(winner) = self.winner {
            match winner {
                // "Empty" winner means "no winner" (board full)
                Tile::Empty => writeln!(f, "No winner.")?,
                tile => writeln!(f, "{} wins!", tile)?,
            }
        }
        Ok(())
    }
}
