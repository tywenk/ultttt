use std::{
    array,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

use anyhow::{bail, ensure, Error, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Type;
use uuid::Uuid;

use crate::{model::MatchModel, AppState};

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
const DEFAULT_TEAM: Team = Team::X;

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
                snap[i][j] = cell.load(Ordering::Acquire);
            }
        }
        snap
    }

    pub fn reset(&self) {
        for row in self.snap.iter() {
            for cell in row.iter() {
                cell.store(0, Ordering::SeqCst);
            }
        }
    }

    pub fn increment(&self, row: usize, col: usize) -> usize {
        self.snap[row][col].fetch_add(1, Ordering::AcqRel)
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

    pub fn is_empty(&self) -> bool {
        self.load()
            .iter()
            .all(|row| row.iter().all(|&cell| cell == 0))
    }

    pub fn validate_move(
        &self,
        state: Arc<AppState>,
        section: usize,
        cell: usize,
        team: Team,
    ) -> Result<()> {
        let curr_match = state.match_schema.load();

        ensure!(section < 9 && cell < 9, "Invalid row or column index");
        ensure!(
            curr_match.board.current_team == team,
            "Invalid team according to match state"
        );

        let board_is_interactive = curr_match.board.is_interactive();
        ensure!(board_is_interactive, "Board is not interactive");

        let section_is_interactive = curr_match.board.data[section].is_interactive();
        ensure!(section_is_interactive, "Section is not interactive");

        let cell_is_interactive = curr_match.board.data[section].data[cell].is_interactive();
        ensure!(cell_is_interactive, "Cell is not interactive");

        Ok(())
    }
}

pub trait ValidateInteractive {
    fn is_interactive(&self) -> bool;
}

pub trait GameStatusEvaluator {
    fn calculate_status(&self, team: Team) -> Status;
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, Type)]
pub struct Cell {
    pub status: Status,
}

impl ValidateInteractive for Cell {
    fn is_interactive(&self) -> bool {
        self.status == Status::Pending
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, Type)]
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
    fn calculate_status(&self, team: Team) -> Status {
        let is_team_won = WINNING_SETS.iter().any(|set| {
            set.iter()
                .all(|&index| self.data[index].status == team.into())
        });

        if is_team_won {
            return team.into();
        }

        if self.data.iter().all(|cell| cell.status != Status::Pending) {
            return Status::Tied;
        }

        return Status::Pending;
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, Type)]
pub struct Board {
    pub data: [Section; 9],
    pub status: Status,
    pub current_team: Team,
}

impl ValidateInteractive for Board {
    fn is_interactive(&self) -> bool {
        self.status == Status::Pending
    }
}

impl GameStatusEvaluator for Board {
    fn calculate_status(&self, team: Team) -> Status {
        let is_team_won = WINNING_SETS.iter().any(|set| {
            set.iter()
                .all(|&index| self.data[index].status == team.into())
        });

        if is_team_won {
            return team.into();
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
            current_team: Team::default(),
        }
    }

    fn validate_move(&self, coord: (usize, usize), team: Team) -> Result<()> {
        if team != self.current_team {
            bail!("Not this team's turn");
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
        let team = self.current_team;
        self.validate_move(coord, team)?;

        let sec_index = coord.0;
        let cell_index = coord.1;

        let mut new_board = self.clone();
        let sec = &mut new_board.data[sec_index];
        let cell = &mut sec.data[cell_index];

        cell.status = team.into();
        sec.status = sec.calculate_status(team);

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

        new_board.status = new_board.calculate_status(team);
        new_board.current_team = team.toggle();

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

impl From<Team> for Status {
    fn from(value: Team) -> Self {
        match value {
            Team::X => Status::X,
            Team::O => Status::O,
        }
    }
}

#[derive(Clone, Copy, Serialize, Deserialize, Debug, PartialEq, Type)]
#[serde(rename_all = "lowercase")]
#[sqlx(rename_all = "lowercase")]
pub enum Team {
    X,
    O,
}

impl TryFrom<Status> for Team {
    type Error = &'static str;
    fn try_from(value: Status) -> Result<Self, Self::Error> {
        match value {
            Status::X => Ok(Team::X),
            Status::O => Ok(Team::O),
            _ => Err("Variant not available in Team"),
        }
    }
}

impl Default for Team {
    fn default() -> Self {
        DEFAULT_TEAM
    }
}

impl Team {
    pub fn toggle(&self) -> Team {
        match self {
            Team::X => Team::O,
            Team::O => Team::X,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateMatchSchema {
    pub state: Status,
    pub board: Board,
}

// For json response
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub struct MatchSchema {
    pub id: Uuid,
    pub board: Board,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl TryFrom<&MatchModel> for MatchSchema {
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

#[derive(Debug, Deserialize)]
pub struct IncrementRequest {
    pub section: usize,
    pub cell: usize,
}

#[derive(Clone, Serialize, Debug)]
pub struct SnapshotResponse {
    // Team is not none only on new connection. This tells
    // the client what team they are on
    pub your_team: Option<Team>,
    pub snap: [[usize; 9]; 9],
}

#[derive(Clone, Serialize)]
pub struct TeamsResponse {
    pub x_team_size: usize,
    pub o_team_size: usize,
}

use dashmap::DashSet;

#[derive(Debug)]
pub struct TeamConnection {
    pub team: Team,
    pub id: Uuid, // Unique identifier for each connection
}

#[derive(Debug)]
pub struct Teams {
    pub team_x: DashSet<Uuid>,
    pub team_o: DashSet<Uuid>,
}

impl Teams {
    pub fn new() -> Self {
        Self {
            team_x: DashSet::new(),
            team_o: DashSet::new(),
        }
    }

    // Assign a new client to a team
    pub fn assign_team(&self) -> TeamConnection {
        let id = Uuid::new_v4();
        let x_count = self.team_x.len();
        let o_count = self.team_o.len();

        // Balance the teams on new connection
        if x_count <= o_count {
            // Default team is X
            self.team_x.insert(id);
            TeamConnection {
                team: Team::default(),
                id,
            }
        } else {
            self.team_o.insert(id);
            TeamConnection {
                team: Team::default().toggle(),
                id,
            }
        }
    }

    pub fn remove_connection(&self, connection: &TeamConnection) {
        match connection.team {
            Team::X => self.team_x.remove(&connection.id),
            Team::O => self.team_o.remove(&connection.id),
        };
    }

    pub fn team_lens(&self) -> (usize, usize) {
        let x_count = self.team_x.len();
        let o_count = self.team_o.len();
        (x_count, o_count)
    }
}
