use crate::db::Database;
use crate::models::{Poll, VotingMethod};
use chrono::Utc;
use log::{error, info, warn};
use serenity::builder::CreateApplicationCommand;
use serenity::builder::{CreateActionRow, CreateEmbed};
use serenity::model::application::component::ButtonStyle;
use serenity::model::application::interaction::InteractionResponseType;
use serenity::model::application::interaction::application_command::ApplicationCommandInteraction;
use serenity::model::id::{ChannelId, MessageId};
use serenity::prelude::*;

pub fn create_poll_command(
    command: &mut CreateApplicationCommand,
) -> &mut CreateApplicationCommand {
    command
        .name("poll")
        .description("Create and manage polls")
        .create_option(|option| {
            option
                .name("create")
                .description("Create a new poll")
                .kind(serenity::model::application::command::CommandOptionType::SubCommand)
                .create_sub_option(|sub_option| {
                    sub_option
                        .name("question")
                        .description("The poll question")
                        .kind(serenity::model::application::command::CommandOptionType::String)
                        .required(true)
                })
                .create_sub_option(|sub_option| {
                    sub_option
                        .name("options")
                        .description("Comma-separated list of options")
                        .kind(serenity::model::application::command::CommandOptionType::String)
                        .required(true)
                })
                .create_sub_option(|sub_option| {
                    sub_option
                        .name("method")
                        .description("Voting method to use")
                        .kind(serenity::model::application::command::CommandOptionType::String)
                        .add_string_choice("STAR", "star")
                        .add_string_choice("Plurality", "plurality")
                        .add_string_choice("Ranked Choice", "ranked")
                        .add_string_choice("Approval", "approval")
                        .required(true)
                })
                .create_sub_option(|sub_option| {
                    sub_option
                        .name("duration")
                        .description(
                            "Duration in minutes (default: 1440 = 24 hours, 0 for manual close)",
                        )
                        .kind(serenity::model::application::command::CommandOptionType::Integer)
                        .required(false)
                })
                .create_sub_option(|sub_option| {
                    sub_option
                        .name("allowed_role")
                        .description("Restrict voting to members with this role (optional)")
                        .kind(serenity::model::application::command::CommandOptionType::Role)
                        .required(false)
                })
            // .create_sub_option(|sub_option| {
            //     sub_option
            //         .name("anonymous")
            //         .description("Whether votes should be anonymous (default: true)")
            //         .kind(serenity::model::application::command::CommandOptionType::Boolean)
            //         .required(false)
            // })
        })
        .create_option(|option| {
            option
                .name("end")
                .description("End an active poll")
                .kind(serenity::model::application::command::CommandOptionType::SubCommand)
                .create_sub_option(|sub_option| {
                    sub_option
                        .name("poll_id")
                        .description("ID of the poll to end")
                        .kind(serenity::model::application::command::CommandOptionType::String)
                        .required(false)
                })
        })
        .create_option(|option| {
            option
                .name("results")
                .description("Show results for an ended poll")
                .kind(serenity::model::application::command::CommandOptionType::SubCommand)
                .create_sub_option(|sub_option| {
                    sub_option
                        .name("poll_id")
                        .description("ID of the poll to show results for")
                        .kind(serenity::model::application::command::CommandOptionType::String)
                        .required(false)
                })
        })
        .create_option(|option| {
            option
                .name("list")
                .description("List active and recently ended polls in this server")
                .kind(serenity::model::application::command::CommandOptionType::SubCommand)
        })
        .create_option(|option| {
            option
                .name("help")
                .description("Show help information")
                .kind(serenity::model::application::command::CommandOptionType::SubCommand)
        })
        .create_option(|option| {
            option
                .name("export")
                .description("Export vote data for a completed poll as CSV")
                .kind(serenity::model::application::command::CommandOptionType::SubCommand)
                .create_sub_option(|sub_option| {
                    sub_option
                        .name("poll_id")
                        .description("ID of the poll to export")
                        .kind(serenity::model::application::command::CommandOptionType::String)
                        .required(true)
                })
        })
}

pub async fn handle_poll_command(
    database: &Database,
    ctx: &Context,
    command: &ApplicationCommandInteraction,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let subcommand_name = match command.data.options.first() {
        Some(option) => option.name.as_str(),
        None => {
            send_error_response(ctx, command, "No subcommand provided").await?;
            return Ok(());
        }
    };

    match subcommand_name {
        "create" => handle_create_poll(database, ctx, command).await?,
        "end" => handle_end_poll(database, ctx, command).await?,
        "results" => handle_poll_results(database, ctx, command).await?,
        "list" => handle_list_polls(database, ctx, command).await?,
        "export" => handle_export_poll(database, ctx, command).await?,
        "help" => {
            command.create_interaction_response(&ctx.http, |resp| {
                resp.kind(serenity::model::application::interaction::InteractionResponseType::ChannelMessageWithSource)
                    .interaction_response_data(|msg| {
                        msg.ephemeral(true).embed(|e| {
                            e.title("📊 Trusty Vote Poll System Guide")
                                .description("Welcome to Trusty Vote! I'm here to help your server make better decisions with better voting.")
                                .field("📝 Creating Polls", 
                                    "Use `/poll create` with a question, comma-separated options, and your preferred voting method.\n\
                                    For longer polls, set the duration in minutes (use 0 for manual closing).\n\
                                    Optionally restrict voting to a specific role.", 
                                    false)
                                .field("🗳️ Voting Methods", 
                                    "**STAR Voting**: Rate each option 0-5 stars. Combines scoring and an automatic runoff between top choices.\n\
                                    **Plurality**: Classic 'most votes wins' system. Each person picks one option.\n\
                                    **Ranked Choice**: Rank options in order of preference. Eliminates lowest choices until majority reached.\n\
                                    **Approval**: Approve all of the options you like. Most approvals wins.", 
                                    false)
                                .field("⚙️ Managing Polls", 
                                    "• End active polls with `/poll end` (interactive) or `/poll end [poll-id]`\n\
                                    • See results with `/poll results` (interactive) or `/poll results [poll-id]`\n\
                                    • See all server polls with `/poll list`\n\
                                    • Export vote data with `/poll export [poll-id]` (for completed polls)\n\
                                    • Poll IDs are shown in poll embeds for reference", 
                                    false)
                                .field("💡 Tips", 
                                    "> Keep option lists concise for better mobile experience\n\
                                    > For complex decisions, STAR or Ranked Choice voting reduces tactical voting\n\
                                    > Plurality works best for simple A/B decisions", 
                                    false)
                                .footer(|f| f.text("Trusty Vote • Helping your server make better decisions"))
                                .color((255, 165, 0))
                        })
                    })
            }).await?;
        }
        _ => {
            send_error_response(ctx, command, "Unknown subcommand").await?;
        }
    }

    Ok(())
}

async fn handle_create_poll(
    database: &Database,
    ctx: &Context,
    command: &ApplicationCommandInteraction,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let options = match command.data.options.first() {
        Some(subcommand) => &subcommand.options,
        None => {
            send_error_response(ctx, command, "Missing options").await?;
            return Ok(());
        }
    };

    let mut question = String::new();
    let mut options_str = String::new();
    let mut method_str = String::new();
    let mut duration: Option<i64> = None;
    let mut allowed_roles: Option<Vec<String>> = None;
    // let mut anonymous = true;

    for option in options {
        match option.name.as_str() {
            "question" => {
                question = option.value.as_ref().unwrap().as_str().unwrap().to_string();
            }
            "options" => {
                options_str = option.value.as_ref().unwrap().as_str().unwrap().to_string();
            }
            "method" => {
                method_str = option.value.as_ref().unwrap().as_str().unwrap().to_string();
            }
            "duration" => {
                if let Some(value) = option.value.as_ref() {
                    duration = Some(value.as_i64().unwrap_or(1440));
                }
            }
            "allowed_role" => {
                if let Some(value) = option.value.as_ref() {
                    let role_id = value.as_str().unwrap_or("").to_string();
                    if !role_id.is_empty() {
                        allowed_roles = Some(vec![role_id]);
                    }
                }
            }
            _ => {}
        }
    }

    let options_vec: Vec<String> = options_str
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    if options_vec.len() < 2 {
        send_error_response(ctx, command, "You need at least 2 options for a poll").await?;
        return Ok(());
    }

    if options_vec.len() > 10 {
        send_error_response(ctx, command, "Maximum 10 options allowed").await?;
        return Ok(());
    }

    let voting_method = match method_str.as_str() {
        "star" => VotingMethod::Star,
        "plurality" => VotingMethod::Plurality,
        "ranked" => VotingMethod::Ranked,
        "approval" => VotingMethod::Approval,
        _ => {
            send_error_response(ctx, command, "Invalid voting method").await?;
            return Ok(());
        }
    };

    let guild_id = command.guild_id.ok_or("Missing guild ID")?.to_string();
    let channel_id = command.channel_id.to_string();
    let creator_id = command.user.id.to_string();

    let mut poll = Poll::new(
        guild_id,
        channel_id.clone(),
        creator_id,
        question.clone(),
        options_vec,
        voting_method.clone(),
        duration,
        allowed_roles,
    );

    database.create_poll(&poll).await?;

    let interaction_response = command
        .create_interaction_response(&ctx.http, |response| {
            response
                .kind(InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|message| {
                    message
                        .embed(|e| create_poll_embed(e, &poll))
                        .components(|c| {
                            c.create_action_row(|row| create_poll_components(row, &poll))
                        })
                })
        })
        .await;

    if let Ok(_) = interaction_response {
        match command.get_interaction_response(&ctx.http).await {
            Ok(message) => {
                let message_id_str = message.id.to_string();
                poll.message_id = Some(message_id_str.clone());
                if let Err(e) = database
                    .update_poll_message_id(&poll.id, &message_id_str)
                    .await
                {
                    error!("Failed to update message ID for poll {}: {}", poll.id, e);
                } else {
                    info!("Stored message ID {} for poll {}", message_id_str, poll.id);
                }
            }
            Err(e) => {
                error!(
                    "Failed to get interaction response message for poll {}: {}",
                    poll.id, e
                );
            }
        }
    } else if let Err(e) = interaction_response {
        error!(
            "Failed to create interaction response for poll {}: {}",
            poll.id, e
        );
    }

    Ok(())
}

async fn handle_poll_results(
    database: &Database,
    ctx: &Context,
    command: &ApplicationCommandInteraction,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let poll_id_opt = command
        .data
        .options
        .first()
        .and_then(|option| option.options.first())
        .and_then(|option| option.value.as_ref())
        .and_then(|value| value.as_str())
        .map(|s| s.to_string());

    let poll_id = match poll_id_opt {
        Some(id) => id,
        None => {
            // No ID provided, show selection menu
            let guild_id = command.guild_id.ok_or("Missing guild ID")?.to_string();
            let ended_polls = database
                .get_recently_ended_polls_by_guild(&guild_id, 25)
                .await?;

            if ended_polls.is_empty() {
                send_error_response(ctx, command, "No recently ended polls found.").await?;
                return Ok(());
            }

            command
                .create_interaction_response(&ctx.http, |response| {
                    response
                        .kind(InteractionResponseType::ChannelMessageWithSource)
                        .interaction_response_data(|message| {
                            message
                                .ephemeral(true)
                                .content("Select a poll to view results:")
                                .components(|c| {
                                    c.create_action_row(|row| {
                                        row.create_select_menu(|menu| {
                                            menu.custom_id("selectResultsPoll")
                                                .placeholder("Choose a poll...")
                                                .options(|opts| {
                                                    for poll in ended_polls.iter() {
                                                        let label = if poll.question.len() > 90 {
                                                            format!("{}...", &poll.question[..87])
                                                        } else {
                                                            poll.question.clone()
                                                        };
                                                        opts.create_option(|o| {
                                                            o.label(label).value(&poll.id)
                                                        });
                                                    }
                                                    opts
                                                })
                                        })
                                    })
                                })
                        })
                })
                .await?;
            return Ok(());
        }
    };

    let poll = match database.get_poll(&poll_id).await {
        Ok(p) => p,
        Err(_) => {
            send_error_response(ctx, command, "Poll not found").await?;
            return Ok(());
        }
    };

    if poll.is_active {
        let ends_at_msg = match poll.ends_at {
            Some(time) => format!("<t:{}:R>", time.timestamp()),
            None => "manually ended".to_string(),
        };

        let msg = format!(
            "Poll is still active. Wait until {} or use `/poll end {}` to end the polling early.",
            ends_at_msg, poll_id
        );
        send_error_response(ctx, command, &msg).await?;
        return Ok(());
    }

    let votes = database.get_poll_votes(&poll_id).await?;
    let results = calculate_poll_results(&poll, &votes);

    command
        .create_interaction_response(&ctx.http, |response| {
            response
                .kind(InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|message| {
                    message
                        .ephemeral(true)
                        .embed(|e| create_results_embed(e, &poll, &results))
                        .components(|c| {
                            c.create_action_row(|row| {
                                row.create_button(|btn| {
                                    btn.custom_id(format!("shareResults_{}", poll.id))
                                        .label("Share Results")
                                        .style(ButtonStyle::Primary)
                                })
                            })
                        })
                })
        })
        .await?;

    Ok(())
}

fn create_poll_embed<'a>(embed: &'a mut CreateEmbed, poll: &Poll) -> &'a mut CreateEmbed {
    let method_name = match poll.voting_method {
        VotingMethod::Star => "STAR Voting",
        VotingMethod::Plurality => "Plurality Voting",
        VotingMethod::Ranked => "Ranked Choice Voting",
        VotingMethod::Approval => "Approval Voting",
    };

    let ends_at_str = match poll.ends_at {
        Some(time) => format!("<t:{}:R>", time.timestamp()),
        None => "When manually ended".to_string(),
    };

    let options_list = poll
        .options
        .iter()
        .map(|option| format!("• {}", option.text))
        .collect::<Vec<String>>()
        .join("\n");

    let mut embed = embed
        .title(&poll.question)
        .description(format!("**Options:**\n{}", options_list))
        .field("Voting Method", method_name, true)
        .field("Poll ID", &poll.id, true)
        .field("Ends", ends_at_str, true);

    if let Some(roles) = &poll.allowed_roles {
        if let Some(role_id) = roles.get(0) {
            embed = embed.field("Who Can Vote", format!("<@&{}> only", role_id), false);
        }
    }

    embed
        .footer(|f| f.text("Click the buttons below to vote!"))
        .timestamp(poll.created_at.to_rfc3339())
}

// Using camelCase format for consistency
fn create_poll_components<'a>(
    row: &'a mut CreateActionRow,
    poll: &Poll,
) -> &'a mut CreateActionRow {
    if !poll.is_active {
        return row;
    }

    row.create_button(|button| {
        button
            .custom_id("voteButton") // Using camelCase format for consistency
            .style(ButtonStyle::Primary)
            .label("Cast Your Vote")
    })
}

pub async fn end_poll_logic(
    database: &Database,
    ctx: &Context,
    poll_id: &str,
    channel_id_str: &str,
    message_id_opt: Option<String>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!("Attempting to end poll: {}", poll_id);

    database.end_poll(poll_id).await?;
    info!("Marked poll {} as inactive in DB", poll_id);

    let poll = database.get_poll(poll_id).await?;
    info!("Fetched poll data for {}", poll_id);

    let votes = database.get_poll_votes(poll_id).await?;
    info!("Fetched {} votes for poll {}", votes.len(), poll_id);

    let results = calculate_poll_results(&poll, &votes);
    info!("Calculated results for poll {}", poll_id);

    if let (Some(message_id_str), Ok(channel_id)) =
        (message_id_opt, channel_id_str.parse::<ChannelId>())
    {
        if let Ok(message_id) = message_id_str.parse::<MessageId>() {
            match ctx.http.get_message(channel_id.0, message_id.0).await {
                Ok(mut message) => {
                    if let Err(e) = message
                        .edit(&ctx.http, |m| {
                            m.components(|c| c).embed(|e| create_poll_embed(e, &poll))
                        })
                        .await
                    {
                        error!(
                            "Failed to edit original poll message {} in channel {}: {}. Check bot permissions (View Channel, Manage Messages).",
                            message_id_str, channel_id_str, e
                        );
                    } else {
                        info!(
                            "Successfully removed buttons from poll message {}",
                            message_id_str
                        );
                    }
                }
                Err(e) => {
                    error!(
                        "Failed to fetch original poll message {} in channel {}: {}. Check bot permissions (View Channel, Read Message History).",
                        message_id_str, channel_id_str, e
                    );
                }
            }
        } else {
            error!(
                "Failed to parse message ID {} for poll {}",
                message_id_str, poll_id
            );
        }
    } else {
        warn!(
            "Could not edit original message for poll {}: Missing message_id or invalid channel_id",
            poll_id
        );
    }

    let channel_id = channel_id_str.parse::<ChannelId>()?;
    if let Err(e) = channel_id
        .send_message(&ctx.http, |m| {
            m.content(format!("Poll '{}' has ended!", poll.question))
                .embed(|e| create_results_embed(e, &poll, &results))
        })
        .await
    {
        error!(
            "Failed to send results for poll {} to channel {}: {}. Check bot permissions (View Channel, Send Messages, Embed Links).",
            poll_id, channel_id_str, e
        );
    } else {
        info!("Successfully sent results for poll {}", poll_id);
    }

    Ok(())
}

async fn handle_end_poll(
    database: &Database,
    ctx: &Context,
    command: &ApplicationCommandInteraction,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let poll_id_opt = command
        .data
        .options
        .first()
        .and_then(|option| option.options.first())
        .and_then(|option| option.value.as_ref())
        .and_then(|value| value.as_str())
        .map(|s| s.to_string());

    let poll_id = match poll_id_opt {
        Some(id) => id,
        None => {
            // No ID provided, show selection menu
            let guild_id = command.guild_id.ok_or("Missing guild ID")?.to_string();
            let active_polls = database.get_active_polls_by_guild(&guild_id).await?;

            if active_polls.is_empty() {
                send_error_response(ctx, command, "No active polls found to end.").await?;
                return Ok(());
            }

            command
                .create_interaction_response(&ctx.http, |response| {
                    response
                        .kind(InteractionResponseType::ChannelMessageWithSource)
                        .interaction_response_data(|message| {
                            message
                                .ephemeral(true)
                                .content("Select a poll to end:")
                                .components(|c| {
                                    c.create_action_row(|row| {
                                        row.create_select_menu(|menu| {
                                            menu.custom_id("selectEndPoll")
                                                .placeholder("Choose a poll...")
                                                .options(|opts| {
                                                    for poll in active_polls.iter().take(25) {
                                                        let label = if poll.question.len() > 90 {
                                                            format!("{}...", &poll.question[..87])
                                                        } else {
                                                            poll.question.clone()
                                                        };
                                                        opts.create_option(|o| {
                                                            o.label(label).value(&poll.id)
                                                        });
                                                    }
                                                    opts
                                                })
                                        })
                                    })
                                })
                        })
                })
                .await?;
            return Ok(());
        }
    };

    let poll = match database.get_poll(&poll_id).await {
        Ok(p) => p,
        Err(_) => {
            send_error_response(ctx, command, "Poll not found").await?;
            return Ok(());
        }
    };

    if !poll.is_active {
        let votes = database.get_poll_votes(&poll_id).await?;
        let results = calculate_poll_results(&poll, &votes);

        let msg = if let Some(ends_at) = poll.ends_at {
            if ends_at < Utc::now() {
                format!(
                    "This poll has already ended at <t:{}:f>, here are the results:",
                    ends_at.timestamp()
                )
            } else {
                "This poll has already ended, here are the results:".to_string()
            }
        } else {
            "This poll has already ended, here are the results:".to_string()
        };

        command
            .create_interaction_response(&ctx.http, |response| {
                response
                    .kind(InteractionResponseType::ChannelMessageWithSource)
                    .interaction_response_data(|message| {
                        message
                            .ephemeral(true)
                            .content(msg)
                            .embed(|e| create_results_embed(e, &poll, &results))
                    })
            })
            .await?;
        return Ok(());
    }

    command
        .create_interaction_response(&ctx.http, |response| {
            response
                .kind(InteractionResponseType::DeferredChannelMessageWithSource)
                .interaction_response_data(|message| message.ephemeral(true))
        })
        .await?;

    match end_poll_logic(database, ctx, &poll_id, &poll.channel_id, poll.message_id).await {
        Ok(_) => {
            command
                .edit_original_interaction_response(&ctx.http, |response| {
                    response.content("Poll ended successfully.")
                })
                .await?;
        }
        Err(e) => {
            error!("Error ending poll {} manually: {}", poll_id, e);
            command
                .edit_original_interaction_response(&ctx.http, |response| {
                    response.content(format!("Failed to end poll: {}", e))
                })
                .await?;
        }
    }

    Ok(())
}

async fn handle_list_polls(
    database: &Database,
    ctx: &Context,
    command: &ApplicationCommandInteraction,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let guild_id = command.guild_id.ok_or("Missing guild ID")?.to_string();

    let active_polls = database.get_active_polls_by_guild(&guild_id).await?;
    let recent_polls = database
        .get_recently_ended_polls_by_guild(&guild_id, 5)
        .await?;

    command
        .create_interaction_response(&ctx.http, |response| {
            response
                .kind(InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|message| {
                    message.ephemeral(true).embed(|e| {
                        e.title("Polls in this Server")
                            .description("Here are the active and recently ended polls.");

                        if !active_polls.is_empty() {
                            let active_list = active_polls
                                .iter()
                                .map(|p| {
                                    let ends = p.ends_at.map_or("Manual".to_string(), |t| {
                                        format!("<t:{}:R>", t.timestamp())
                                    });
                                    format!("`{}`: {} (Ends: {})", p.id, p.question, ends)
                                })
                                .collect::<Vec<_>>()
                                .join("\n");
                            e.field("Active Polls", active_list, false);
                        } else {
                            e.field("Active Polls", "No active polls.", false);
                        }

                        if !recent_polls.is_empty() {
                            let recent_list = recent_polls
                                .iter()
                                .map(|p| {
                                    let ended = p.ends_at.map_or("N/A".to_string(), |t| {
                                        format!("<t:{}:R>", t.timestamp())
                                    });
                                    format!("`{}`: {} (Ended: {})", p.id, p.question, ended)
                                })
                                .collect::<Vec<_>>()
                                .join("\n");
                            e.field("Recently Ended Polls (Max 5)", recent_list, false);
                        } else {
                            e.field(
                                "Recently Ended Polls",
                                "No recently ended polls found.",
                                false,
                            );
                        }

                        e
                    })
                })
        })
        .await?;

    Ok(())
}

pub fn calculate_poll_results(
    poll: &crate::models::Poll,
    votes: &[crate::models::Vote],
) -> crate::voting::PollResults {
    match poll.voting_method {
        crate::models::VotingMethod::Star => crate::voting::star::calculate_results(poll, votes),
        crate::models::VotingMethod::Plurality => {
            crate::voting::plurality::calculate_results(poll, votes)
        }
        crate::models::VotingMethod::Ranked => {
            crate::voting::ranked::calculate_results(poll, votes)
        }
        crate::models::VotingMethod::Approval => {
            crate::voting::approval::calculate_results(poll, votes)
        }
    }
}

pub fn create_results_embed<'a>(
    embed: &'a mut CreateEmbed,
    poll: &crate::models::Poll,
    results: &crate::voting::PollResults,
) -> &'a mut CreateEmbed {
    // Truncate summary if it's too long for an embed field
    let summary_display = if results.summary.len() > 1024 {
        format!("{}...", &results.summary[..1020]) // Leave space for "..."
    } else {
        results.summary.clone()
    };

    embed
        .title(format!("Results: {}", poll.question))
        .description("The poll has ended. Here are the results:")
        .field("Winner", &results.winner, false)
        .field("Details", &summary_display, false) // Use the potentially truncated summary
        .footer(|f| f.text(format!("Poll ID: {}", poll.id)))
        .timestamp(Utc::now().to_rfc3339())
}

// Export poll votes as CSV
async fn handle_export_poll(
    database: &Database,
    ctx: &Context,
    command: &ApplicationCommandInteraction,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let poll_id = match command
        .data
        .options
        .first()
        .and_then(|option| option.options.first())
        .and_then(|option| option.value.as_ref())
        .and_then(|value| value.as_str())
    {
        Some(id) => id.to_string(),
        None => {
            send_error_response(ctx, command, "No poll ID provided").await?;
            return Ok(());
        }
    };

    let poll = match database.get_poll(&poll_id).await {
        Ok(p) => p,
        Err(_) => {
            send_error_response(ctx, command, "Poll not found").await?;
            return Ok(());
        }
    };

    if poll.is_active {
        send_error_response(
            ctx,
            command,
            "Cannot export data for an active poll. End the poll first with `/poll end`.",
        )
        .await?;
        return Ok(());
    }

    let votes = database.get_poll_votes(&poll_id).await?;

    // Generate CSV content
    let mut csv_content = String::new();
    csv_content.push_str("User ID,Option ID,Option Text,Rating,Timestamp\n");

    for vote in &votes {
        let option_text = poll
            .options
            .iter()
            .find(|opt| opt.id == vote.option_id)
            .map(|opt| opt.text.clone())
            .unwrap_or_else(|| "Unknown Option".to_string());

        // Escape CSV values that contain commas or quotes
        let escaped_text = if option_text.contains(',') || option_text.contains('"') {
            format!("\"{}\"", option_text.replace('"', "\"\""))
        } else {
            option_text
        };

        csv_content.push_str(&format!(
            "{},{},{},{},{}\n",
            vote.user_id,
            vote.option_id,
            escaped_text,
            vote.rating,
            vote.timestamp.format("%Y-%m-%d %H:%M:%S UTC")
        ));
    }

    // Send the CSV content as text since file attachments aren't supported in responses
    command
        .create_interaction_response(&ctx.http, |response| {
            response
                .kind(InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|message| {
                    message.ephemeral(true).content(format!(
                        "**Exported {} votes from poll: \"{}\"**\n\n```csv\n{}\n```",
                        votes.len(),
                        poll.question,
                        if csv_content.len() > 1800 {
                            format!(
                                "{}...\n(CSV truncated - too many votes to display)",
                                &csv_content[..1800]
                            )
                        } else {
                            csv_content
                        }
                    ))
                })
        })
        .await?;

    Ok(())
}

async fn send_error_response(
    ctx: &Context,
    command: &ApplicationCommandInteraction,
    error_message: &str,
) -> Result<(), serenity::Error> {
    command
        .create_interaction_response(&ctx.http, |response| {
            response
                .kind(InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|message| {
                    message
                        .content(format!("Error: {}", error_message))
                        .ephemeral(true)
                })
        })
        .await
}
