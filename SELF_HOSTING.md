# Self-Hosting

Trusty Vote is designed to run as a single bot instance backed by a single PostgreSQL database.

For a self-hosted deployment, you only need this repository and a PostgreSQL database. The separate website and API repositories used for the hosted instance are not required.

## What You Need

- A Discord application and bot token
- A PostgreSQL database
- Rust and Cargo, or Docker with Docker Compose

## Discord Setup

1. Create a new application in the Discord developer portal.
2. Add a bot user to the application.
3. Copy the bot token into your environment configuration.
4. Invite the bot to your server.

Recommended bot permissions:

- View Channel
- Send Messages
- Embed Links
- Read Message History
- Manage Messages

## Environment Variables

Use [.env.example](.env.example) as a starting point.

Required values:

- `DISCORD_TOKEN`
- `DATABASE_URL`

Optional values:

- `RUST_LOG`

Example PostgreSQL URL:

```env
DATABASE_URL=postgres://postgres:postgres@localhost:5432/trusty_vote
```

## Option 1: Docker Compose

This is the simplest demonstration deployment for other operators.

1. Copy [.env.example](.env.example) to `.env` and fill in `DISCORD_TOKEN`.
2. Start the stack:

```bash
docker compose up --build
```

This starts:

- one PostgreSQL container
- one Trusty Vote container

The bot will create its tables on startup.

## Option 2: Local Process Run

1. Start a PostgreSQL database.
2. Copy [.env.example](.env.example) to `.env` and fill in the values.
3. Start the bot:

```bash
cargo run
```

For local-only development, you can still use the embedded Postgres feature instead of an external database:

```bash
cargo run --features embedded-postgres
```

That mode is for development convenience and is not the recommended self-hosted production path.

## Startup Checklist

When the bot starts successfully, you should expect:

- the process to connect to PostgreSQL
- slash commands to register
- the poll-ending background task to start
- the bot to appear online in your server

## Notes

- Trusty Vote is intentionally a single-instance deployment.
- If you want to operate multiple independent deployments, run separate bot instances with separate databases.
- There is currently no requirement to run the hosted service website or API alongside the bot.