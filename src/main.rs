mod commands;
mod db;
mod handlers;
mod models;
mod tasks;
mod voting;

use db::Database;
use log::{error, info};
use serenity::async_trait;
use serenity::model::application::command::Command;
use serenity::model::application::interaction::Interaction;
use serenity::model::gateway::Ready;
use serenity::prelude::*;
use std::env;
use std::sync::Arc;

// Custom ID format for components:
// We use camelCase format for action names (e.g., starSelect, pluralityVote)
// Format: actionName_pollId_optionId[_additionalData]
// This makes parsing poll IDs more consistent (always at index 1)

struct Bot {
    database: Arc<Database>,
}

#[async_trait]
impl EventHandler for Bot {
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        // Clone Arc for the handler
        let db = Arc::clone(&self.database);
        let ctx_clone = ctx.clone();

        // Spawn a task to handle the interaction concurrently
        tokio::spawn(async move {
            handlers::handle_interaction(&db, &ctx_clone, interaction).await;
        });
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        info!("{} is connected!", ready.user.name);

        // Register slash commands globally or for specific guilds
        let commands = Command::set_global_application_commands(&ctx.http, |commands_builder| {
            commands_builder
                .create_application_command(|command| commands::poll::create_poll_command(command))
            // Add other commands here
        })
        .await;

        if let Err(why) = commands {
            error!("Failed to register slash commands: {:?}", why);
        } else {
            info!("Successfully registered global slash commands.");
        }

        // --- Start Background Task for Ending Polls ---
        let db_clone = Arc::clone(&self.database);
        let ctx_clone = ctx.clone();
        tokio::spawn(async move {
            tasks::poll_ender::check_expired_polls_task(db_clone, ctx_clone).await;
        });
        // --- End Background Task ---
    }
}

#[tokio::main]
async fn main() {
    // Initialize logging
    dotenvy::dotenv().ok();
    env_logger::init();

    // Load token from environment variable
    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");

    // Initialize database
    let database = match Database::new().await {
        Ok(db) => Arc::new(db),
        Err(e) => {
            error!("Failed to initialize database: {}", e);
            return;
        }
    };

    // Define intents
    let intents = GatewayIntents::GUILDS
        | GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::GUILD_INTEGRATIONS; // Add necessary intents

    // Build client
    let mut client = Client::builder(&token, intents)
        .event_handler(Bot { database })
        .await
        .expect("Err creating client");

    // Start client
    if let Err(why) = client.start().await {
        error!("Client error: {:?}", why);
    }
}
