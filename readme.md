# Trusty Vote ⭐👍✅

*Lightweight rust discord bot for running alternative polling/voting methods such as STAR. Previously called Rusty Bote.*

### <a href="https://discord.com/oauth2/authorize?client_id=1360286244856402222" target="_blank" noreferer>+ Add Trusty Vote to a Discord Server ⭐</a>

see **project.md** for project info. project.md is also designed to be passed to an LLM along with any prompts for assistance with the project.

see **ROADMAP.md** for the current product and engineering roadmap.

### running locally

**1\.** Set up your own discord app, get the token and add it to .env, invite the bot to your server

**2a.** `cargo run --features embedded-postgres`

\- or \-

**2b.** set up a local postgres server and provide it via DATABASE_URL .env var

The bot should add its slash commands to your server and you can interface with it as normal. Embedded-postgres provides an sqlite-like experience where you can run the bot in one file, but it is currently not persisted between runs. I think that's possible and may be added in the future.

### misc links and info

discord app config: https://discord.com/developers/applications/1360286244856402222/installation

bot pfp is from: https://pixabay.com/photos/mediterranean-sea-old-ship-rusty-112004/