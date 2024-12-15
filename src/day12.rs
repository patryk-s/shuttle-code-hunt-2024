use std::{fmt::Display, sync::Arc};

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use rand::{rngs::StdRng, Rng, SeedableRng};
use tokio::sync::RwLock;

const RNG_SEED: u64 = 2024;

struct Data {
    board: Board,
    rng: StdRng,
}

pub fn router() -> Router {
    let board = Board::default();
    let rng = StdRng::seed_from_u64(RNG_SEED);

    let data = Arc::new(RwLock::new(Data { board, rng }));

    Router::new()
        .route("/12/board", get(get_board))
        .route("/12/reset", post(reset_board))
        .route("/12/place/:team/:column", post(place))
        .route("/12/random-board", get(random_board))
        .with_state(data)
}

#[axum::debug_handler]
async fn get_board(State(data): State<Arc<RwLock<Data>>>) -> impl IntoResponse {
    data.read().await.board.to_string()
}

#[axum::debug_handler]
async fn reset_board(State(data): State<Arc<RwLock<Data>>>) -> impl IntoResponse {
    eprintln!("resetting");
    let mut data = data.write_owned().await;
    let rng = StdRng::seed_from_u64(RNG_SEED);
    data.board = Board::default();
    data.rng = rng;
    data.board.to_string()
}

#[axum::debug_handler]
async fn random_board(State(data): State<Arc<RwLock<Data>>>) -> impl IntoResponse {
    let mut data = data.write_owned().await;
    Board::new_random(&mut data.rng).to_string()
}

#[axum::debug_handler]
async fn place(
    State(data): State<Arc<RwLock<Data>>>,
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
    let mut data = data.write_owned().await;
    match data.board.place(team, column as usize) {
        Ok(_) => data.board.to_string().into_response(),
        Err(_) => (StatusCode::SERVICE_UNAVAILABLE, data.board.to_string()).into_response(),
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

    fn new_random(rng: &mut StdRng) -> Self {
        let mut board = Self {
            winner: None,
            state: [Tile::Milk; 16],
        };
        for i in 0..16 {
            if rng.gen() {
                board.state[i] = Tile::Cookie;
            }
        }
        board.check_winner();
        board
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
