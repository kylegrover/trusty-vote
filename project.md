# Trusty Vote: Discord Voting Bot

## Project Overview

Trusty Vote (previously Rusty Bote) is a lightweight Discord bot written in Rust for running polls with alternative voting methods: STAR, Plurality, Ranked Choice, and Approval. It is live and deployed on Railway, serving real Discord servers.

The bot is one part of a three-repo system (bot, website, API), but only this repo is required for self-hosting.

## Deployment

- **Hosting:** Railway with auto-deploy
- **Branch flow:** feature branches -> `dev` (staging auto-deploy) -> `master` (production auto-deploy)
- **Architecture:** Single bot instance + single PostgreSQL database (intentional, not a limitation)

## Technology Stack

- **Language:** Rust (edition 2024)
- **Discord API:** Serenity 0.11
- **Database:** PostgreSQL via SQLx 0.8
- **Async Runtime:** Tokio
- **Optional:** Embedded PostgreSQL for local dev (`cargo run --features embedded-postgres`)

## Source Structure

```
src/
├── main.rs              # Bot init, event loop, command registration
├── commands/
│   ├── mod.rs           # Re-exports
│   └── poll.rs          # All /poll subcommand handlers (~1000 lines, largest file)
├── db/
│   └── mod.rs           # PostgreSQL schema, queries, connection pooling
├── handlers/
│   ├── mod.rs           # Interaction routing, component dispatch
│   └── vote.rs          # Per-method voting UI handlers
├── models/
│   ├── mod.rs           # Poll, Vote, PollOption, VotingMethod structs
│   └── tests.rs         # Model unit tests
├── tasks/
│   ├── mod.rs           # Re-exports
│   └── poll_ender.rs    # Background task checking expired polls every 60s
├── voting/
│   ├── mod.rs           # PollResults, VoteCount structs
│   ├── star.rs          # STAR tally (scoring + runoff)
│   ├── plurality.rs     # Plurality tally
│   ├── ranked.rs        # Ranked choice instant runoff
│   ├── approval.rs      # Approval tally
│   └── tests.rs         # Voting method unit tests
└── utils/
    └── mod.rs           # Placeholder (empty)
```

## Database Schema

Three tables, created inline at startup via `CREATE TABLE IF NOT EXISTS` (no migration tool yet):

- **polls** — Poll metadata (question, method, timestamps, active status, role restrictions, vote sharing flag)
- **poll_options** — Options per poll with position ordering
- **votes** — Per-user per-option ratings. PK: (user_id, poll_id, option_id). Supports upsert for vote changes.

## Discord Commands

| Command | Description | Permissions |
|---|---|---|
| `/poll create` | Create poll with question, options, method, optional duration/role/sharing | Any user |
| `/poll end [id]` | End an active poll | Creator or admin |
| `/poll results [id]` | View results of ended poll | Any user |
| `/poll list` | List active and recently ended polls | Any user |
| `/poll help` | Usage guide | Any user |
| `/poll export [id]` | Export votes as CSV file attachment | Creator or admin |

### Poll Creation Parameters

- `question` — Poll question (required)
- `options` — Comma-separated choices, 2-10 (required)
- `method` — STAR, Plurality, Ranked Choice, or Approval (required)
- `duration` — Minutes until auto-close, default 1440 (24h), 0 = manual close (optional)
- `allowed_role` — Restrict voting to a single role (optional)
- `share_vote` — Enable vote sharing button (optional)

## Voting Methods

### STAR (Score Then Automatic Runoff)
- Users rate each option 0-5 stars via select menus (paginated, 4 options per page)
- Scoring phase: sum all ratings per option
- Runoff phase: top 2 by score, each voter's preference compared, most-preferred wins
- **Known issue:** Tie-breaking is currently non-deterministic (HashMap iteration order). Plan: port [tim-one/startie](https://github.com/tim-one/startie) to Rust (fork at kylegrover/startie).

### Plurality
- Users click one button to vote
- Simple count, highest wins
- Ties resolved arbitrarily (same HashMap issue)

### Ranked Choice (Instant Runoff)
- Users arrange preferences with up/down/remove buttons (paginated)
- Rounds: count first preferences, check majority (>50% of all voters), eliminate lowest, repeat
- Eliminates ALL tied-lowest candidates per round
- Exhausted ballots don't reduce the majority threshold
- Safety break prevents infinite loops

### Approval
- Users toggle approve/disapprove per option
- Count approvals, highest wins

## Interaction Architecture

- Custom IDs follow format: `actionName_pollId_optionId[_additionalData]`
- All vote interactions are ephemeral (private to voter)
- Component routing in `handlers/mod.rs` dispatches ~15+ custom_id prefixes
- Role restrictions enforced at the handler level before reaching voting logic
- Votes saved atomically; batch saves use transactions

## Test Coverage

**15 passing tests** (`cargo test`):

- 2 model tests (default duration, manual close)
- 4 STAR tests (scoring+runoff, runoff tie, skipped options, equal scores)
- 3 Plurality tests (no votes, counting, ties)
- 2 Approval tests (counting, all-approve)
- 4 Ranked Choice tests (elimination to majority, exhausted ballots, unbreakable tie, duplicate rankings)

**Not tested:** Database operations, command validation, interaction routing, CSV export, poll lifecycle (ending/expiration), permission enforcement.

## Current Gaps

Refer to ROADMAP.md for the full plan. Key gaps as of now:

1. **No CI** — No GitHub Actions workflow. Build/test failures can ship unnoticed.
2. **No database migrations** — Schema is inline in code. First schema evolution with existing deployments will need care.
3. **Non-deterministic tie-breaking** — HashMap order for STAR and Plurality ties. Startie port will fix this.
4. **Thin test coverage** — Voting tally logic is well-tested. Everything else (DB, commands, handlers, permissions, exports) is untested.
5. **Dead code warnings** — `winner_id` and `raw_results` fields in PollResults are unused.
6. **utils/mod.rs** is an empty placeholder.

## What's Complete

- All 4 voting methods fully implemented with interactive UIs
- Poll lifecycle: create, vote, end (manual + auto), results, list, export
- CSV export as Discord file attachment
- Role-based poll access control (single role)
- Vote sharing (optional per poll)
- Self-hosting documentation + Docker Compose example
- Embedded Postgres for local dev
- Permission model for end/export (creator or admin)
- Background poll expiration task (60s interval)

## Related Repositories

- **Website** — Separate repo, live (not required for self-hosting)
- **API** — Separate repo, live (not required for self-hosting)
- **Startie** — Forked to kylegrover/startie, will be ported to Rust for STAR tie-breaking
