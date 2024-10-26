use serde::Serialize;

use crate::error::result::CliResult;

pub const KMS_CLI_FORMAT: &str = "KMS_CLI_FORMAT";
pub const CLI_DEFAULT_FORMAT: &str = "text";
pub const CLI_JSON_FORMAT: &str = "json";

#[derive(Serialize, Debug, Default)]
pub struct Stdout {
    stdout: String,
}

impl Stdout {
    #[must_use]
    pub fn new(stdout: &str) -> Self {
        Self {
            stdout: stdout.to_owned(),
        }
    }

    /// Writes the output to the console.
    ///
    /// # Errors
    ///
    /// Returns an error if there is an issue with writing to the console.
    #[allow(clippy::print_stdout)]
    pub fn write(&self) -> CliResult<()> {
        // Check if the output format should be JSON
        let json_format_from_env = std::env::var(KMS_CLI_FORMAT)
            .unwrap_or_else(|_| CLI_DEFAULT_FORMAT.to_owned())
            .to_lowercase()
            == CLI_JSON_FORMAT;

        if json_format_from_env {
            // Serialize the output as JSON and print it
            let console_stdout = serde_json::to_string_pretty(&self)?;
            println!("{console_stdout}");
        } else {
            // Print the output in text format
            if !self.stdout.is_empty() {
                println!("{}", self.stdout);
            }
        }
        Ok(())
    }
}
