use crate::branch_utils;
use crate::prompts;
use crate::storage;
use crate::utils::claude;
use crate::ux_utils;
use clap::ArgMatches;
use indicatif::ProgressBar;
use inquire::Confirm;
use log::debug;
use log::{error, info, warn};
use std::process::Command;
use std::time::Duration;
use std::{
    io::{self, Write},
    process::{self},
    str,
};

pub fn commit(
    matches: ArgMatches,
    git_branch: &str,
    directory: &str,
    github_api_token: Option<&str>,
    _mcp_config: Option<&str>,
    has_gh: bool,
) {
    let use_claude = matches.is_present("claude");
    debug!("USE CLAUDE {}, {:?}", use_claude, matches);
    let stdout = io::stdout(); // get the global stdout entity
    let mut handle = io::BufWriter::new(&stdout); // optional: wrap that handle in a buffer
    // Show the PR template only
    if matches.is_present("show_pr_template") {
        storage::load_commit_template(&git_branch, &directory);
        process::exit(0);
    }

    let ci_mode = matches.is_present("ci_mode");
    let no_verify = matches.is_present("no_verify");
    let stored_pr_template = match storage::get_branch_config(git_branch, directory) {
        Ok(x) => x,
        Err(_) => None,
    };
    let mut commit_message = None;
    let mut pr_template = None;
    match stored_pr_template {
        Some(x) => {
            if x.commit_message.is_some() {
                commit_message = x.commit_message
            }
            if x.pr_template.is_some() {
                pr_template = x.pr_template
            }
        }
        None => {}
    };
    let is_new_branch = storage::setup_branch_env(&git_branch, &directory).unwrap();
    let mut issue_id = branch_utils::issue_id(&git_branch);
    let team_prefix = "INF";

    let mut additional_commit_message = vec![];
    match (use_claude, commit_message.is_none()) {
        (true, true) => {
            writeln!(handle, "Will build a commit message using Claude").unwrap_or_default();

            let _ = handle.flush();

            let cmd_arg = format!(
                r#"cd {} && claude --model sonnet --output-format json  -p "We have done several changes to this repository that we are going to commit. Please analyze the staged files and return a json object with the following structure:
                    commit_message: string,
                    commit_type: string,
                    commit_labels: string[],

The expected values are the following.
- commit_message: should be a string under 50 characters that summarizes the main changes to the pr. Do not include the commit type in the commit_message entry
- commit_type as a string choose the best from the following options and return the key:
        "feat: A new feature",
        "fix: Bug feature related or code linting, typecheck, etc fixes",
        "test: Adding missing tests or correcting existing tests",
        "refactor: A code change that improves performance or code quality",
        "docs: Documentation only changes",
        "build: Changes that affect the build system or external dependencies example scopes: gulp, broccoli, npm",
        "ci: Changes to our CI configuration files and scripts example scopes: Travis, Circle, BrowserStack, SauceLabs",
        "revert: Reverts a previous commit",

- commit_labels: an array of strings; choose all that apply from the web, api or ci; if none match return an empty array
                web: files related to frontend code
                api: files related to backend code
                ci: files related to deployments
Please only analyse code for staged files. Do not include un staged files in the analysis.
                ""#,
                &directory
            );
            let bar = ProgressBar::new_spinner();
            bar.enable_steady_tick(Duration::from_millis(100));
            let output = Command::new("sh")
                .arg("-c")
                .arg(cmd_arg)
                .output()
                .expect("Failed to run  process");

            bar.finish();

            let _cmd_arg_status_code = output.status.code();
            let result_stdout_string = str::from_utf8(&output.stdout).unwrap();
            let _result_stderr = str::from_utf8(&output.stderr).unwrap();

            let result_json = claude::parse_claude_response(result_stdout_string)
                .ok()
                .expect("Should return JSON");
            commit_message = Some(format!(
                "{}: {} [{}] #{}",
                result_json
                    .get("commit_type")
                    .unwrap()
                    .as_str()
                    .unwrap()
                    .to_lowercase(),
                result_json
                    .get("commit_message")
                    .unwrap()
                    .as_str()
                    .unwrap()
                    .to_lowercase()
                    .chars()
                    .take(50)
                    .into_iter()
                    .collect::<String>(),
                team_prefix,
                issue_id
            ));
        }
        _ => {
            let message_name;
            if matches.is_present("message") {
                message_name = matches
                    .value_of("message")
                    .unwrap()
                    .to_string()
                    .to_lowercase();
            } else {
                message_name = branch_utils::branch_name(&git_branch)
                    .to_lowercase()
                    .chars()
                    .take(55)
                    .collect::<String>();
            };

            let mut can_build_default_message = true;
            let mut output_text: String = "\n\x1b[1;1mCommit utility\x1b[0m\n".to_owned();
            output_text.push_str("- Working in directory ");
            output_text.push_str(&format!("\x1b[1;1m{}\x1b[0m\n", &directory));
            output_text.push_str("- Git branch is ");
            output_text.push_str(&format!("\x1b[1;1m{}\x1b[0m\n", &git_branch));
            output_text.push_str("- Team prefix is ");
            output_text.push_str(&format!("\x1b[1;1m{}\x1b[0m\n", &team_prefix));
            if !&issue_id.is_empty() {
                output_text.push_str("- Issue id is ");
                output_text.push_str(&format!("\x1b[1;1m{}\x1b[0m\n", &issue_id));
            } else {
                can_build_default_message = false;
                output_text.push_str(&format!("\x1b[1;31m- No issue id found\x1b[0m\n"));
            }
            if !&message_name.is_empty() {
                output_text.push_str("- Message name is ");
                output_text.push_str(&format!("\x1b[1;1m{}\x1b[0m\n", &message_name));
            } else {
                can_build_default_message = false;
                output_text.push_str(&format!("\x1b[1;31m- No message name found\x1b[0m\n"));
            }
            if is_new_branch == false {
                let _ = storage::load_branch_config(
                    &git_branch,
                    directory,
                    no_verify,
                    ci_mode,
                    github_api_token,
                    has_gh,
                );
            }
            info!("Is new branch: {}", &is_new_branch);
            let (proposed_type, used_types) =
                branch_utils::find_changed_file_types(directory, &is_new_branch);
            info!("Proposed types: {:?}", &proposed_type);
            if proposed_type.is_none() {
                can_build_default_message = false;
            }
            info!("Used types: {:?}", &used_types);
            writeln!(handle, "{}", output_text).unwrap_or_default();

            warn!("Can build message: {}", &can_build_default_message);
            let _ = handle.flush();
            let mut will_accept_suggested_message = false;
            if can_build_default_message {
                let mut proposed_output_string: String = "".to_owned();
                proposed_output_string.push_str(&format!(
                    "{}: {} [{}] #{}",
                    &proposed_type.clone().unwrap(),
                    &message_name,
                    &team_prefix,
                    &issue_id
                ));
                info!("Will propose default message: {}", &proposed_output_string);
                let mut proposed_ouput_message = "".to_owned();
                proposed_ouput_message.push_str(
                    "We have enough information to propose the following commit message:\n\n",
                );
                proposed_ouput_message
                    .push_str(&format!("\x1b[1;32m{}\x1b[1;0m", &proposed_output_string));
                proposed_ouput_message.push_str("\n");
                writeln!(handle, "{}", proposed_ouput_message).unwrap_or_default();
                let _ = handle.flush();

                if ci_mode == true {
                    commit_message = Some(proposed_output_string);
                    will_accept_suggested_message = true;
                } else {
                    let confimation_prompt =
                        Confirm::new("Do you want to accept the proposed message?")
                            .with_default(true)
                            .prompt();

                    will_accept_suggested_message = match confimation_prompt {
                        Ok(selection) => {
                            commit_message = Some(proposed_output_string);
                            selection
                        }

                        Err(_) => {
                            print!("Did not understand your input.");
                            process::exit(1);
                        }
                    }
                }
            }

            if !will_accept_suggested_message {
                let mut output_string: String = "".to_owned();
                let selected_issue_id = prompts::issue_id_prompt(&issue_id);
                issue_id = selected_issue_id;
                let selected_team_prefix = prompts::team_prefix_prompt(&team_prefix);
                let selected_type = prompts::select_types_prompt(proposed_type);
                info!("Selected type: {}", selected_type);

                let _scope_options: Vec<&str> = vec![
                    "web: Work related to front end",
                    "api: work related to the back end",
                    "devops: work related to infrastructure, tools, etc.",
                ];

                let message = prompts::select_message_prompt(&git_branch);
                output_string.push_str(&format!(
                    // "\x1b[1;32m{}: {} [{}] #{}\x1b[1;0m",
                    "{}: {} [{}] #{}",
                    selected_type,
                    message.to_lowercase(),
                    selected_team_prefix,
                    issue_id
                ));
                let additional_message = prompts::select_additional_message_prompt();
                if additional_message.is_some() {
                    info!("Additional message: {:?}", &additional_message);
                    for line in additional_message.unwrap().split("\n") {
                        info!("Additional message line : {}", line);
                        additional_commit_message.push(line.to_owned());
                    }
                }

                // Add scope
                if matches.is_present("scope") {
                    let scope = matches.value_of("scope").unwrap();
                    output_string.push_str("(");
                    output_string.push_str(scope);
                    output_string.push_str(")");
                }

                // Add message
                if matches.is_present("message") {
                    match matches.value_of("message") {
                        Some(message) => {
                            output_string.push_str(message);
                        }
                        None => {
                            error!("No scope defined");
                            return;
                        }
                    };
                }
                commit_message = Some(output_string.clone());
                if additional_commit_message.len() > 0 {
                    output_string.push_str("\x1b[1;32m\n");
                    for line in &additional_commit_message {
                        output_string.push_str(&format!("\n{}", line));
                    }
                    output_string.push_str("\x1b[1;0m");
                }
                writeln!(handle, "{}", output_string).unwrap_or_default();
                let _ = handle.flush();
            }
        }
    }

    let mut confirm_message = "Do you want to build a PR template?".to_owned();
    if use_claude {
        confirm_message.push_str(" We will use Claude Code to build it");
    }

    let mut build_pr_template = false;
    if ci_mode == true && pr_template.is_none() {
        build_pr_template = true;
    } else {
        let pr_template_prompt = Confirm::new(&confirm_message)
            .with_default(is_new_branch)
            .prompt();
        match pr_template_prompt {
            Ok(selection) => {
                if selection {
                    build_pr_template = true
                }
            }
            Err(_) => {
                print!("Did not understand your input.");
                process::exit(1);
            }
        }
    }

    if build_pr_template {
        pr_template = Some(prompts::pr_template_prompt(
            &issue_id, use_claude, &directory,
        ));
    }
    info!("Will ask for commit");
    let will_commit_pr = prompts::commit_pr_prompt();

    info!("Commit was defined: {}", will_commit_pr);
    if will_commit_pr == true {
        ux_utils::commit_and_push(
            directory,
            commit_message.unwrap(),
            additional_commit_message.clone(),
            &git_branch,
            pr_template,
            no_verify,
            ci_mode,
            github_api_token,
            has_gh,
        );
    }
}
