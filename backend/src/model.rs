use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::types::Json;
use uuid::Uuid;

use crate::schema::Status;

// For sqlx
#[derive(Debug, Deserialize, Serialize, sqlx::FromRow)]
pub struct MatchModel {
    pub id: Uuid,
    pub state: Status,
    pub board: Json<Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
