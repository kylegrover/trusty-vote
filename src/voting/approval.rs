use crate::models::{Poll, Vote};
use crate::voting::{PollResults, VoteCount};
use startie;
use std::collections::HashMap;

pub fn calculate_results(poll: &Poll, votes: &[Vote]) -> PollResults {
    // Count approvals for each option
    let mut option_approvals: HashMap<String, i32> = HashMap::new();
    let mut option_text: HashMap<String, String> = HashMap::new();
    let mut voters = std::collections::HashSet::new();

    // Initialize counts to 0
    for option in &poll.options {
        option_approvals.insert(option.id.clone(), 0);
        option_text.insert(option.id.clone(), option.text.clone());
    }

    // Count approvals (in approval voting, a vote of 1 means approved)
    for vote in votes {
        if vote.rating == 1
            && let Some(count) = option_approvals.get_mut(&vote.option_id)
        {
            *count += 1;
        }
        voters.insert(vote.user_id.clone());
    }

    // Build deterministic tiebreak ordering
    let tiebreak_scores: Vec<(String, i64)> = option_approvals
        .iter()
        .map(|(id, approvals)| {
            let text = option_text.get(id).cloned().unwrap_or_default();
            (text, *approvals as i64)
        })
        .collect();
    let tiebreak_order = startie::permute(&tiebreak_scores, b"");
    let tiebreak_rank: HashMap<String, usize> = tiebreak_order
        .iter()
        .enumerate()
        .map(|(i, name)| (name.clone(), i))
        .collect();

    // Build vote counts
    let mut vote_counts: Vec<VoteCount> = option_approvals
        .iter()
        .map(|(option_id, approvals)| {
            VoteCount {
                option_id: option_id.clone(),
                option_text: option_text.get(option_id).cloned().unwrap_or_default(),
                score: *approvals as f64,
                rank: 0, // Will set this after sorting
            }
        })
        .collect();

    // Sort by score (highest first), tiebreak by startie permutation order
    vote_counts.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                let ra = tiebreak_rank
                    .get(&a.option_text)
                    .copied()
                    .unwrap_or(usize::MAX);
                let rb = tiebreak_rank
                    .get(&b.option_text)
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
        let winner_approvals = vote_counts[0].score as i32;

        // Build summary text
        let mut summary = String::new();

        for count in &vote_counts {
            let percentage = if !voters.is_empty() {
                (count.score / voters.len() as f64) * 100.0
            } else {
                0.0
            };

            summary.push_str(&format!(
                "{}: {} approvals ({:.1}%)\n",
                count.option_text, count.score, percentage
            ));
        }

        summary.push_str(&format!("\nTotal voters: {}", voters.len()));

        PollResults {
            winner: format!("{} ({} approvals)", winner_text, winner_approvals),
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
