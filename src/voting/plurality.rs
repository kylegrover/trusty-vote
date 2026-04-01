use crate::models::{Poll, Vote};
use startie;
use crate::voting::{PollResults, VoteCount};
use std::collections::HashMap;

pub fn calculate_results(poll: &Poll, votes: &[Vote]) -> PollResults {
    // Count votes for each option
    let mut option_votes: HashMap<String, i32> = HashMap::new();
    let mut option_text: HashMap<String, String> = HashMap::new();
    let mut voters = std::collections::HashSet::new();

    // Initialize counts to 0
    for option in &poll.options {
        option_votes.insert(option.id.clone(), 0);
        option_text.insert(option.id.clone(), option.text.clone());
    }

    // Count votes (in plurality, a vote is a rating of 1)
    for vote in votes {
        if vote.rating == 1 {
            if let Some(count) = option_votes.get_mut(&vote.option_id) {
                *count += 1;
            }
            voters.insert(vote.user_id.clone());
        }
    }

    // Build deterministic tiebreak ordering. Use option IDs as unique keys to avoid text-collisions.
    let tiebreak_scores: Vec<(String, i64)> = option_votes
        .iter()
        .map(|(id, score)| (id.clone(), *score as i64))
        .collect();
    let tiebreak_order = startie::permute(&tiebreak_scores, b"");
    let tiebreak_rank: HashMap<String, usize> = tiebreak_order
        .iter()
        .enumerate()
        .map(|(i, option_id)| (option_id.clone(), i))
        .collect();

    // Build vote counts
    let mut vote_counts: Vec<VoteCount> = option_votes
        .iter()
        .map(|(option_id, votes)| VoteCount {
            option_id: option_id.clone(),
            option_text: option_text.get(option_id).cloned().unwrap_or_default(),
            score: *votes as f64,
            rank: 0,
        })
        .collect();

    // Sort by score (highest first), tiebreak by startie permutation order
    vote_counts.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                let ra = tiebreak_rank
                    .get(&a.option_id)
                    .copied()
                    .unwrap_or(usize::MAX);
                let rb = tiebreak_rank
                    .get(&b.option_id)
                    .copied()
                    .unwrap_or(usize::MAX);
                ra.cmp(&rb)
            })
    });

    // Assign ranks
    for (i, count) in vote_counts.iter_mut().enumerate() {
        count.rank = i + 1;
    }

    // Determine winner
    if !vote_counts.is_empty() && vote_counts[0].score > 0.0 {
        let winner_text = vote_counts[0].option_text.clone();
        let winner_votes = vote_counts[0].score as i32;

        // Build summary text
        let mut summary = String::new();

        for count in &vote_counts {
            let percentage = if !voters.is_empty() {
                (count.score / voters.len() as f64) * 100.0
            } else {
                0.0
            };

            summary.push_str(&format!(
                "{}: {} votes ({:.1}%)\n",
                count.option_text, count.score, percentage
            ));
        }

        summary.push_str(&format!("\nTotal voters: {}", voters.len()));

        PollResults {
            winner: format!("{} ({} votes)", winner_text, winner_votes),
            summary,
        }
    } else {
        // No votes cast
        PollResults {
            winner: "No winner".to_string(),
            summary: "No votes were cast.".to_string(),
        }
    }
}
