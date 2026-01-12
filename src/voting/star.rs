use crate::models::{Poll, Vote};
use crate::voting::{PollResults, VoteCount};
use std::collections::HashMap;

pub fn calculate_results(poll: &Poll, votes: &[Vote]) -> PollResults {
    // Group votes by user and option, storing the highest rating per user per option
    let mut user_option_ratings: HashMap<String, HashMap<String, i32>> = HashMap::new();
    let mut voters = std::collections::HashSet::new();
    let mut option_text: HashMap<String, String> = HashMap::new();

    // Store option text for reference
    for option in &poll.options {
        option_text.insert(option.id.clone(), option.text.clone());
    }

    for vote in votes {
        voters.insert(vote.user_id.clone());
        let user_ratings = user_option_ratings.entry(vote.user_id.clone()).or_default();
        let current_rating = user_ratings.entry(vote.option_id.clone()).or_insert(0);
        *current_rating = (*current_rating).max(vote.rating); // Keep the highest rating if user voted multiple times (shouldn't happen with UI)
    }

    // --- Scoring Phase ---
    let mut option_scores: HashMap<String, i32> = HashMap::new();
    for option in &poll.options {
        option_scores.insert(option.id.clone(), 0);
    }

    for user_ratings in user_option_ratings.values() {
        for (option_id, rating) in user_ratings {
            if let Some(score) = option_scores.get_mut(option_id) {
                *score += rating;
            }
        }
    }

    let mut score_counts: Vec<VoteCount> = option_scores
        .iter()
        .map(|(option_id, score)| VoteCount {
            option_id: option_id.clone(),
            option_text: option_text.get(option_id).cloned().unwrap_or_default(),
            score: *score as f64, // Use score for sorting
            rank: 0,
        })
        .collect();

    // Sort by score (highest first)
    score_counts.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Assign ranks based on score
    for (i, count) in score_counts.iter_mut().enumerate() {
        count.rank = i + 1;
    }

    let mut summary = "**Scoring Phase Results:**\n".to_string();
    for count in &score_counts {
        summary.push_str(&format!(
            "• {}: {} total stars\n",
            count.option_text, count.score
        ));
    }
    summary.push('\n');

    // --- Runoff Phase ---
    if score_counts.len() < 2 {
        // Not enough options for a runoff
        let winner_text = score_counts
            .first()
            .map_or("No winner".to_string(), |c| c.option_text.clone());
        let winner_id = score_counts
            .first()
            .map_or("".to_string(), |c| c.option_id.clone());
        summary.push_str("Not enough options for a runoff.");
        return PollResults {
            winner: winner_text,
            summary,
            winner_id,
            raw_results: score_counts,
        };
    }

    let top_two = &score_counts[0..2];
    let candidate1_id = &top_two[0].option_id;
    let candidate2_id = &top_two[1].option_id;
    let candidate1_text = &top_two[0].option_text;
    let candidate2_text = &top_two[1].option_text;

    summary.push_str(&format!(
        "**Runoff Phase:** Comparing {} vs {}\n",
        candidate1_text, candidate2_text
    ));

    let mut runoff_votes1 = 0;
    let mut runoff_votes2 = 0;
    let mut ties = 0;

    for user_ratings in user_option_ratings.values() {
        let rating1 = user_ratings.get(candidate1_id).copied().unwrap_or(0);
        let rating2 = user_ratings.get(candidate2_id).copied().unwrap_or(0);

        if rating1 > rating2 {
            runoff_votes1 += 1;
        } else if rating2 > rating1 {
            runoff_votes2 += 1;
        } else {
            ties += 1; // Count ties for informational purposes
        }
    }

    summary.push_str(&format!(
        "• {}: {} preferred votes\n",
        candidate1_text, runoff_votes1
    ));
    summary.push_str(&format!(
        "• {}: {} preferred votes\n",
        candidate2_text, runoff_votes2
    ));
    if ties > 0 {
        summary.push_str(&format!("• Tied preference: {} voters\n", ties));
    }
    summary.push('\n');

    let (winner_id, winner_text, winner_score) = if runoff_votes1 >= runoff_votes2 {
        (
            candidate1_id.clone(),
            candidate1_text.clone(),
            runoff_votes1,
        )
    } else {
        (
            candidate2_id.clone(),
            candidate2_text.clone(),
            runoff_votes2,
        )
    };

    summary.push_str(&format!("Total voters: {}", voters.len()));

    PollResults {
        winner: format!(
            "{} ({} preferred votes in runoff)",
            winner_text, winner_score
        ),
        summary,
        winner_id,
        raw_results: score_counts, // Return the scoring phase results as raw
    }
}
