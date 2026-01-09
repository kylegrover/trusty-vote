mod vote;

use crate::db::Database;
use crate::models::Poll;
use log::{error, info, warn};
use serenity::model::application::component::ButtonStyle;
use serenity::model::application::interaction::application_command::ApplicationCommandInteraction;
use serenity::model::application::interaction::message_component::MessageComponentInteraction;
use serenity::model::application::interaction::{Interaction, InteractionResponseType};
use serenity::prelude::*;

// Handle slash commands
pub async fn handle_command(
    database: &Database,
    ctx: &Context,
    command: &ApplicationCommandInteraction,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!("Received command: {}", command.data.name);
    match command.data.name.as_str() {
        "poll" => crate::commands::poll::handle_poll_command(database, ctx, command).await?,
        _ => {
            command
                .create_interaction_response(&ctx.http, |response| {
                    response
                        .kind(InteractionResponseType::ChannelMessageWithSource)
                        .interaction_response_data(|message| {
                            message.content("Unknown command").ephemeral(true)
                        })
                })
                .await?;
        }
    }
    Ok(())
}

// Helper function to parse poll_id from custom_id
// Simplified to handle only the new camelCase format
fn parse_poll_id_from_custom_id(custom_id: &str) -> Option<String> {
    // Special case for vote buttons (both old and new format)
    if custom_id == "vote_button" || custom_id == "voteButton" {
        return None;
    }

    // Special cases with direct poll ID extraction
    if custom_id.starts_with("doneVoting_") {
        return Some(custom_id.replace("doneVoting_", ""));
    } else if custom_id.starts_with("voteChange_") {
        return Some(custom_id.replace("voteChange_", ""));
    }

    // Standard camelCase format with underscore separating action and poll ID
    // Example: "starSelect_pollID_optionID"
    let parts: Vec<&str> = custom_id.split('_').collect();
    if parts.len() >= 2 {
        // All camelCase component IDs have poll ID as the second part (index 1)
        return parts.get(1).map(|s| s.to_string());
    }

    None
}

// Main component handler
pub async fn handle_component(
    database: &Database,
    ctx: &Context,
    component: &MessageComponentInteraction,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let custom_id = &component.data.custom_id;
    info!("Received component interaction: {}", custom_id);

    // Handle selection menus that don't have poll_id in custom_id
    if custom_id == "selectEndPoll" {
        if let Some(poll_id) = component.data.values.get(0) {
            // We need to fetch the poll to get channel_id and message_id
            match database.get_poll(poll_id).await {
                Ok(poll) => {
                    component
                        .create_interaction_response(&ctx.http, |response| {
                            response.kind(InteractionResponseType::DeferredUpdateMessage)
                        })
                        .await?;

                    match crate::commands::poll::end_poll_logic(
                        database,
                        ctx,
                        poll_id,
                        &poll.channel_id,
                        poll.message_id,
                    )
                    .await
                    {
                        Ok(_) => {
                            component
                                .edit_original_interaction_response(&ctx.http, |response| {
                                    response
                                        .content(format!(
                                            "Poll '{}' ended successfully.",
                                            poll.question
                                        ))
                                        .components(|c| c)
                                })
                                .await?;
                        }
                        Err(e) => {
                            error!("Error ending poll {} via selection: {}", poll_id, e);
                            component
                                .edit_original_interaction_response(&ctx.http, |response| {
                                    response
                                        .content(format!("Failed to end poll: {}", e))
                                        .components(|c| c)
                                })
                                .await?;
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to fetch poll {} for ending: {}", poll_id, e);
                    component
                        .create_interaction_response(&ctx.http, |response| {
                            response
                                .kind(InteractionResponseType::ChannelMessageWithSource)
                                .interaction_response_data(|msg| {
                                    msg.content("Failed to fetch poll details.").ephemeral(true)
                                })
                        })
                        .await?;
                }
            }
        }
        return Ok(());
    } else if custom_id == "selectResultsPoll" {
        if let Some(poll_id) = component.data.values.get(0) {
            match database.get_poll(poll_id).await {
                Ok(poll) => {
                    let votes = database.get_poll_votes(poll_id).await?;
                    let results = crate::commands::poll::calculate_poll_results(&poll, &votes);

                    component
                        .create_interaction_response(&ctx.http, |response| {
                            response
                                .kind(InteractionResponseType::UpdateMessage)
                                .interaction_response_data(|message| {
                                    message
                                        .content("") // Clear the "Select a poll..." text
                                        .embed(|e| {
                                            crate::commands::poll::create_results_embed(
                                                e, &poll, &results,
                                            )
                                        })
                                        .components(|c| {
                                            c.create_action_row(|row| {
                                                row.create_button(|btn| {
                                                    btn.custom_id(format!(
                                                        "shareResults_{}",
                                                        poll.id
                                                    ))
                                                    .label("Share Results")
                                                    .style(ButtonStyle::Primary)
                                                })
                                            })
                                        })
                                })
                        })
                        .await?;
                }
                Err(e) => {
                    error!("Failed to fetch poll {} for results: {}", poll_id, e);
                    component
                        .create_interaction_response(&ctx.http, |response| {
                            response
                                .kind(InteractionResponseType::ChannelMessageWithSource)
                                .interaction_response_data(|msg| {
                                    msg.content("Failed to fetch poll details.").ephemeral(true)
                                })
                        })
                        .await?;
                }
            }
        }
        return Ok(());
    }

    // Extract poll_id from the component custom_id
    let poll_id_opt: Option<String> = if custom_id == "vote_button" || custom_id == "voteButton" {
        component.message.embeds.get(0).and_then(|embed| {
            embed
                .fields
                .iter()
                .find(|field| field.name == "Poll ID")
                .map(|field| field.value.clone())
        })
    } else {
        let poll_id = parse_poll_id_from_custom_id(custom_id);
        if poll_id.is_none() {
            warn!("Could not parse poll ID from custom_id: {}", custom_id);
        } else {
            info!(
                "Parsed poll ID '{}' from custom_id: {}",
                poll_id.as_ref().unwrap(),
                custom_id
            );
        }
        poll_id
    };

    // Fetch the poll from database
    let poll: Option<Poll> = if let Some(ref poll_id) = poll_id_opt {
        match database.get_poll(poll_id).await {
            Ok(p) => Some(p),
            Err(e) => {
                error!(
                    "Failed to fetch poll ID '{}' for component interaction '{}': {}",
                    poll_id, custom_id, e
                );
                let error_message = if custom_id.starts_with("voteChange_") {
                    "Cannot change vote: the poll no longer exists or has been deleted."
                } else if custom_id.starts_with("doneVoting_") {
                    "Cannot complete voting: the poll no longer exists or has been deleted."
                } else {
                    "Error: The poll no longer exists or has been deleted."
                };
                component
                    .create_interaction_response(&ctx.http, |response| {
                        response
                            .kind(InteractionResponseType::ChannelMessageWithSource)
                            .interaction_response_data(|message| {
                                message.content(error_message).ephemeral(true)
                            })
                    })
                    .await?;
                return Ok(());
            }
        }
    } else {
        if !custom_id.starts_with("rankLabel_") && !custom_id.starts_with("label_") {
            component
                .create_interaction_response(&ctx.http, |response| {
                    response
                        .kind(InteractionResponseType::ChannelMessageWithSource)
                        .interaction_response_data(|message| {
                            message
                                .content("Error identifying the poll for this action.")
                                .ephemeral(true)
                        })
                })
                .await?;
        }
        return Ok(());
    };

    // If poll is found but inactive, disallow all interactions except sharing results/votes
    if let Some(ref p) = poll {
        if !p.is_active
            && !custom_id.starts_with("shareResults_")
            && !custom_id.starts_with("shareVote_")
        {
            component
                .create_interaction_response(&ctx.http, |response| {
                    response
                        .kind(InteractionResponseType::ChannelMessageWithSource)
                        .interaction_response_data(|message| {
                            message.content("This poll has ended.").ephemeral(true)
                        })
                })
                .await?;
            return Ok(());
        }

        // Enforce role restrictions
        if let Some(allowed_roles) = &p.allowed_roles {
            let has_permission = if let Some(member) = &component.member {
                member
                    .roles
                    .iter()
                    .any(|role_id| allowed_roles.contains(&role_id.to_string()))
            } else {
                false // If we can't verify roles (e.g. not in guild), deny access
            };

            if !has_permission {
                component
                    .create_interaction_response(&ctx.http, |response| {
                        response
                            .kind(InteractionResponseType::ChannelMessageWithSource)
                            .interaction_response_data(|message| {
                                message
                                    .content("You do not have permission to vote in this poll.")
                                    .ephemeral(true)
                            })
                    })
                    .await?;
                return Ok(());
            }
        }
    }

    // Route to the appropriate handler based on the custom_id
    if custom_id == "vote_button" || custom_id == "voteButton" {
        if let Some(p) = poll {
            vote::handle_vote_button(database, ctx, component, &p).await?;
        }
    } else if custom_id.starts_with("starSelect_") {
        let parts: Vec<&str> = custom_id.split('_').collect();
        if parts.len() >= 3 {
            let poll_id = parts[1];
            let option_id = parts[2];
            vote::handle_star_select(database, ctx, component, poll_id, option_id).await?;
        } else {
            warn!("Invalid starSelect format: {}", custom_id);
        }
    } else if custom_id.starts_with("starPage_") {
        let parts: Vec<&str> = custom_id.split('_').collect();
        if parts.len() >= 3 {
            info!("Navigating to star voting page {}", parts[2]);
        }
        if let Some(p) = poll {
            vote::handle_vote_button(database, ctx, component, &p).await?;
        }
    } else if custom_id.starts_with("rankPage_") {
        let parts: Vec<&str> = custom_id.split('_').collect();
        if parts.len() >= 3 {
            info!("Navigating to rank voting page {}", parts[2]);
        }
        if let Some(p) = poll {
            vote::handle_vote_button(database, ctx, component, &p).await?;
        }
    } else if custom_id.starts_with("doneVoting_") {
        if let Some(p) = poll {
            vote::handle_done_voting(database, ctx, component, &p.id, &p).await?;
        } else {
            error!("Poll object unavailable for done voting action.");
            component
                .create_interaction_response(&ctx.http, |response| {
                    response
                        .kind(InteractionResponseType::ChannelMessageWithSource)
                        .interaction_response_data(|message| {
                            message
                                .content("Error processing your vote completion.")
                                .ephemeral(true)
                        })
                })
                .await?;
        }
    } else if custom_id.starts_with("voteChange_") {
        if let Some(p) = poll {
            vote::handle_change_vote(database, ctx, component, &p.id, &p).await?;
        } else {
            error!("Poll object unavailable for change vote action.");
            component
                .create_interaction_response(&ctx.http, |response| {
                    response
                        .kind(InteractionResponseType::ChannelMessageWithSource)
                        .interaction_response_data(|message| {
                            message
                                .content("Error returning to voting interface.")
                                .ephemeral(true)
                        })
                })
                .await?;
        }
    } else if custom_id.starts_with("pluralityVote_") {
        let parts: Vec<&str> = custom_id.split('_').collect();
        if parts.len() >= 3 {
            let poll_id = parts[1];
            let option_id = parts[2];
            if let Some(p) = poll {
                vote::handle_plurality_vote(database, ctx, component, poll_id, option_id, &p)
                    .await?;
            }
        } else {
            warn!("Invalid pluralityVote format: {}", custom_id);
        }
    } else if custom_id.starts_with("approvalVote_") {
        let parts: Vec<&str> = custom_id.split('_').collect();
        if parts.len() >= 4 {
            let poll_id = parts[1];
            let option_id = parts[2];
            let current_value = parts[3].parse().unwrap_or(0);
            if let Some(p) = poll {
                vote::handle_approval_vote_toggle(
                    database,
                    ctx,
                    component,
                    poll_id,
                    option_id,
                    current_value,
                    &p,
                )
                .await?;
            }
        } else {
            warn!("Invalid approvalVote format: {}", custom_id);
        }
    } else if custom_id.starts_with("rankUp_")
        || custom_id.starts_with("rankDown_")
        || custom_id.starts_with("rankRemove_")
    {
        let action = if custom_id.starts_with("rankUp_") {
            "up"
        } else if custom_id.starts_with("rankDown_") {
            "down"
        } else {
            "remove"
        };
        let parts: Vec<&str> = custom_id.split('_').collect();
        if parts.len() >= 3 {
            let option_id = parts[2];
            if let Some(p) = poll {
                vote::handle_rank_action(database, ctx, component, option_id, action, &p).await?;
            }
        } else {
            warn!("Invalid rank action format: {}", custom_id);
        }
    } else if custom_id.starts_with("rankLabel_") {
        component
            .create_interaction_response(&ctx.http, |response| {
                response.kind(InteractionResponseType::DeferredUpdateMessage)
            })
            .await?;
    } else if custom_id.starts_with("shareResults_") {
        if let Some(p) = poll {
            let votes = database.get_poll_votes(&p.id).await?;
            let results = crate::commands::poll::calculate_poll_results(&p, &votes);

            // Send public message
            component
                .channel_id
                .send_message(&ctx.http, |m| {
                    m.content(format!(
                        "Results for poll '{}' shared by <@{}>:",
                        p.question, component.user.id
                    ))
                    .embed(|e| crate::commands::poll::create_results_embed(e, &p, &results))
                })
                .await?;

            // Update ephemeral message to disable button
            component
                .create_interaction_response(&ctx.http, |response| {
                    response
                        .kind(InteractionResponseType::UpdateMessage)
                        .interaction_response_data(|message| {
                            message.components(|c| {
                                c.create_action_row(|row| {
                                    row.create_button(|btn| {
                                        btn.custom_id("shared")
                                            .label("Results Shared!")
                                            .style(ButtonStyle::Success)
                                            .disabled(true)
                                    })
                                })
                            })
                        })
                })
                .await?;
        }
    } else if custom_id.starts_with("shareVote_") {
        if let Some(p) = poll {
            let user_votes = database
                .get_user_poll_votes(&p.id, &component.user.id.to_string())
                .await?;
            let vote_details = vote::format_user_vote(&p, &user_votes);

            // Send public message
            component
                .channel_id
                .send_message(&ctx.http, |m| {
                    m.embed(|e| {
                        e.title(format!("🗳️ Vote Shared: {}", p.question))
                            .description(format!(
                                "**User**: <@{}>\n**Method**: {}\n\n{}",
                                component.user.id, p.voting_method, vote_details
                            ))
                            .color((0, 255, 0)) // Green
                    })
                })
                .await?;

            // Update ephemeral message to disable button
            component
                .create_interaction_response(&ctx.http, |response| {
                    response
                        .kind(InteractionResponseType::UpdateMessage)
                        .interaction_response_data(|message| {
                            message.components(|c| {
                                c.create_action_row(|row| {
                                    row.create_button(|btn| {
                                        btn.custom_id(format!("voteChange_{}", p.id))
                                            .label("Change My Vote")
                                            .style(ButtonStyle::Secondary)
                                    })
                                    .create_button(|btn| {
                                        btn.custom_id("shared")
                                            .label("Vote Shared!")
                                            .style(ButtonStyle::Success)
                                            .disabled(true)
                                    })
                                })
                            })
                        })
                })
                .await?;
        }
    } else {
        warn!("Unhandled component custom_id: {}", custom_id);
        component
            .create_interaction_response(&ctx.http, |response| {
                response
                    .kind(InteractionResponseType::ChannelMessageWithSource)
                    .interaction_response_data(|message| {
                        message.content("Unknown button action.").ephemeral(true)
                    })
            })
            .await?;
    }

    Ok(())
}

pub async fn handle_interaction(database: &Database, ctx: &Context, interaction: Interaction) {
    let result = match interaction {
        Interaction::ApplicationCommand(command) => handle_command(database, ctx, &command).await,
        Interaction::MessageComponent(component) => {
            handle_component(database, ctx, &component).await
        }
        _ => {
            warn!("Unhandled interaction type: {:?}", interaction.kind());
            Ok(())
        }
    };

    if let Err(why) = result {
        error!("Interaction handler error: {:?}", why);
    }
}
