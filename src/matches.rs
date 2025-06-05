use clap::{App, Arg, ArgMatches, SubCommand};

pub fn build_matches() -> ArgMatches<'static> {
    App::new("Commit Message Builder")
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
            Arg::with_name("github_api_token")
                .long("github-api-token")
                .env("GITHUB_API_TOKEN")
                .takes_value(true)
                .help("A Github fine grained personal access token. See https://docs.github.com/en/authentication/keeping-your-account-and-data-secure/managing-your-personal-access-tokens#creating-a-fine-grained-personal-access-token \n You might require organization approval before being able to use it."),
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
                    SubCommand::with_name("issues"),
                    SubCommand::with_name("issue").arg(
                        Arg::with_name("issue")
                            .help("Get description from Clickup ticket so we can pipe it into another tool.")
                            .required(true)
                            .takes_value(true),
                    ),
                ]),
            SubCommand::with_name("push")
                .arg(
                    Arg::with_name("push_branch")
                        .long("push-branch")
                        .value_name("push-branch")
                        .help("Push the branch to the origin")
                        .takes_value(false),
                )
                .arg(
                    Arg::with_name("cowboy_mode")
                        .long("cowboy-mode")
                        .short("c")
                        .help("Auto accept most prompts, except potentially destructive ones.")
                        .takes_value(false),
                )
                .arg(
                    Arg::with_name("no_verify")
                        .short("n")
                        .long("no-verify")
                        .help("Skip git pre-commit and pre-push hooks")
                        .takes_value(false),
                ),
            SubCommand::with_name("commit")
                .arg(
                    Arg::with_name("claude")
                        .long("claude")
                        .takes_value(false)
                        .help("Pass this flag if you want to use Claude Code to help on the commit construction"),
                )
                .arg(
                    Arg::with_name("type")
                        .short("t")
                        .long("type")
                        .takes_value(true)
                        .help("Type of PR"),
                )
                .arg(
                    Arg::with_name("scope")
                        .short("s")
                        .long("scope")
                        .value_name("type")
                        .help("Scope of changes")
                        .takes_value(true),
                )
                .arg(
                    Arg::with_name("message")
                        .short("m")
                        .long("message")
                        .value_name("message")
                        .help("Commit message")
                        .takes_value(true),
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
                    Arg::with_name("cowboy_mode")
                        .long("cowboy-mode")
                        .short("c")
                        .help("Auto accept most prompts, except potentially destructive ones.")
                        .takes_value(false),
                )
                .arg(
                    Arg::with_name("no_verify")
                        .short("n")
                        .long("no-verify")
                        .help("Skip git pre-commit and pre-push hooks")
                        .takes_value(false),
                ),
        ])
        .get_matches()
}
