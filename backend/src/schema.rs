use std::{
    array,
    sync::atomic::{AtomicUsize, Ordering},
};

use anyhow::{bail, Error, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Type;
use uuid::Uuid;

use crate::model::MatchModel;

const WINNING_SETS: [[usize; 3]; 8] = [
    [0, 1, 2],
    [3, 4, 5],
    [6, 7, 8],
    [0, 3, 6],
    [1, 4, 7],
    [2, 5, 8],
    [0, 4, 8],
    [2, 4, 6],
];
const DEFAULT_PLAYER: Player = Player::X;

// #[derive(Clone, Serialize, Deserialize, Debug)]
// #[serde(rename_all = "lowercase")]
// pub enum SnapshotStatus {
//     New,
//     Pending,
// }

pub struct Snapshot {
    pub snap: [[AtomicUsize; 9]; 9],
}

impl Snapshot {
    pub fn new() -> Self {
        // Initialize all elements with AtomicUsize::new(0)
        let mut snap: [[AtomicUsize; 9]; 9] = Default::default();
        // We have to iterate to implement copy trait
        for sec in snap.iter_mut() {
            for cell in sec.iter_mut() {
                *cell = AtomicUsize::new(0);
            }
        }

        Self { snap }
    }

    pub fn load(&self) -> [[usize; 9]; 9] {
        let mut snap: [[usize; 9]; 9] = Default::default();
        for (i, row) in self.snap.iter().enumerate() {
            for (j, cell) in row.iter().enumerate() {
                snap[i][j] = cell.load(Ordering::Relaxed);
            }
        }
        snap
    }

    pub fn reset(&self) {
        for row in self.snap.iter() {
            for cell in row.iter() {
                cell.store(0, Ordering::Relaxed);
            }
        }
    }

    pub fn increment(&self, row: usize, col: usize) -> usize {
        self.snap[row][col].fetch_add(1, Ordering::Relaxed)
    }

    pub fn find_max_indices(&self) -> Option<(usize, usize)> {
        let arr = self.load();

        // If the arr is all zeros, return None
        if arr.iter().all(|row| row.iter().all(|&cell| cell == 0)) {
            return None;
        }

        let mut max = 0;
        let mut max_indices = (0, 0);

        for (i, subarr) in arr.iter().enumerate() {
            for (j, &value) in subarr.iter().enumerate() {
                if value > max {
                    max = value;
                    max_indices = (i, j);
                }
            }
        }

        Some(max_indices)
    }
}

pub trait ValidateInteractive {
    fn is_interactive(&self) -> bool;
}

pub trait GameStatusEvaluator {
    fn calculate_status(&self, player: Player) -> Status;
}

#[derive(Clone, Debug, Deserialize, Serialize, Type)]
pub struct Cell {
    pub status: Status,
}

impl ValidateInteractive for Cell {
    fn is_interactive(&self) -> bool {
        self.status == Status::Pending
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, Type)]
pub struct Section {
    pub data: [Cell; 9],
    pub status: Status,
    pub is_interactive: bool,
}

impl ValidateInteractive for Section {
    fn is_interactive(&self) -> bool {
        self.status == Status::Pending && self.is_interactive
    }
}

impl GameStatusEvaluator for Section {
    fn calculate_status(&self, player: Player) -> Status {
        let is_player_won = WINNING_SETS.iter().any(|set| {
            set.iter()
                .all(|&index| self.data[index].status == player.into())
        });

        if is_player_won {
            return player.into();
        }

        if self.data.iter().all(|cell| cell.status != Status::Pending) {
            return Status::Tied;
        }

        return Status::Pending;
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, Type)]
pub struct Board {
    pub data: [Section; 9],
    pub status: Status,
    pub current_player: Player,
}

impl ValidateInteractive for Board {
    fn is_interactive(&self) -> bool {
        self.status == Status::Pending
    }
}

impl GameStatusEvaluator for Board {
    fn calculate_status(&self, player: Player) -> Status {
        let is_player_won = WINNING_SETS.iter().any(|set| {
            set.iter()
                .all(|&index| self.data[index].status == player.into())
        });

        if is_player_won {
            return player.into();
        }

        if self.data.iter().all(|sec| sec.status != Status::Pending) {
            return Status::Tied;
        }

        return Status::Pending;
    }
}

impl Board {
    pub fn new() -> Board {
        Board {
            data: array::from_fn(|_| Section {
                data: array::from_fn(|_| Cell {
                    status: Status::Pending,
                }),
                status: Status::Pending,
                is_interactive: true,
            }),
            status: Status::Pending,
            current_player: Player::default(),
        }
    }

    fn validate_move(&self, coord: (usize, usize), player: Player) -> Result<()> {
        if player != self.current_player {
            bail!("Invalid player");
        }

        if !self.is_interactive() {
            bail!("Board is not interactive");
        }

        let sec = &self.data[coord.0];
        if !sec.is_interactive() {
            bail!("Section is not interactive");
        }

        let cell = &sec.data[coord.1];
        if !cell.is_interactive() {
            bail!("Cell is not interactive");
        }

        Ok(())
    }

    pub fn get_updated(&self, coord: (usize, usize)) -> Result<Self> {
        let player = self.current_player;
        self.validate_move(coord, player)?;

        let sec_index = coord.0;
        let cell_index = coord.1;

        let mut new_board = self.clone();
        let sec = &mut new_board.data[sec_index];
        let cell = &mut sec.data[cell_index];

        cell.status = player.into();
        sec.status = sec.calculate_status(player);

        // Normalize interactivity
        for sec in new_board.data.iter_mut() {
            sec.is_interactive = false;
        }
        // Locate the next interactive section
        let next_sec = &mut new_board.data[cell_index];
        if next_sec.status == Status::Pending {
            next_sec.is_interactive = true;
        } else {
            for sec in new_board.data.iter_mut() {
                sec.is_interactive = sec.status == Status::Pending;
            }
        }

        new_board.status = new_board.calculate_status(player);
        new_board.current_player = player.toggle();

        Ok(new_board)
    }
}

#[derive(Deserialize, Debug, Default)]
pub struct Pagination {
    pub offset: Option<usize>,
    pub limit: Option<usize>,
}

#[derive(Deserialize, Debug)]
pub struct Coords {
    pub section: usize,
    pub cell: usize,
}

#[derive(Clone, Copy, Serialize, Deserialize, Debug, PartialEq, Type)]
#[serde(rename_all = "lowercase")]
#[sqlx(type_name = "status", rename_all = "lowercase")]
pub enum Status {
    X,
    O,
    Tied,
    Pending,
}

impl TryFrom<String> for Status {
    type Error = Error;
    fn try_from(s: String) -> Result<Self> {
        Status::try_from(s.as_str())
    }
}

impl TryFrom<&str> for Status {
    type Error = Error;
    fn try_from(s: &str) -> Result<Self> {
        match s {
            "x" => Ok(Status::X),
            "o" => Ok(Status::O),
            "tied" => Ok(Status::Tied),
            "pending" => Ok(Status::Pending),
            _ => bail!("Invalid status: {}", s),
        }
    }
}

impl From<Player> for Status {
    fn from(value: Player) -> Self {
        match value {
            Player::X => Status::X,
            Player::O => Status::O,
        }
    }
}

#[derive(Clone, Copy, Serialize, Deserialize, Debug, PartialEq, Type)]
#[serde(rename_all = "lowercase")]
#[sqlx(rename_all = "lowercase")]
pub enum Player {
    X,
    O,
}

impl TryFrom<Status> for Player {
    type Error = &'static str;
    fn try_from(value: Status) -> Result<Self, Self::Error> {
        match value {
            Status::X => Ok(Player::X),
            Status::O => Ok(Player::O),
            _ => Err("Variant not available in Player"),
        }
    }
}

impl Default for Player {
    fn default() -> Self {
        DEFAULT_PLAYER
    }
}

impl Player {
    pub fn toggle(&self) -> Player {
        match self {
            Player::X => Player::O,
            Player::O => Player::X,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateMatchSchema {
    pub state: Status,
    pub board: Board,
}

// For json response
#[derive(Debug, Deserialize, Serialize)]
pub struct GetMatchSchema {
    pub id: Uuid,
    pub board: Board,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl TryFrom<&MatchModel> for GetMatchSchema {
    type Error = Error;
    fn try_from(m: &MatchModel) -> Result<Self, Self::Error> {
        Ok(Self {
            id: m.id,
            board: serde_json::from_value::<Board>(m.board.0.clone())?,
            created_at: m.created_at,
            updated_at: m.updated_at,
        })
    }
}

#[derive(Deserialize)]
pub struct IncrementRequest {
    pub section: usize,
    pub cell: usize,
}

#[derive(Clone, Serialize)]
pub struct SnapshotResponse {
    pub snap: [[usize; 9]; 9],
}
