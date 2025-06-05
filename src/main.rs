pub mod ticket;
extern crate clap;
pub mod branch_utils;
pub mod commit;
pub mod matches;
pub mod path_utils;
pub mod storage;
pub mod utils;
pub mod ux_utils;

use log::{debug, info};
pub mod prompts;
extern crate log;

#[tokio::main]
async fn main() {
    env_logger::init();

    let matches = matches::build_matches();

    let config_directory_matches = matches.value_of("config_directory").unwrap_or("");
    let config_editor_matches = matches.value_of("editor").unwrap_or("");
    info!("Configured editor is {:?}", config_editor_matches);
    let _ = storage::setup_commit_tool(config_directory_matches, config_editor_matches);
    debug!("Arguments: {:?}", matches);
    let matches_clone = matches.clone();
    let directory = matches_clone.value_of("directory").unwrap_or(".");

    let github_api_token = matches.value_of("github_api_token").unwrap_or("");

    info!("Base directory is {:?}", directory);
    path_utils::top_level(&directory.to_owned());

    let git_branch = path_utils::git_branch(&directory);
    if let Some(_) = matches.subcommand_matches("push") {
        let commit_matches = matches.subcommand_matches("commit").unwrap().clone();
        let no_verify = commit_matches.is_present("no_verify");
        let cowboy_mode = commit_matches.is_present("cowboy_mode");
        branch_utils::push_pr(directory, no_verify, cowboy_mode);
    }

    if let Some(_) = matches.subcommand_matches("commit") {
        commit::commit(
            matches.subcommand_matches("commit").unwrap().clone(),
            &git_branch,
            &directory,
            github_api_token,
        );
    }

    if let Some(_) = matches.subcommand_matches("ticket") {
        let _ = ticket::ticket(
            matches.subcommand_matches("ticket").unwrap().clone(),
            &directory,
            github_api_token,
        )
        .await;
    }
}
