# Trusty Vote Roadmap

## Product Direction

Trusty Vote is a Discord-first polling bot.

The primary product is the bot experience inside Discord after a server adds the app. There is no web login flow and no plan to make the website the primary interface. The separate API and website repos support the hosted service, but they are not required for a self-hosted deployment of the bot.

The intended deployment model is a single bot instance backed by a single PostgreSQL database. That is a deliberate choice, not a temporary limitation. Horizontal growth can happen by running separate isolated deployments for different communities or operators rather than building a shared global control plane.

## Planning Principles

- Keep operating cost low enough for the hosted public instance to remain practical.
- Prioritize correctness and trust over adding many new features.
- Keep self-hosting simple and honest: this repo plus PostgreSQL should be enough.
- Avoid premature distributed-systems work.
- Extract reusable voting logic only where it meaningfully improves reliability and reuse.

## Scope Boundaries

### In Scope for This Repo

- Discord bot runtime
- Poll creation, voting, closing, and results
- Vote storage in PostgreSQL
- Self-hosting documentation for one bot instance and one database
- Testing for voting logic and poll lifecycle behavior

### Out of Scope for Self-Hosted Basics

- Separate website repo
- Separate API repo
- Web accounts or login
- Multi-instance coordination against one shared database
- Centralized hosting control plane

## Current Gaps To Address

- Database schema setup is implicit in code instead of managed with explicit migrations.
- CI and release-quality verification are missing.
- ~~Non-deterministic tie-breaking in STAR and Plurality.~~ ✓ Fixed via startie port.
- Test coverage exists for voting tally logic but not for DB, commands, handlers, permissions, or exports.

## Roadmap

## Phase 1: Operational Baseline

Goal: make the bot easier to trust, easier to run, and easier to understand without changing the single-instance architecture.

### 1. Self-Hosting Basics ✓

Done. See SELF_HOSTING.md, docker-compose.yml, and readme.md.

### 2. Database Hygiene

- Move schema management to explicit SQLx migrations.
- Document upgrade expectations for existing deployments.
- Add a short backup and restore guide suitable for a small hosted service.
- Add indexes only where they clearly improve active-poll and poll-results queries.

### 3. Runtime Observability

- Add a small startup checklist in docs: required env vars, DB connectivity, command registration, expected logs.
- Improve operational logging around poll creation, vote saving, poll ending, and export actions.
- Add a minimal health-check story where practical, even if it is only documented startup validation and a simple API-less readiness approach.

## Phase 2: Correctness And Testing

Goal: make voting behavior defensible and regression-resistant.

STAR tie-breaking: ✓ Ported [kylegrover/startie](https://github.com/kylegrover/startie) to Rust. Integrated into STAR and Plurality voting.

### 1. Voting Method Tests (partial ✓)

15 unit tests exist covering STAR (scoring+runoff, ties, skipped options, equal scores), Plurality (no votes, counting, ties), Approval (counting, all-approve), Ranked Choice (elimination, exhausted ballots, unbreakable tie, duplicate rankings), and model construction.

Remaining:
- Build fixed fixtures for additional edge cases (partial ballots, more tie scenarios).
- Add regression tests for ranked-choice elimination order and STAR runoff behavior.
- Assert exact result summaries where summaries are user-visible contract.

### 2. Poll Lifecycle Tests

- Add tests for poll creation constraints such as option count and duration behavior.
- Add tests for ending polls and rejecting late votes.
- Add tests for role-restricted polls and vote-sharing behavior.
- Add tests for export authorization so only poll creators or admins can export.

### 3. Verification In CI

- Add a minimal CI workflow that runs cargo fmt checks, cargo check, and cargo test.
- Keep CI lightweight and fast enough that it is not a burden for a small project.
- Add test coverage for the extracted voting core before any repo split happens.

## Phase 3: Export And Data Handling

Goal: make exports useful in real servers and align behavior with user expectations.

### 1. CSV Export Attachments ✓

Done. Export sends a CSV file attachment with poll metadata. Permission-limited to creator or admin.

### 2. Export Format Quality

- Make CSV columns stable and documented.
- Decide whether raw user IDs should remain in the default export or whether there should be an anonymized option.
- Ensure large exports still work cleanly without truncation.

### 3. Retention And Privacy Notes

- Document what vote data is stored and for how long in the hosted instance.
- Clarify how vote sharing and exports interact with privacy expectations.
- Keep these notes lightweight, but explicit.

## Phase 4: Product Polish With Low Operational Cost

Goal: improve usefulness without turning the project into a high-overhead platform.

- Improve result presentation in Discord embeds.
- Add better summaries for runoff rounds and ranked-choice elimination rounds.
- Improve mobile readability for long option lists.
- Tighten admin workflows for ending polls, reviewing results, and exporting data.
- Consider small accessibility improvements where Discord component limits permit them.

These are worthwhile, but they should follow reliability and testing work.

## Phase 5: Voting Engine Extraction

Goal: turn the voting algorithms into a reusable, well-tested Rust library if the boundary proves clean.

## Recommendation

Yes, this likely does make sense, but not as the next step.

The current voting methodology code already has real value outside the bot. It is mostly pure tally logic and is only lightly coupled to the rest of the application through the current Poll and Vote structs. That means the idea is sound. The mistake would be splitting it into a new repo before the interface is stable and well tested.

## Proposed Extraction Path

### Step 1: Create A Clean Internal Boundary

- Define a bot-agnostic core data model for tallying inputs and outputs.
- Keep Discord interaction code, database code, and presentation formatting in this repo.
- Move only the pure tally logic and method-specific result structures behind a clean internal module or workspace crate.

### Step 2: Test The Core Thoroughly

- Add comprehensive tests before and during extraction.
- Treat the test suite as the contract for the future reusable library.
- Verify performance on realistic ballot sizes before publishing anything externally.

### Step 3: Publish Only If The API Is Worth Reusing

- If the core ends up small, clear, and method-agnostic, publish it as a separate crate or repo.
- If the API remains awkward or mostly tailored to this bot's result formatting, keep it internal.

## What Should Stay In The Bot Repo

- Discord command definitions
- Discord component handling
- Poll persistence and SQLx queries
- Export formatting tied to Discord UX
- Hosted-service documentation and deployment notes

## What Belongs In A Reusable Voting Crate

- Input ballot types
- Normalized election result types
- STAR tallying
- Approval tallying
- Plurality tallying
- Ranked-choice tallying
- Tie and runoff handling rules
- Deterministic test fixtures and benchmarks

## Rough Order Of Execution

1. ~~Add this roadmap and align docs around the single-instance model.~~ ✓
2. ~~Add a minimal self-hosting guide and example deployment.~~ ✓
3. Move schema setup to migrations.
4. ~~Change CSV export to real file attachments.~~ ✓
5. Add unit and regression tests for all voting methods. (partial — 15 tests exist)
6. Add lightweight CI.
7. Refactor the voting logic behind a cleaner internal boundary.
8. Decide whether the extracted core is strong enough to become a separate crate.

## Non-Goals For Now

- Shared multi-tenant infrastructure across many bot instances
- A web application with account login
- Complex orchestration beyond one bot process and one database
- Building the API and website into a required part of self-hosting

## Success Criteria

This roadmap is succeeding if Trusty Vote becomes:

- easier to self-host in a minimal way
- more trustworthy because vote logic is tested
- safer and more useful for exports
- easier to maintain as a small public utility
- optionally reusable as a voting-method library without dragging Discord-specific concerns into that library
