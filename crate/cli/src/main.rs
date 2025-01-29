use std::process;

use cosmian_findex_cli::findex_cli_main;

#[tokio::main]
async fn main() {
    if let Some(err) = findex_cli_main().await.err() {
        eprintln!("ERROR: {err}");
        process::exit(1);
    }
}
