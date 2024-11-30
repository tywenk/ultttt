use anyhow::{anyhow, Error, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Type;
use uuid::Uuid;

use crate::model::MatchModel;

pub type Snapshot = [[u8; 9]; 9];

#[derive(Deserialize, Debug, Default)]
pub struct Pagination {
    pub offset: Option<usize>,
    pub limit: Option<usize>,
}

#[derive(Clone, Serialize, Deserialize, Debug, Type)]
#[serde(rename_all = "lowercase")]
#[sqlx(type_name = "status", rename_all = "lowercase")]
pub enum Status {
    X,
    O,
    Tied,
    Pending,
}

impl TryFrom<&str> for Status {
    type Error = Error;

    fn try_from(s: &str) -> Result<Self> {
        match s {
            "x" => Ok(Status::X),
            "o" => Ok(Status::O),
            "tied" => Ok(Status::Tied),
            "pending" => Ok(Status::Pending),
            _ => Err(anyhow!("Invalid status: {}", s)),
        }
    }
}

// For String
impl TryFrom<String> for Status {
    type Error = Error;

    fn try_from(s: String) -> Result<Self> {
        Status::try_from(s.as_str())
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateMatchSchema {
    pub state: Status,
    pub snapshot: Snapshot,
}

// For json response
#[derive(Debug, Deserialize, Serialize)]
pub struct GetMatchSchema {
    pub id: Uuid,
    pub state: Status,
    pub snapshot: Snapshot,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl TryFrom<&MatchModel> for GetMatchSchema {
    type Error = Error;
    fn try_from(m: &MatchModel) -> Result<Self, Self::Error> {
        let snapshot = match &m.snapshot {
            Some(s) => serde_json::from_value(s.0.clone())?,
            None => return Err(anyhow::anyhow!("No snapshot found")),
        };
        Ok(Self {
            id: m.id,
            state: m.state.clone(),
            snapshot,
            created_at: m.created_at,
            updated_at: m.updated_at,
        })
    }
}
