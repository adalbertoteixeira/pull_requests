pub mod ticket;
extern crate clap;
pub mod branch_utils;
pub mod commit;
pub mod path_utils;
pub mod storage;
pub mod utils;
pub mod ux_utils;

use clap::{App, Arg, SubCommand};
use log::{debug, info};
pub mod prompts;
extern crate log;

#[tokio::main]
async fn main() {
    env_logger::init();

    let matches = App::new("Commit Message Builder")
        .version("1.0")
        .arg(
            Arg::with_name("directory")
                .short("d")
                .long("directory")
                .value_name("directory")
                .takes_value(true)
                .help("Optional directory to start from")
                .default_value("."),
        )
        .arg(
            Arg::with_name("config_directory")
                .short("c")
                .help("Config directory to store the tools global information.")
                .env("COMMIT_TOOL_CONFIG_DIRECTORY")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("cowboy_mode")
                .long("cowboy-mode")
                .help("Auto accept most prompts, except potentially destructive ones.")
                .takes_value(false),
        )
        .subcommands(vec![
            SubCommand::with_name("ticket")
                .arg(
                    Arg::with_name("clickup_api_key")
                        .help("Clickup API key to interact with issues")
                        .env("CLICKUP_API_KEY")
                        .required(true),
                )
                .arg(
                    Arg::with_name("clickup_workspace_id")
                        .help("Clickup workspace where actions will be taken")
                        .env("CLICKUP_WORKSPACE_ID")
                        .required(true),
                )
                .subcommands(vec![
                    SubCommand::with_name("spaces"),
                    SubCommand::with_name("issue").arg(
                        Arg::with_name("issue")
                            .help("Get description from Clickup ticket so we can pipe it into another tool.")
                            .required(true)
                            .takes_value(true),
                    ),
                ]),
            SubCommand::with_name("commit")
                .arg(
                    Arg::with_name("claude")
                        .long("claude")
                        .takes_value(false)
                        .help("Pass this flag if you want to use Claude Code to help on the commit construction"), // .possible_values(&possible_types_slice)
                                             // .default_value(&possible_types_slice[0]),
                )
                .arg(
                    Arg::with_name("type")
                        .short("t")
                        .long("type")
                        .takes_value(true)
                        .help("Type of PR"), // .possible_values(&possible_types_slice)
                                             // .default_value(&possible_types_slice[0]),
                )
                .arg(
                    Arg::with_name("scope")
                        .short("s")
                        .long("scope")
                        .value_name("type")
                        .help("Scope of changes")
                        // .possible_values(&possible_scopes_slice)
                        .takes_value(true),
                    // .default_value(&possible_scopes_slice[0]),
                )
                .arg(
                    Arg::with_name("message")
                        .short("m")
                        .long("message")
                        .value_name("message")
                        .help("Commit message")
                        .takes_value(true),
                    // .required(true),
                )
                .arg(
                    Arg::with_name("prefix")
                        .short("p")
                        .long("prefix")
                        .value_name("prefix")
                        .help("Issue prefix")
                        .takes_value(true),
                )
                .arg(
                    Arg::with_name("show_pr_template")
                        .short("r")
                        .long("show-pr-template")
                        .value_name("show-pr-template")
                        .help("Show PR template")
                        .takes_value(false),
                )
                .arg(
                    Arg::with_name("push_branch")
                        .long("push-branch")
                        .value_name("push-branch")
                        .help("Push the branch to the origin")
                        .takes_value(false),
                ),
        ])
        .get_matches();

    let config_directory_matches = matches.value_of("config_directory").unwrap_or("");
    let config_editor_matches = matches.value_of("editor").unwrap_or("");
    info!("Configured editor is {:?}", config_editor_matches);
    let _ = storage::setup_commit_tool(config_directory_matches, config_editor_matches);
    debug!("Arguments: {:?}", matches);
    let matches_clone = matches.clone();
    let directory = matches_clone.value_of("directory").unwrap_or(".");

    info!("Base directory is {:?}", directory);
    path_utils::top_level(&directory.to_owned());

    let cowboy_mode = matches.is_present("cowboy_mode");
    let git_branch = path_utils::git_branch(&directory);
    if let Some(_) = matches.subcommand_matches("commit") {
        commit::commit(
            matches.subcommand_matches("commit").unwrap().clone(),
            &git_branch,
            &directory,
            &cowboy_mode,
        );
    }

    if let Some(_) = matches.subcommand_matches("ticket") {
        let _ = ticket::ticket(
            matches.subcommand_matches("ticket").unwrap().clone(),
            &directory,
        )
        .await;
    }
}
