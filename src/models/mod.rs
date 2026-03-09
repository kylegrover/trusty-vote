use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Poll {
    pub id: String,
    pub guild_id: String,
    pub channel_id: String,
    pub creator_id: String,
    pub question: String,
    pub options: Vec<PollOption>,
    pub voting_method: VotingMethod,
    pub created_at: DateTime<Utc>,
    pub ends_at: Option<DateTime<Utc>>,
    pub is_active: bool,
    pub message_id: Option<String>,         // Added message_id
    pub allowed_roles: Option<Vec<String>>, // Restrict voting to these role IDs (if set)
    pub allow_vote_sharing: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PollOption {
    pub id: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VotingMethod {
    Star,
    Plurality,
    Ranked,
    Approval,
}

impl fmt::Display for VotingMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VotingMethod::Star => write!(f, "STAR"),
            VotingMethod::Plurality => write!(f, "Plurality"),
            VotingMethod::Ranked => write!(f, "Ranked Choice"),
            VotingMethod::Approval => write!(f, "Approval"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vote {
    pub user_id: String,
    pub poll_id: String,
    pub option_id: String,
    pub rating: i32,
    pub timestamp: DateTime<Utc>,
}

/// Lightweight poll summary for list views - avoids loading full poll data
#[derive(Debug, Clone)]
pub struct PollSummary {
    pub id: String,
    pub question: String,
    pub ends_at: Option<DateTime<Utc>>,
    pub is_active: bool,
}

impl Poll {
    pub fn new(
        guild_id: String,
        channel_id: String,
        creator_id: String,
        question: String,
        options_text: Vec<String>,
        voting_method: VotingMethod,
        duration_minutes: Option<i64>,
        allowed_roles: Option<Vec<String>>,
        allow_vote_sharing: bool,
    ) -> Self {
        let options = options_text
            .into_iter()
            .map(|text| PollOption {
                id: Uuid::new_v4().to_string(),
                text,
            })
            .collect();

        let created_at = Utc::now();

        // Calculate end time if duration is provided
        let ends_at = match duration_minutes {
            Some(0) => None, // 0 means manual ending
            Some(minutes) => Some(created_at + Duration::minutes(minutes)),
            None => Some(created_at + Duration::days(1)), // Default: 1 day
        };

        Self {
            id: Uuid::new_v4().to_string(),
            guild_id,
            channel_id,
            creator_id,
            question,
            options,
            voting_method,
            created_at,
            ends_at,
            is_active: true,
            message_id: None, // Initialize message_id as None
            allowed_roles,
            allow_vote_sharing,
        }
    }
}

#[cfg(test)]
mod tests;
