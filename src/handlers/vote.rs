use crate::db::Database;
use crate::models::Poll;
use chrono::Utc;
use log::{info, warn};
use serenity::model::application::component::{ActionRowComponent, ButtonStyle};
use serenity::model::application::interaction::{
    InteractionResponseType, message_component::MessageComponentInteraction,
};
use serenity::prelude::*;

pub async fn handle_vote_button(
    database: &Database,
    ctx: &Context,
    component: &MessageComponentInteraction,
    poll: &Poll,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Add log message to show which custom_id was used for debugging
    info!(
        "Processing vote button interaction with custom_id: {}",
        component.data.custom_id
    );

    let user_id = component.user.id.to_string();

    let existing_votes = database.get_user_poll_votes(&poll.id, &user_id).await?;
    let mut option_ratings = std::collections::HashMap::<String, i32>::new();
    for vote in &existing_votes {
        option_ratings.insert(vote.option_id.clone(), vote.rating);
    }

    match poll.voting_method {
        crate::models::VotingMethod::Star => {
            let page = if component.data.custom_id.starts_with("starPage_") {
                component
                    .data
                    .custom_id
                    .split('_')
                    .next_back()
                    .and_then(|s| s.parse::<usize>().ok())
                    .unwrap_or(0)
            } else if component.data.custom_id.starts_with("star_page_") {
                component
                    .data
                    .custom_id
                    .split('_')
                    .next_back()
                    .and_then(|s| s.parse::<usize>().ok())
                    .unwrap_or(0)
            } else {
                0
            };

            let options_per_page = 4;
            let total_pages = poll.options.len().div_ceil(options_per_page);
            let start_idx = page * options_per_page;
            let end_idx = std::cmp::min(start_idx + options_per_page, poll.options.len());

            let options_to_show = &poll.options[start_idx..end_idx];
            let pagination_info = if total_pages > 1 {
                format!(
                    "\nPage {} of {} - Rate each option from 0-5 stars",
                    page + 1,
                    total_pages
                )
            } else {
                String::from("\nRate each option from 0-5 stars")
            };

            component
                .create_interaction_response(&ctx.http, |response| {
                    response
                        .kind(InteractionResponseType::ChannelMessageWithSource)
                        .interaction_response_data(|message| {
                            message
                                .ephemeral(true)
                                .content(format!("**{}**{}", poll.question, pagination_info))
                                .components(|c| {
                                    for option in options_to_show {
                                        let rating =
                                            option_ratings.get(&option.id).copied().unwrap_or(0);
                                        let truncated_name = if option.text.len() > 80 {
                                            format!("{}...", &option.text[..77])
                                        } else {
                                            option.text.clone()
                                        };
                                        let short_name = if option.text.len() > 20 {
                                            format!("{}...", &option.text[..17])
                                        } else {
                                            option.text.clone()
                                        };

                                        c.create_action_row(|row| {
                                            row.create_select_menu(|menu| {
                                                menu.custom_id(format!(
                                                    "starSelect_{}_{}",
                                                    poll.id, option.id
                                                ))
                                                .placeholder(if rating > 0 {
                                                    format!(
                                                        "{} - {}",
                                                        truncated_name,
                                                        "★".repeat(rating as usize)
                                                    )
                                                } else {
                                                    format!("{} - Rate this option", truncated_name)
                                                })
                                                .options(|opts| {
                                                    opts.create_option(|opt| {
                                                        opt.label(format!(
                                                            "{} - No rating",
                                                            short_name
                                                        ))
                                                        .description(format!(
                                                            "{} - Clear rating",
                                                            truncated_name
                                                        ))
                                                        .value("0".to_string())
                                                        .default_selection(rating == 0)
                                                    });
                                                    for i in 1..=5 {
                                                        let stars = "⭐".repeat(i as usize);
                                                        opts.create_option(|opt| {
                                                            opt.label(format!(
                                                                "{} {}",
                                                                short_name, stars
                                                            ))
                                                            .description(format!(
                                                                "{} - {} stars",
                                                                truncated_name, i
                                                            ))
                                                            .value(i.to_string())
                                                            .default_selection(rating == i)
                                                        });
                                                    }
                                                    opts
                                                })
                                            })
                                        });
                                    }

                                    c.create_action_row(|row| {
                                        if page > 0 {
                                            row.create_button(|btn| {
                                                btn.custom_id(format!(
                                                    "starPage_{}_{}",
                                                    poll.id,
                                                    page - 1
                                                ))
                                                .label("◀ Previous")
                                                .style(ButtonStyle::Secondary)
                                            });
                                        }
                                        row.create_button(|btn| {
                                            btn.custom_id(format!("doneVoting_{}", poll.id))
                                                .label("Done Voting")
                                                .style(ButtonStyle::Success)
                                        });
                                        if page < total_pages - 1 {
                                            row.create_button(|btn| {
                                                btn.custom_id(format!(
                                                    "starPage_{}_{}",
                                                    poll.id,
                                                    page + 1
                                                ))
                                                .label("Next ▶")
                                                .style(ButtonStyle::Secondary)
                                            });
                                        }
                                        row
                                    });
                                    c
                                })
                        })
                })
                .await?;
        }
        crate::models::VotingMethod::Plurality => {
            component
                .create_interaction_response(&ctx.http, |response| {
                    response
                        .kind(InteractionResponseType::ChannelMessageWithSource)
                        .interaction_response_data(|message| {
                            message
                                .ephemeral(true)
                                .content(format!("**{}**\nSelect ONE option:", poll.question))
                                .components(|c| {
                                    let mut options_iter = poll.options.iter().peekable();
                                    while options_iter.peek().is_some() {
                                        c.create_action_row(|row| {
                                            for _ in 0..5 {
                                                if let Some(option) = options_iter.next() {
                                                    let selected = option_ratings
                                                        .get(&option.id)
                                                        .copied()
                                                        .unwrap_or(0)
                                                        > 0;
                                                    let style = if selected {
                                                        ButtonStyle::Success
                                                    } else {
                                                        ButtonStyle::Primary
                                                    };
                                                    let prefix = if selected { "✓ " } else { "" };

                                                    row.create_button(|btn| {
                                                        btn.custom_id(format!(
                                                            "pluralityVote_{}_{}",
                                                            poll.id, option.id
                                                        ))
                                                        .label(format!("{}{}", prefix, option.text))
                                                        .style(style)
                                                    });
                                                } else {
                                                    break;
                                                }
                                            }
                                            row
                                        });
                                    }
                                    c.create_action_row(|row| {
                                        row.create_button(|btn| {
                                            btn.custom_id(format!("doneVoting_{}", poll.id))
                                                .label("Done Voting")
                                                .style(ButtonStyle::Success)
                                        })
                                    });
                                    c
                                })
                        })
                })
                .await?;
        }
        crate::models::VotingMethod::Approval => {
            component
                .create_interaction_response(&ctx.http, |response| {
                    response
                        .kind(InteractionResponseType::ChannelMessageWithSource)
                        .interaction_response_data(|message| {
                            message
                                .ephemeral(true)
                                .content(format!(
                                    "**{}**\nApprove as many options as you like:",
                                    poll.question
                                ))
                                .components(|c| {
                                    let mut options_iter = poll.options.iter().peekable();
                                    while options_iter.peek().is_some() {
                                        c.create_action_row(|row| {
                                            for _ in 0..5 {
                                                if let Some(option) = options_iter.next() {
                                                    let value = option_ratings
                                                        .get(&option.id)
                                                        .copied()
                                                        .unwrap_or(0);
                                                    let display_symbol =
                                                        if value > 0 { "✅" } else { "❌" };
                                                    let button_style = if value > 0 {
                                                        ButtonStyle::Success
                                                    } else {
                                                        ButtonStyle::Danger
                                                    };

                                                    row.create_button(|btn| {
                                                        btn.custom_id(format!(
                                                            "approvalVote_{}_{}_{}",
                                                            poll.id, option.id, value
                                                        ))
                                                        .label(format!(
                                                            "{} {}",
                                                            display_symbol, option.text
                                                        ))
                                                        .style(button_style)
                                                    });
                                                } else {
                                                    break;
                                                }
                                            }
                                            row
                                        });
                                    }
                                    c.create_action_row(|row| {
                                        row.create_button(|btn| {
                                            btn.custom_id(format!("doneVoting_{}", poll.id))
                                                .label("Done Voting")
                                                .style(ButtonStyle::Success)
                                        })
                                    });
                                    c
                                })
                        })
                })
                .await?;
        }
        crate::models::VotingMethod::Ranked => {
            // Paginate ranked UI like STAR so we don't exceed Discord's 5 action row limit
            let page = if component.data.custom_id.starts_with("rankPage_") {
                component
                    .data
                    .custom_id
                    .split('_')
                    .next_back()
                    .and_then(|s| s.parse::<usize>().ok())
                    .unwrap_or(0)
            } else if component.data.custom_id.starts_with("rank_page_") {
                component
                    .data
                    .custom_id
                    .split('_')
                    .next_back()
                    .and_then(|s| s.parse::<usize>().ok())
                    .unwrap_or(0)
            } else {
                0
            };

            let options_per_page = 4;
            let total_pages = poll.options.len().div_ceil(options_per_page);
            let start_idx = page * options_per_page;
            let end_idx = std::cmp::min(start_idx + options_per_page, poll.options.len());
            let options_to_show = &poll.options[start_idx..end_idx];

            let user_id = component.user.id.to_string();
            let existing_votes = database.get_user_poll_votes(&poll.id, &user_id).await?;
            let mut option_ranks: std::collections::HashMap<String, i32> =
                std::collections::HashMap::new();
            for vote in existing_votes {
                option_ranks.insert(vote.option_id, vote.rating);
            }

            component
                .create_interaction_response(&ctx.http, |response| {
                    response
                        .kind(InteractionResponseType::ChannelMessageWithSource)
                        .interaction_response_data(|message| {
                            message
                                .ephemeral(true)
                                .content(format!("**{}**\nRank the options in your order of preference:\nPage {} of {}", poll.question, page + 1, total_pages))
                                .components(|c| {
                                    for option in options_to_show {
                                        let current_rank = option_ranks.get(&option.id).unwrap_or(&0);
                                        let display_text = if *current_rank > 0 {
                                            format!("#{} - {}", current_rank, option.text)
                                        } else {
                                            format!("Unranked - {}", option.text)
                                        };
                                        c.create_action_row(|row| {
                                            row.create_button(|btn| {
                                                btn.custom_id(format!("rankLabel_{}_{}", poll.id, option.id))
                                                   .label(&display_text)
                                                   .style(ButtonStyle::Secondary)
                                                   .disabled(true)
                                            })
                                            .create_button(|btn| {
                                                btn.custom_id(format!("rankUp_{}_{}", poll.id, option.id))
                                                   .emoji('⬆')
                                                   .style(ButtonStyle::Primary)
                                                   .disabled(*current_rank == 1)
                                            })
                                            .create_button(|btn| {
                                                let is_at_bottom = *current_rank == option_ranks.values().filter(|&&r| r > 0).count() as i32;
                                                let is_unranked = *current_rank == 0;
                                                btn.custom_id(format!("rankDown_{}_{}", poll.id, option.id))
                                                   .emoji('⬇')
                                                   .style(ButtonStyle::Primary)
                                                   .disabled(is_unranked || is_at_bottom)
                                            })
                                            .create_button(|btn| {
                                                btn.custom_id(format!("rankRemove_{}_{}", poll.id, option.id))
                                                   .emoji('🗑')
                                                   .style(ButtonStyle::Danger)
                                                   .disabled(*current_rank == 0)
                                            })
                                        });
                                    }

                                    // Navigation / done row
                                    c.create_action_row(|row| {
                                        if page > 0 {
                                            row.create_button(|btn| {
                                                btn.custom_id(format!("rankPage_{}_{}", poll.id, page - 1))
                                                   .label("◀ Previous")
                                                   .style(ButtonStyle::Secondary)
                                            });
                                        }
                                        row.create_button(|btn| {
                                            btn.custom_id(format!("doneVoting_{}", poll.id))
                                               .label("Done Voting")
                                               .style(ButtonStyle::Success)
                                        });
                                        if page < total_pages - 1 {
                                            row.create_button(|btn| {
                                                btn.custom_id(format!("rankPage_{}_{}", poll.id, page + 1))
                                                   .label("Next ▶")
                                                   .style(ButtonStyle::Secondary)
                                            });
                                        }
                                        row
                                    });

                                    c
                                })
                        })
                })
                .await?;
        }
    }
    Ok(())
}

pub async fn handle_star_vote(
    database: &Database,
    ctx: &Context,
    component: &MessageComponentInteraction,
    poll_id: &str,
    option_id: &str,
    rating: i32,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!(
        "Recording star vote: poll_id={}, option_id={}, rating={}",
        poll_id, option_id, rating
    );

    if !(0..=5).contains(&rating) {
        warn!("Rating out of 0-5 range: {}", rating);
        return Ok(());
    }

    let vote = crate::models::Vote {
        user_id: component.user.id.to_string(),
        poll_id: poll_id.to_string(),
        option_id: option_id.to_string(),
        rating,
        timestamp: Utc::now(),
    };

    database.save_vote(&vote).await?;
    let poll = database.get_poll(poll_id).await?;

    // Find which page this option is on
    let options_per_page = 4;
    let option_index = poll
        .options
        .iter()
        .position(|o| o.id == option_id)
        .unwrap_or(0);
    let current_page = option_index / options_per_page;

    let total_pages = poll.options.len().div_ceil(options_per_page);
    let start_idx = current_page * options_per_page;
    let end_idx = std::cmp::min(start_idx + options_per_page, poll.options.len());

    let options_to_show = &poll.options[start_idx..end_idx];
    let pagination_info = if total_pages > 1 {
        format!(
            "\nPage {} of {} - Rate each option from 0-5 stars",
            current_page + 1,
            total_pages
        )
    } else {
        String::from("\nRate each option from 0-5 stars")
    };

    let existing_votes = database
        .get_user_poll_votes(&poll.id, &component.user.id.to_string())
        .await?;
    let mut option_ratings = std::collections::HashMap::<String, i32>::new();
    for v in &existing_votes {
        option_ratings.insert(v.option_id.clone(), v.rating);
    }

    component
        .create_interaction_response(&ctx.http, |resp| {
            resp.kind(InteractionResponseType::UpdateMessage)
                .interaction_response_data(|msg| {
                    msg.ephemeral(true)
                        .content(format!("**{}**{}", poll.question, pagination_info))
                        .components(|c| {
                            for option in options_to_show {
                                let rating = option_ratings.get(&option.id).copied().unwrap_or(0);
                                let truncated_name = if option.text.len() > 30 {
                                    format!("{}...", &option.text[..27])
                                } else {
                                    option.text.clone()
                                };
                                c.create_action_row(|row| {
                                    row.create_select_menu(|menu| {
                                        menu.custom_id(format!(
                                            "starSelect_{}_{}",
                                            poll.id, option.id
                                        ))
                                        .placeholder(if rating > 0 {
                                            format!(
                                                "{} - {}",
                                                truncated_name,
                                                "⭐".repeat(rating as usize)
                                            )
                                        } else {
                                            format!("{} - Rate this option", truncated_name)
                                        })
                                        .options(|opts| {
                                            opts.create_option(|opt| {
                                                opt.label(format!("{} - No rating", truncated_name))
                                                    .description("Clear rating")
                                                    .value("0".to_string())
                                                    .default_selection(rating == 0)
                                            });
                                            for i in 1..=5 {
                                                let stars = "⭐".repeat(i as usize);
                                                opts.create_option(|opt| {
                                                    opt.label(format!(
                                                        "{} - {}",
                                                        truncated_name, stars
                                                    ))
                                                    .description(format!("Set rating to {}", i))
                                                    .value(i.to_string())
                                                    .default_selection(rating == i)
                                                });
                                            }
                                            opts
                                        })
                                    })
                                });
                            }
                            c.create_action_row(|row| {
                                if current_page > 0 {
                                    row.create_button(|btn| {
                                        btn.custom_id(format!(
                                            "starPage_{}_{}",
                                            poll.id,
                                            current_page - 1
                                        ))
                                        .label("◀ Previous")
                                        .style(ButtonStyle::Secondary)
                                    });
                                }
                                row.create_button(|btn| {
                                    btn.custom_id(format!("doneVoting_{}", poll.id))
                                        .label("Done Voting")
                                        .style(ButtonStyle::Success)
                                });
                                if current_page < total_pages - 1 {
                                    row.create_button(|btn| {
                                        btn.custom_id(format!(
                                            "starPage_{}_{}",
                                            poll.id,
                                            current_page + 1
                                        ))
                                        .label("Next ▶")
                                        .style(ButtonStyle::Secondary)
                                    });
                                }
                                row
                            });
                            c
                        })
                })
        })
        .await?;

    info!("Successfully recorded vote and updated UI");
    Ok(())
}

pub async fn handle_star_select(
    database: &Database,
    ctx: &Context,
    component: &MessageComponentInteraction,
    poll_id: &str,
    option_id: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let rating = component
        .data
        .values
        .first()
        .and_then(|v| v.parse::<i32>().ok())
        .unwrap_or(0);

    handle_star_vote(database, ctx, component, poll_id, option_id, rating).await
}

pub async fn handle_plurality_vote(
    database: &Database,
    ctx: &Context,
    component: &MessageComponentInteraction,
    poll_id: &str,
    option_id: &str,
    poll: &Poll,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!(
        "Recording plurality vote: poll_id={}, option_id={}",
        poll_id, option_id
    );

    let user_id = component.user.id.to_string();
    for option in &poll.options {
        let rating = if option.id == option_id { 1 } else { 0 };

        let vote = crate::models::Vote {
            user_id: user_id.clone(),
            poll_id: poll_id.to_string(),
            option_id: option.id.clone(),
            rating,
            timestamp: Utc::now(),
        };
        database.save_vote(&vote).await?;
    }

    let existing_votes = database.get_user_poll_votes(poll_id, &user_id).await?;
    let mut option_ratings = std::collections::HashMap::<String, i32>::new();
    for v in &existing_votes {
        option_ratings.insert(v.option_id.clone(), v.rating);
    }

    component
        .create_interaction_response(&ctx.http, |response| {
            response
                .kind(InteractionResponseType::UpdateMessage)
                .interaction_response_data(|message| {
                    message
                        .ephemeral(true)
                        .content(format!("**{}**\nSelect ONE option:", poll.question))
                        .components(|c| {
                            let mut options_iter = poll.options.iter().peekable();
                            while options_iter.peek().is_some() {
                                c.create_action_row(|row| {
                                    for _ in 0..5 {
                                        if let Some(option) = options_iter.next() {
                                            let selected = option_ratings
                                                .get(&option.id)
                                                .copied()
                                                .unwrap_or(0)
                                                > 0;
                                            let style = if selected {
                                                ButtonStyle::Success
                                            } else {
                                                ButtonStyle::Primary
                                            };
                                            let prefix = if selected { "✓ " } else { "" };

                                            row.create_button(|btn| {
                                                btn.custom_id(format!(
                                                    "pluralityVote_{}_{}",
                                                    poll_id, option.id
                                                ))
                                                .label(format!("{}{}", prefix, option.text))
                                                .style(style)
                                            });
                                        } else {
                                            break;
                                        }
                                    }
                                    row
                                });
                            }
                            c.create_action_row(|row| {
                                row.create_button(|btn| {
                                    btn.custom_id(format!("doneVoting_{}", poll_id))
                                        .label("Done Voting")
                                        .style(ButtonStyle::Success)
                                })
                            });
                            c
                        })
                })
        })
        .await?;

    info!("Successfully recorded plurality vote and updated UI");
    Ok(())
}

pub async fn handle_approval_vote_toggle(
    database: &Database,
    ctx: &Context,
    component: &MessageComponentInteraction,
    poll_id: &str,
    option_id: &str,
    current_value: i32,
    poll: &Poll,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let new_value = 1 - current_value;
    let display_symbol = if new_value == 1 { "✅" } else { "❌" };
    let button_style = if new_value == 1 {
        ButtonStyle::Success
    } else {
        ButtonStyle::Danger
    };

    let option_text = poll
        .options
        .iter()
        .find(|o| o.id == option_id)
        .map(|o| o.text.clone())
        .unwrap_or_else(|| "Option".to_string());

    let vote = crate::models::Vote {
        user_id: component.user.id.to_string(),
        poll_id: poll_id.to_string(),
        option_id: option_id.to_string(),
        rating: new_value,
        timestamp: Utc::now(),
    };
    database.save_vote(&vote).await?;

    component
        .create_interaction_response(&ctx.http, |response| {
            response
                .kind(InteractionResponseType::UpdateMessage)
                .interaction_response_data(|message| {
                    message.ephemeral(true).components(|c| {
                        for row_data in &component.message.components {
                            c.create_action_row(|ar| {
                                for comp_data in &row_data.components {
                                    if let ActionRowComponent::Button(button_data) = comp_data {
                                        if button_data.custom_id.as_deref()
                                            == Some(&component.data.custom_id)
                                        {
                                            ar.create_button(|b| {
                                                b.custom_id(format!(
                                                    "approvalVote_{}_{}_{}",
                                                    poll_id, option_id, new_value
                                                ))
                                                .label(format!(
                                                    "{} {}",
                                                    display_symbol, option_text
                                                ))
                                                .style(button_style)
                                            });
                                        } else {
                                            ar.create_button(|b| {
                                                b.custom_id(
                                                    button_data
                                                        .custom_id
                                                        .clone()
                                                        .unwrap_or_default(),
                                                )
                                                .label(
                                                    button_data.label.clone().unwrap_or_default(),
                                                )
                                                .style(button_data.style)
                                                .disabled(button_data.disabled)
                                            });
                                        }
                                    }
                                }
                                ar
                            });
                        }
                        c
                    })
                })
        })
        .await?;

    Ok(())
}

pub fn format_user_vote(poll: &Poll, user_votes: &[crate::models::Vote]) -> String {
    let mut vote_summary = String::new();

    match poll.voting_method {
        crate::models::VotingMethod::Star => {
            let mut vote_map = std::collections::HashMap::new();
            for v in user_votes {
                vote_map.insert(v.option_id.clone(), v.rating);
            }
            for option in &poll.options {
                let rating = vote_map.get(&option.id).cloned().unwrap_or(0);
                let stars = "⭐".repeat(rating as usize);
                vote_summary.push_str(&format!("• **{}**: {}\n", option.text, stars));
            }
        }
        crate::models::VotingMethod::Plurality => {
            for option in &poll.options {
                let selected = user_votes
                    .iter()
                    .any(|v| v.option_id == option.id && v.rating > 0);
                let symbol = if selected { "✓" } else { " " };
                if selected {
                    vote_summary.push_str(&format!("• **{}**: {}\n", option.text, symbol));
                } else {
                    vote_summary.push_str(&format!("• {}: {}\n", option.text, symbol));
                }
            }
        }
        crate::models::VotingMethod::Approval => {
            let mut vote_map = std::collections::HashMap::new();
            for v in user_votes {
                vote_map.insert(v.option_id.clone(), v.rating);
            }
            for option in &poll.options {
                let approved = vote_map.get(&option.id).copied().unwrap_or(0) == 1;
                let symbol = if approved { "✅" } else { "❌" };
                vote_summary.push_str(&format!("• **{}**: {}\n", option.text, symbol));
            }
        }
        crate::models::VotingMethod::Ranked => {
            let mut rankings = std::collections::HashMap::new();
            for v in user_votes {
                if v.rating > 0 {
                    rankings.insert(v.option_id.clone(), v.rating);
                }
            }
            let mut ranked_options = poll.options.clone();
            ranked_options.sort_by(|a, b| {
                let rank_a = rankings.get(&a.id).unwrap_or(&0);
                let rank_b = rankings.get(&b.id).unwrap_or(&0);
                if rank_a == &0 && rank_b == &0 {
                    return a.text.cmp(&b.text);
                }
                if rank_a == &0 {
                    return std::cmp::Ordering::Greater;
                }
                if rank_b == &0 {
                    return std::cmp::Ordering::Less;
                }
                rank_a.cmp(rank_b)
            });
            for option in &ranked_options {
                let rank = rankings.get(&option.id).cloned().unwrap_or(0);
                if rank > 0 {
                    vote_summary.push_str(&format!("• **#{}**: {}\n", rank, option.text));
                } else {
                    vote_summary.push_str(&format!("• Unranked: {}\n", option.text));
                }
            }
        }
    }
    vote_summary
}

pub async fn handle_done_voting(
    database: &Database,
    ctx: &Context,
    component: &MessageComponentInteraction,
    poll_id: &str,
    poll: &Poll,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!("User completed voting for poll_id={}", poll_id);
    let user_id = component.user.id.to_string();
    let user_votes = database.get_user_poll_votes(poll_id, &user_id).await?;

    let vote_details = format_user_vote(poll, &user_votes);
    let vote_summary = format!(
        "**{}**\n{} Voting\n\nYour vote has been recorded:\n{}",
        poll.question, poll.voting_method, vote_details
    );

    component
        .create_interaction_response(&ctx.http, |response| {
            response
                .kind(InteractionResponseType::UpdateMessage)
                .interaction_response_data(|message| {
                    message.content(&vote_summary).components(|c| {
                        c.create_action_row(|row| {
                            row.create_button(|btn| {
                                btn.custom_id(format!("voteChange_{}", poll_id))
                                    .label("Change My Vote")
                                    .style(ButtonStyle::Secondary)
                            });
                            if poll.allow_vote_sharing {
                                row.create_button(|btn| {
                                    btn.custom_id(format!("shareVote_{}", poll_id))
                                        .label("Share My Vote")
                                        .style(ButtonStyle::Primary)
                                });
                            }
                            row
                        })
                    })
                })
        })
        .await?;

    info!("Successfully displayed vote confirmation");
    Ok(())
}

pub async fn handle_change_vote(
    database: &Database,
    ctx: &Context,
    component: &MessageComponentInteraction,
    poll_id: &str,
    poll: &Poll,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!("User wants to change their vote for poll_id={}", poll_id);
    handle_vote_button(database, ctx, component, poll).await
}

pub async fn handle_rank_action(
    database: &Database,
    ctx: &Context,
    component: &MessageComponentInteraction,
    option_id: &str,
    action: &str,
    poll: &Poll,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let user_id = component.user.id.to_string();
    let existing_votes = database.get_user_poll_votes(&poll.id, &user_id).await?;
    let mut rankings = std::collections::HashMap::new();

    for vote in &existing_votes {
        if vote.rating > 0 {
            rankings.insert(vote.option_id.clone(), vote.rating);
        }
    }

    let current_rank = rankings.get(option_id).cloned().unwrap_or(0);

    match action {
        "up" => {
            if current_rank == 0 {
                let next_rank = rankings.values().max().unwrap_or(&0) + 1;
                rankings.insert(option_id.to_string(), next_rank);
            } else if current_rank > 1 {
                for (k, v) in rankings.clone() {
                    if v == current_rank - 1 {
                        rankings.insert(k, current_rank);
                    }
                }
                rankings.insert(option_id.to_string(), current_rank - 1);
            }
        }
        "down" => {
            if current_rank > 0 {
                let max_used = rankings.values().max().unwrap_or(&0).to_owned();
                if current_rank < max_used {
                    for (k, v) in rankings.clone() {
                        if v == current_rank + 1 {
                            rankings.insert(k, current_rank);
                        }
                    }
                    rankings.insert(option_id.to_string(), current_rank + 1);
                } else {
                    rankings.insert(option_id.to_string(), max_used + 1);
                }
            } else {
                let new_rank = rankings.values().max().unwrap_or(&0) + 1;
                rankings.insert(option_id.to_string(), new_rank);
            }
        }
        "remove" => {
            if current_rank > 0 {
                rankings.remove(option_id);
                let mut items: Vec<_> = rankings.into_iter().collect();
                items.sort_by_key(|(_, r)| *r);
                let mut rank_count = 1;
                rankings = std::collections::HashMap::new();
                for (k, _) in items {
                    rankings.insert(k, rank_count);
                    rank_count += 1;
                }
            }
        }
        _ => {}
    }

    // Build all votes and save in a single transaction (batch operation)
    let now = Utc::now();
    let all_votes: Vec<crate::models::Vote> = poll
        .options
        .iter()
        .map(|opt| crate::models::Vote {
            user_id: user_id.clone(),
            poll_id: poll.id.clone(),
            option_id: opt.id.clone(),
            rating: rankings.get(&opt.id).copied().unwrap_or(0),
            timestamp: now,
        })
        .collect();

    database.save_votes_batch(&all_votes).await?;

    handle_vote_button(database, ctx, component, poll).await
}
