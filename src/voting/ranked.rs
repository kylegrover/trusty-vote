use crate::models::{Poll, Vote};
use crate::voting::{PollResults, VoteCount};
use log::error;
use std::collections::{HashMap, HashSet};

pub fn calculate_results(poll: &Poll, votes: &[Vote]) -> PollResults {
    // Group votes by user, storing their ranking for each option
    let mut user_rankings: HashMap<String, HashMap<String, i32>> = HashMap::new();
    let mut option_text: HashMap<String, String> = HashMap::new();
    let mut voters = HashSet::new(); // Track unique voters

    // Store option text for reference
    for option in &poll.options {
        option_text.insert(option.id.clone(), option.text.clone());
    }

    // Collect all rankings, ensuring only ranked options (rating > 0) are stored
    for vote in votes {
        voters.insert(vote.user_id.clone()); // Track voter
        if vote.rating > 0 {
            user_rankings
                .entry(vote.user_id.clone())
                .or_default()
                .insert(vote.option_id.clone(), vote.rating);
        }
    }

    // If no valid rankings, return early
    if user_rankings.is_empty() {
        return PollResults {
            winner: "No winner".to_string(),
            summary: "No valid rankings were submitted.".to_string(),
        };
    }

    // Calculate results using instant-runoff voting
    let mut eliminated: HashSet<String> = HashSet::new();
    let total_voters = voters.len(); // Use the count of unique voters
    let majority_threshold = (total_voters as f64 / 2.0).floor() + 1.0; // Votes needed for majority
    let mut round = 1;
    let mut summary = String::new();
    #[allow(unused_assignments)]
    let mut final_results: Vec<VoteCount> = Vec::new();

    loop {
        summary.push_str(&format!("**Round {}**\n", round));

        // Count first preferences for each candidate that hasn't been eliminated
        let mut first_preferences: HashMap<String, i32> = HashMap::new();
        for option_id in option_text.keys() {
            if !eliminated.contains(option_id) {
                first_preferences.insert(option_id.clone(), 0);
            }
        }

        // For each voter, find their highest-ranked non-eliminated candidate
        for user_votes in user_rankings.values() {
            let mut best_option: Option<String> = None;
            let mut best_rank = i32::MAX;

            // Find the highest ranked option (lowest rank number) that hasn't been eliminated
            for (option_id, rank) in user_votes {
                if !eliminated.contains(option_id) && *rank < best_rank {
                    best_rank = *rank;
                    best_option = Some(option_id.clone());
                }
            }

            // Count this as a first preference
            if let Some(option_id) = best_option
                && let Some(count) = first_preferences.get_mut(&option_id)
            {
                *count += 1;
            }
        }

        // Build vote counts for this round
        let mut round_counts: Vec<VoteCount> = first_preferences
            .iter()
            .map(|(option_id, count)| VoteCount {
                option_id: option_id.clone(),
                option_text: option_text.get(option_id).cloned().unwrap_or_default(),
                score: *count as f64, // Score is the number of first preferences
                rank: 0,              // Will set after sorting
            })
            .collect();

        // Sort by score (highest first)
        round_counts.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Assign ranks for this round
        for (i, count) in round_counts.iter_mut().enumerate() {
            count.rank = i + 1;
        }

        // Add round results to summary
        for count in &round_counts {
            let percentage = if total_voters > 0 {
                (count.score / total_voters as f64) * 100.0
            } else {
                0.0
            };
            summary.push_str(&format!(
                "• {}: {:.0} votes ({:.1}%)\n",
                count.option_text, count.score, percentage
            ));
        }
        summary.push('\n');

        // Check if we have a majority winner
        if !round_counts.is_empty() && round_counts[0].score >= majority_threshold {
            let winner = &round_counts[0];
            summary.push_str(&format!("{} has reached a majority!", winner.option_text));
            final_results = round_counts; // Store this round's results
            break; // Winner found
        }

        // Check for ties or only one candidate left
        if round_counts.len() <= 1 {
            let winner_text = round_counts
                .first()
                .map_or("No winner (tie or no remaining options)".to_string(), |c| {
                    c.option_text.clone()
                });
            summary.push_str(&format!("{} wins (last remaining).", winner_text));
            final_results = round_counts; // Store this round's results
            break; // End condition met
        }

        // Check for unbreakable tie among all remaining candidates
        let min_score = round_counts.last().map_or(0.0, |c| c.score);
        if round_counts.iter().all(|c| c.score == min_score) {
            summary.push_str("Unbreakable tie among remaining candidates.");
            final_results = round_counts; // Store this round's results
            break; // Tie condition
        }

        // Eliminate the lowest-ranked candidate(s) with the minimum score
        let mut eliminated_this_round_text = Vec::new();
        let candidates_to_eliminate: Vec<String> = round_counts
            .iter()
            .filter(|c| c.score == min_score)
            .map(|c| c.option_id.clone())
            .collect();

        for option_id in candidates_to_eliminate {
            if let Some(text) = option_text.get(&option_id) {
                eliminated_this_round_text.push(text.clone());
            }
            eliminated.insert(option_id);
        }

        summary.push_str(&format!(
            "Eliminating: {}\n\n",
            eliminated_this_round_text.join(", ")
        ));

        round += 1;

        // Safety break to prevent infinite loops in unexpected scenarios
        if round > poll.options.len() + 5 {
            // Allow a few extra rounds just in case
            error!(
                "Ranked choice calculation exceeded expected rounds for poll {}",
                poll.id
            );
            summary.push_str("Calculation stopped due to excessive rounds.");
            final_results = round_counts; // Store current state
            break;
        }
    }

    // Determine final winner text
    let winner_text = if !final_results.is_empty() && final_results[0].score >= majority_threshold {
        format!(
            "{} ({:.0} votes)",
            final_results[0].option_text, final_results[0].score
        )
    } else if final_results.len() == 1 {
        format!("{} (last remaining)", final_results[0].option_text)
    } else if !final_results.is_empty()
        && final_results
            .iter()
            .all(|c| c.score == final_results[0].score)
    {
        "Tie".to_string() // Indicate a tie if multiple winners have same score
    } else if !final_results.is_empty() {
        // If no majority but someone has highest score (e.g. due to exhaustion)
        format!("{} (most votes)", final_results[0].option_text)
    } else {
        "No clear winner".to_string()
    };

    PollResults {
        winner: winner_text,
        summary,
    }
}
