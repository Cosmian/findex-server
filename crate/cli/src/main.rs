use std::{path::PathBuf, process};

use clap::{CommandFactory, Parser, Subcommand};
use cosmian_findex_cli::{
    actions::{
        findex::{add_or_delete::AddOrDeleteAction, search::SearchAction},
        login::LoginAction,
        logout::LogoutAction,
        markdown::MarkdownAction,
        permissions::PermissionsAction,
        version::ServerVersionAction,
    },
    error::result::CliResult,
};
use cosmian_logger::log_utils::log_init;
use cosmian_rest_client::ClientConf;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: CliCommands,

    /// Configuration file location
    ///
    /// This is an alternative to the env variable `FINDEX_CLI_CONF`.
    /// Takes precedence over `FINDEX_CLI_CONF` env variable.
    #[arg(short, long)]
    conf: Option<PathBuf>,

    /// The URL of the Findex
    #[arg(long, action)]
    pub(crate) url: Option<String>,

    /// Allow to connect using a self-signed cert or untrusted cert chain
    ///
    /// `accept_invalid_certs` is useful if the CLI needs to connect to an HTTPS
    /// Findex server running an invalid or insecure SSL certificate
    #[arg(long)]
    pub(crate) accept_invalid_certs: Option<bool>,
}

#[derive(Subcommand)]
enum CliCommands {
    /// Index new keywords.
    Add(AddOrDeleteAction),
    /// Delete indexed keywords
    Delete(AddOrDeleteAction),
    Search(SearchAction),
    ServerVersion(ServerVersionAction),
    Login(LoginAction),
    Logout(LogoutAction),
    #[command(subcommand)]
    Permissions(PermissionsAction),

    /// Action to auto-generate doc in Markdown format
    /// Run `cargo run --bin findex -- markdown
    /// documentation/docs/cli/main_commands.md`
    #[clap(hide = true)]
    Markdown(MarkdownAction),
}

#[tokio::main]
#[allow(clippy::needless_return)]
async fn main() {
    if let Some(err) = main_().await.err() {
        eprintln!("ERROR: {err}");
        process::exit(1);
    }
}

async fn main_() -> CliResult<()> {
    log_init(None);
    let opts = Cli::parse();

    if let CliCommands::Markdown(action) = opts.command {
        let command = <Cli as CommandFactory>::command();
        action.process(&command)?;
        return Ok(());
    }

    let conf_path = ClientConf::location(opts.conf)?;

    match opts.command {
        CliCommands::Login(action) => action.process(&conf_path).await?,
        CliCommands::Logout(action) => action.process(&conf_path)?,

        command => {
            let conf = ClientConf::load(&conf_path)?;
            let rest_client =
                conf.initialize_findex_client(opts.url.as_deref(), opts.accept_invalid_certs)?;

            match command {
                CliCommands::Add(action) => action.add(rest_client).await?,
                CliCommands::Delete(action) => action.delete(rest_client).await?,
                CliCommands::Search(action) => action.process(rest_client).await?,
                CliCommands::ServerVersion(action) => action.process(rest_client).await?,
                CliCommands::Permissions(action) => action.process(rest_client).await?,
                _ => {
                    tracing::error!("unexpected command");
                }
            }
        }
    }

    Ok(())
}
