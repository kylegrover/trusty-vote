pub mod approval;
pub mod plurality;
pub mod ranked;
pub mod star;

// Generic structure for poll results
pub struct PollResults {
    pub winner: String,              // Name of the winning option
    pub summary: String,             // Detailed results as formatted text
    pub winner_id: String,           // ID of the winning option
    pub raw_results: Vec<VoteCount>, // Raw vote counts for all options
}

// Structure to hold vote counts
#[derive(Debug, Clone)]
pub struct VoteCount {
    pub option_id: String,
    pub option_text: String,
    pub score: f64,
    pub rank: usize,
}

#[cfg(test)]
mod tests;
