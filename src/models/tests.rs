use crate::models::{Poll, VotingMethod};

#[test]
fn poll_defaults_to_one_day_duration() {
    let poll = Poll::new(
        "guild".to_string(),
        "channel".to_string(),
        "creator".to_string(),
        "Question".to_string(),
        vec!["One".to_string(), "Two".to_string()],
        VotingMethod::Approval,
        None,
        None,
        false,
    );

    let ends_at = poll.ends_at.expect("default poll should have an end time");
    let duration = ends_at - poll.created_at;

    assert_eq!(duration.num_days(), 1);
}

#[test]
fn poll_supports_manual_end_when_duration_is_zero() {
    let poll = Poll::new(
        "guild".to_string(),
        "channel".to_string(),
        "creator".to_string(),
        "Question".to_string(),
        vec!["One".to_string(), "Two".to_string()],
        VotingMethod::Star,
        Some(0),
        None,
        false,
    );

    assert!(poll.ends_at.is_none());
}
