pub mod ticket;
extern crate clap;
pub mod branch_utils;
pub mod commit;
pub mod gh;
pub mod matches;
pub mod path_utils;
pub mod progress;
pub mod storage;
pub mod types;
pub mod utils;
pub mod ux_utils;
use branch_utils::validate_branch;
use gh::validate_gh;
use log::{debug, info};
use storage::get_branch_config;
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

    let github_api_token = matches.value_of("github_api_token");
    let no_verify = matches.is_present("no_verify");
    let ci_mode = matches.is_present("ci_mode");

    info!("Base directory is {:?}", directory);
    path_utils::top_level(&directory.to_owned());

    let git_branch = path_utils::git_branch(&directory);
    let mcp_config = matches.value_of("mcp_config");

    let has_gh = validate_gh();

    if let Some(_) = matches.subcommand_matches("push") {
        validate_branch(&git_branch);
        let branch_config = get_branch_config(&git_branch, &directory).expect("Should load config");
        let branch_config_parts = branch_config.unwrap();
        branch_utils::push_pr(
            &directory,
            no_verify,
            ci_mode,
            github_api_token,
            &git_branch,
            Some(branch_config_parts.commit_message.unwrap().as_str()),
            branch_config_parts.pr_template,
            has_gh,
        )
        .await;
    }

    if let Some(_) = matches.subcommand_matches("commit") {
        validate_branch(&git_branch);
        commit::commit(
            matches.subcommand_matches("commit").unwrap().clone(),
            &git_branch,
            &directory,
            github_api_token,
            mcp_config,
            has_gh,
        )
        .await;
    }

    if let Some(_) = matches.subcommand_matches("ticket") {
        let _ = ticket::ticket(
            matches.subcommand_matches("ticket").unwrap().clone(),
            &directory,
            github_api_token,
            mcp_config,
            has_gh,
        )
        .await;
    }

    if let Some(_) = matches.subcommand_matches("progress") {
        progress::progress(matches.subcommand_matches("progress").unwrap().clone()).await;
    }
}
