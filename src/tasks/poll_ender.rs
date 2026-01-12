use crate::commands::poll::end_poll_logic; // Import the refactored logic
use crate::db::Database;
use chrono::Utc;
use log::{error, info};
use serenity::prelude::*;
use std::sync::Arc;
use std::time::Duration as StdDuration;
use tokio::time::interval;

const CHECK_INTERVAL_SECONDS: u64 = 60; // Check every 60 seconds

pub async fn check_expired_polls_task(database: Arc<Database>, ctx: Context) {
    info!("Starting background task to check for expired polls...");
    let mut interval = interval(StdDuration::from_secs(CHECK_INTERVAL_SECONDS));

    loop {
        interval.tick().await; // Wait for the next interval tick
        let now = Utc::now();
        info!("Checking for expired polls at {}", now.to_rfc3339());

        match database.get_expired_polls(now).await {
            Ok(expired_polls) => {
                if !expired_polls.is_empty() {
                    info!("Found {} expired poll(s).", expired_polls.len());
                    for (poll_id, channel_id, message_id_opt) in expired_polls {
                        info!("Processing expired poll: {}", poll_id);
                        // Clone Arcs/Context for the spawned task
                        let db_clone = Arc::clone(&database);
                        let ctx_clone = ctx.clone();
                        let poll_id_clone = poll_id.clone();
                        let channel_id_clone = channel_id.clone();
                        let message_id_clone = message_id_opt.clone();

                        // Spawn a separate task for each poll to avoid blocking the loop
                        tokio::spawn(async move {
                            match end_poll_logic(
                                &db_clone,
                                &ctx_clone,
                                &poll_id_clone,
                                &channel_id_clone,
                                message_id_clone,
                            )
                            .await
                            {
                                Ok(_) => {
                                    info!("Successfully processed expired poll {}", poll_id_clone)
                                }
                                Err(e) => {
                                    error!("Error processing expired poll {}: {}", poll_id_clone, e)
                                }
                            }
                        });
                    }
                } else {
                    // info!("No expired polls found."); // Optional: reduce log noise
                }
            }
            Err(e) => {
                error!("Failed to query for expired polls: {}", e);
            }
        }
    }
}
