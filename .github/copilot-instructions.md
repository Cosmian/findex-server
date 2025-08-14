# Cosmian Findex Server

Cosmian Findex Server is a high-performance, Rust-based server implementing the Findex cryptographic protocol for secure search on encrypted indexes. The server uses client-side encryption to ensure data privacy even on untrusted infrastructure.

Always reference these instructions first and fallback to search or bash commands only when you encounter unexpected information that does not match the info here.

## Working Effectively

### Initial Repository Setup

- Initialize git submodules (REQUIRED for test data): `git submodule update --init --recursive`
- The repository requires the nightly Rust toolchain: `nightly-2025-03-31` (configured in `rust-toolchain.toml`)
- Verify Rust toolchain: `rustc --version && cargo --version`

### Building the Project

- Debug build: `cargo build` -- takes 6-7 minutes on first build. NEVER CANCEL. Set timeout to 15+ minutes.
- Release build: `cargo build --release` -- takes 3-4 minutes. NEVER CANCEL. Set timeout to 10+ minutes.
- Build specific crates: `cargo build -p cosmian_findex_server` or `cargo build -p cosmian_findex_cli`
- The main server binary: `target/debug/cosmian_findex_server` or `target/release/cosmian_findex_server`

### Testing the Project

- **CRITICAL**: Tests require Redis database running. Use: `docker compose up -d` before running tests.
- Library tests only: `cargo test --lib --package cosmian_findex_server` -- takes 30-45 seconds. Set timeout to 2+ minutes.
- Integration tests: `export FINDEX_TEST_DB="redis-findex" && cargo test --package cosmian_findex_cli --lib` -- takes 3+ minutes but may fail without KMS server. Set timeout to 10+ minutes.
- **DO NOT** run full integration tests without proper infrastructure setup (requires both Redis and KMS servers).

### Code Quality and Formatting

- Install clippy: `rustup component add clippy`
- Format check: `cargo fmt --check` -- takes <1 second
- Format code: `cargo fmt`
- Clippy check: `cargo clippy --workspace --all-targets -- -D warnings` -- takes 1-2 minutes. Set timeout to 5+ minutes.
- **NEVER CANCEL** linting operations. They may appear to hang but will complete.

### Running the Server

#### With Docker (Recommended for Quick Testing)

- Start Redis only: `docker compose up -d`
- Run local server: `./target/debug/cosmian_findex_server`
- Test server: `curl http://localhost:6668/version`
- Default configuration: HTTP on port 6668, Redis on localhost:6379

#### From Source (Development)

- **PREREQUISITE**: Start Redis: `docker compose up -d`
- Run server: `cargo run --bin cosmian_findex_server`
- Or use built binary: `./target/debug/cosmian_findex_server`
- **IMPORTANT**: Server will fail to start without Redis running

#### Docker Quick Start (May Have Issues)

- Full stack: `docker compose -f docker-compose-quick-start.yml up -d`
- **NOTE**: The Docker quick start may have connectivity issues in some environments

### Server Configuration

- Default config: HTTP server on 0.0.0.0:6668, Redis on localhost:6379
- Configuration via environment variables (prefix: `FINDEX_SERVER_`)
- Configuration via TOML file (see `documentation/docs/configuration.md`)
- Help: `./target/debug/cosmian_findex_server --help`

## Validation

### Always validate changes by

1. Building successfully: `cargo build` (NEVER CANCEL - 6+ minutes)
2. Running clippy: `cargo clippy --workspace --all-targets -- -D warnings` (NEVER CANCEL - 2+ minutes)
3. Formatting: `cargo fmt --check`
4. Starting Redis: `docker compose up -d`
5. Testing basic server functionality: `./target/debug/cosmian_findex_server` then `curl http://localhost:6668/version`
6. Running lib tests: `cargo test --lib --package cosmian_findex_server`

### Expected Success Outputs

- **Build success**: "Finished `dev` profile [unoptimized + debuginfo] target(s) in Xm Ys"
- **Test success**: "test result: ok. 19 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in X.Ys"
- **Server startup**: Log messages including "starting 4 workers" and "listening on: 0.0.0.0:6668"
- **Clippy success**: "Finished `dev` profile [unoptimized + debuginfo] target(s) in Xm Ys" with no warnings
- **Format check**: No output (silent success)
- **Basic functionality**: Start server and verify `/version` endpoint returns `"0.4.1 (OpenSSL 3.0.13 30 Jan 2024)"`
- **Database connectivity**: Server should log Redis connection info on startup with message containing `redis://localhost:6379`
- **Configuration testing**: Verify server respects environment variables and command-line arguments
- **Server startup sequence**: Look for log messages including "starting 4 workers" and "Starting the HTTP Findex server..."

### CRITICAL Timeout Guidelines

- **NEVER CANCEL builds or long-running commands**
- Debug build: 15+ minute timeout
- Release build: 10+ minute timeout
- Clippy: 5+ minute timeout
- Library tests: 2+ minute timeout
- Integration tests: 10+ minute timeout (but expect failures without full infrastructure)

## Key Projects and Structure

### Workspace Crates

- `crate/server` - Main Findex server binary (`cosmian_findex_server`)
- `crate/cli` - Command-line interface library (`cosmian_findex_cli`)
- `crate/findex_client` - Client library for interacting with server
- `crate/structs` - Shared data structures
- `crate/test_findex_server` - Test utilities

### Important Files

- `Cargo.toml` - Workspace configuration
- `rust-toolchain.toml` - Specifies required nightly toolchain
- `docker-compose.yml` - Redis for development
- `docker-compose-quick-start.yml` - Full stack (Redis + Findex server)
- `.rustfmt.toml` - Rust formatting configuration
- `documentation/` - MkDocs-based documentation

### Dependencies

- **Redis**: Required for all server operations and tests
- **Docker**: Used for Redis and integration testing
- **Git submodules**: `test_data` and `.github/reusable_scripts` contain required test certificates and scripts

## Common Issues

### Build Issues

- **Network timeouts during initial build**: Retry `cargo build` - dependency downloads can be slow
- **Missing git submodules**: Run `git submodule update --init --recursive`
- **Wrong Rust toolchain**: Verify `rust-toolchain.toml` is respected

### Runtime Issues

- **Server won't start**: Ensure Redis is running with `docker compose up -d`
- **Test failures**: Most integration tests require both Redis and KMS servers
- **Port conflicts**: Default ports are 6668 (Findex) and 6379 (Redis)

### Development Workflow

- Always start with: `git submodule update --init --recursive`
- Always ensure Redis is running before testing: `docker compose up -d`
- Always run the full validation sequence after making changes
- Use `cargo check` for faster syntax validation during development
- Use `RUST_LOG=debug` for verbose server logging during development

## Infrastructure Requirements

- **Operating System**: Linux (tested), macOS (supported), Windows (may have issues)
- **Memory**: 4GB+ recommended for builds
- **Disk**: 2GB+ for full build artifacts
- **Network**: Required for initial dependency downloads and Docker image pulls
- **Docker**: Required for Redis database and integration testing
