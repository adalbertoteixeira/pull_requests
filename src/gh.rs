use std::process::Command;

use inquire::Confirm;
use log::{debug, info};

pub fn validate_gh() -> bool {
    let result = Command::new("gh").arg("auth").arg("status").output();

    match result {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            debug!("OUT {:?}", stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            debug!("ERR: {:?}", stderr);
            stdout.contains("Logged in to github.com account")
        }
        Err(err) => {
            info!("ERR: {:?}", err);

            // Print error message with red color for "GitHub CLI is not installed."
            println!(
                "\x1b[31mGitHub CLI is not installed.\x1b[0m It's required to perform operations such as creating PRs. You can install it following the steps in https://cli.github.com/."
            );

            // Prompt user to continue
            let continue_prompt = Confirm::new("Continue without GitHub CLI?")
                .with_default(false)
                .prompt();

            match continue_prompt {
                Ok(true) => false,
                Ok(false) => std::process::exit(0),
                Err(_) => std::process::exit(1),
            }
        }
    }
}
