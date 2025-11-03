# Anime-dl Codebase Guide

## Project Overview
Anime-dl is a Rust CLI application (`adl`) for downloading and streaming anime from Italian websites. The binary name is `adl` (configured in `Cargo.toml`). Two main commands: `download` for saving files, `stream` for watching with mpv/vlc.

## Architecture

### Core Components
- **`anime.rs`**: `Anime` struct - central data model with URL generation logic via `gen_url!` macro
- **`scraper.rs`**: Web scraping coordinator with proxy/cookie support, runs parallel searches
- **`archive.rs`**: Site-specific scrapers as traits (e.g., `AnimeWorld`). Each implements `Archive` trait with `REFERRER` constant and async `run()` method
- **`parser.rs`**: URL parsing utilities - extracts episode numbers, filenames, names from URLs using pattern matching on `_digit_` format
- **`cli/`**: Command handlers (`download.rs`, `stream.rs`) with `clap` argument parsing
- **`anilist.rs`**: Optional GraphQL integration with AniList API for progress tracking (feature-gated)
- **`tui.rs`**: Interactive UI using `indicatif` for progress bars and `rustyline` for selections

### Data Flow
1. User input → CLI parser (`clap`)
2. Input converted to `Search` struct with optional anime ID
3. Scraper fetches archive pages, builds `Anime` vectors
4. URL generation: `Anime::select_from_range()` uses `gen_url!` macro to replace `_{}` placeholders with zero-filled episode numbers
5. Download: parallel async streams (`tokio`), range requests with `REFERER` headers
6. Anilist sync: tracks episode progress via GraphQL mutations

## Development Workflow

### Build & Test
Use `just` commands (defined in `justfile`):
- `just test` - Run tests with nextest
- `just test-all` - Include ignored tests (likely integration tests hitting real sites)
- `just test-all-musl` - Test with musl target for static binaries
- `just pre-commit` - Format + clippy (enforces `-D warnings`)
- `just release` - Build optimized musl binary for x86_64-linux

### Key Patterns

**URL Placeholder System**: Anime URLs use `_{}` as episode number placeholder:
```rust
// In macros.rs
gen_url!($str, $num, $alignment)  // "_02" for episode 2 with alignment 2
```

**Feature Gates**: AniList integration behind `anilist` feature (default enabled):
```rust
#[cfg(feature = "anilist")]
mod anilist;
```

**Async Concurrency**: Downloads use bounded concurrency via `futures::StreamExt`:
```rust
// In download.rs - dim_buff controls simultaneous downloads (default: 24)
stream::iter(urls).buffer_unordered(dim_buff)
```

**Error Handling**: Pervasive `anyhow::Result` with context chains. Terminal output uses `owo_colors` for styling.

## Project-Specific Conventions

### Commit Messages
- **MUST** follow [Conventional Commits](https://www.conventionalcommits.org/) format: `<type>[optional scope]: <description>`
- Allowed types: `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`, `build`, `ci`, `chore`, `revert`
- Subject must start with lowercase
- Examples:
  - `feat: add Italian subtitle support`
  - `fix: correct episode number parsing`
  - `docs: update installation instructions`
  - `chore: upgrade dependencies`

### Naming
- Snake_case with underscores separates anime name from episode: `anime_name_01_sub_ita.mp4`
- Episode numbers extracted via `parser::parse_number()` - looks for `_digit_` patterns
- Use `InfoNum` struct to preserve zero-padding alignment

### Module Structure
- `cli/mod.rs` defines shared types (`Site` enum, `Args` struct)
- Each command in separate file (`download.rs`, `stream.rs`)
- Macros in dedicated `macros.rs` file with `#[macro_use]` at module root

### Testing
- Integration tests marked `#[ignore]` (hit real websites) - run with `just test-ignored`
- Use `nextest` runner (faster parallel execution)

## External Dependencies

### Critical Services
- **AnimeWorld** (animeworld.ac): Primary Italian anime archive, requires `REFERRER` header
- **AniList API**: GraphQL endpoint at `graphql.anilist.co`, needs client ID via `ANIMEDL_ID` env var (default: 4047)
- **Media Players**: mpv or vlc required for streaming (checked via `which` crate)

### Key Crates
- `scraper`: HTML parsing with CSS selectors
- `reqwest`: HTTP client with rustls-tls (no native-tls)
- `tokio`: Async runtime
- `graphql_client`: Code generation from `graphql/*.graphql` schema files
- `clap`: CLI parsing with derive macros
- `indicatif`: Progress bars

## Common Tasks

### Adding New Archive Site
1. Implement `Archive` trait in `archive.rs`
2. Define `REFERRER` constant
3. Implement async `run()` - parse HTML, extract anime URLs, populate `Anime` structs
4. Add site to `Site` enum in `cli/mod.rs`

### Modifying URL Generation
Check `macros.rs` for `gen_url!` macro and `parser.rs` for episode number extraction logic. URLs must follow `base_url_NN_suffix` pattern.

### GraphQL Schema Updates
Regenerate: modify `graphql/*.graphql` files, rebuild triggers `graphql_client` codegen.

## Configuration
- Config stored via `config.rs` (AniList OAuth tokens)
- `clean` command deletes cache: `adl clean`
- Env vars: `ANIMEDL_ID` (AniList client ID)
