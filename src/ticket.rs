use std::{
    io::{self, Write},
    process::{self, Command, Stdio},
};

use clap::ArgMatches;
use inquire::{Select, Text};
use log::{debug, info};

use crate::{
    storage::{load_clickup_config, load_github_config},
    utils::{
        extract_clickup_spaces_data::{extract_clickup_spaces_data, make_clickup_request},
        extract_github_spaces_data::{
            GithubIssue, extract_github_spaces_data, get_github_user_issues,
        },
    },
};

#[derive(Debug, Clone)]
pub enum IssueManagementTool {
    GitHub,
    Clickup,
}

fn define_issue_management_tool(
    github_api_token: &str,
    clickup_api_key: Option<&str>,
) -> IssueManagementTool {
    let stdout = io::stdout();
    let mut handle = io::BufWriter::new(&stdout);

    let has_github = !github_api_token.is_empty();
    let has_clickup = clickup_api_key.is_some() && !clickup_api_key.unwrap().is_empty();

    match (has_github, has_clickup) {
        (false, false) => {
            writeln!(
                handle,
                "At least one API key for GitHub or ClickUp is required"
            )
            .unwrap_or_default();
            writeln!(
                handle,
                "Please set GITHUB_API_TOKEN or CLICKUP_API_KEY environment variable"
            )
            .unwrap_or_default();
            let _ = handle.flush();
            process::exit(1);
        }
        (true, false) => IssueManagementTool::GitHub,
        (false, true) => IssueManagementTool::Clickup,
        (true, true) => {
            let options = vec!["GitHub", "ClickUp"];
            let selection = Select::new("Select issue management tool:", options).prompt();

            match selection {
                Ok("GitHub") => IssueManagementTool::GitHub,
                Ok("ClickUp") => IssueManagementTool::Clickup,
                Ok(_) => unreachable!(),
                Err(_) => {
                    writeln!(handle, "Selection cancelled").unwrap_or_default();
                    let _ = handle.flush();
                    process::exit(1);
                }
            }
        }
    }
}

fn prompt_claude(issue_to_implement: &str, directory: &str) {
    let stdout = io::stdout();
    let mut handle = io::BufWriter::new(&stdout);

    // Spawn an interactive shell
    let prompt_text = format!(
        "Given the following issue description, implement all the changes required to the codebase:\n{:?}",
        issue_to_implement
    );
    let mut child = Command::new("claude")
        .arg(prompt_text)
        .current_dir(&directory)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("Failed to spawn interactive shell");

    // Wait for the shell to exit
    let status = child.wait().expect("Failed to wait for shell");
    info!("Shell exited with status: {:?}", status);
    writeln!(
        handle,
        "{}",
        "Work is done. We are working to implement the next automations in the future."
    )
    .unwrap_or_default();
    let _ = handle.flush();
}

pub async fn ticket(matches: ArgMatches<'static>, directory: &str, github_api_token: &str) {
    info!("Ticket command");
    let stdout = io::stdout(); // get the global stdout entity
    let mut handle = io::BufWriter::new(&stdout); // optional: wrap that handle in a buffer
    debug!("matches {:?}", matches);

    let clickup_api_key = matches.value_of("clickup_api_key");
    let tool = define_issue_management_tool(github_api_token, clickup_api_key);

    let client = reqwest::Client::new();
    match matches.subcommand() {
        ("issues", Some(arg)) => {
            match tool {
                IssueManagementTool::GitHub => {
                    let mut github_config = load_github_config(directory).ok();
                    if github_config.is_some() {
                        github_config =
                            extract_github_spaces_data(&directory, &client, github_api_token)
                                .await
                                .ok();
                    }

                    // Get GitHub issues
                    match get_github_user_issues(&client, github_api_token).await {
                        Ok(issues) => {
                            if issues.is_empty() {
                                writeln!(handle, "No issues found").unwrap_or_default();
                                let _ = handle.flush();
                                process::exit(0);
                            }

                            // Create options for the select prompt
                            let options: Vec<String> = issues
                                .iter()
                                .map(|issue| {
                                    format!("{} - {} - {}", issue.id, issue.repository, issue.title)
                                })
                                .collect();

                            // Show select prompt
                            let selection = Select::new("Select an issue:", options).prompt();

                            match selection {
                                Ok(selected) => {
                                    let selected_index = issues
                                        .iter()
                                        .position(|issue| {
                                            format!(
                                                "{} - {} - {}",
                                                issue.id, issue.repository, issue.title
                                            ) == selected
                                        })
                                        .unwrap();

                                    let selected_issue = &issues[selected_index];

                                    info!("selected {:?}", selected_issue.body);
                                    let _ = prompt_claude(&selected_issue.body, directory);
                                }
                                Err(_) => {
                                    writeln!(handle, "Selection cancelled").unwrap_or_default();
                                    let _ = handle.flush();
                                    process::exit(1);
                                }
                            }
                        }
                        Err(e) => {
                            writeln!(handle, "Error fetching issues: {}", e).unwrap_or_default();
                            let _ = handle.flush();
                            process::exit(1);
                        }
                    }
                }
                IssueManagementTool::Clickup => {
                    let mut clickup_config = load_clickup_config(directory)
                        .ok()
                        .expect("Config should be available");
                    if clickup_config.is_none() {
                        clickup_config = extract_clickup_spaces_data(&directory, &matches, &client)
                            .await
                            .ok()
                            .expect("Config should be available");
                    }

                    // Get issue_id from argument or prompt user
                    let issue_id = match arg.value_of("issue_id") {
                        Some(id) if !id.is_empty() => id.to_string(),
                        _ => match Text::new("Enter the ClickUp issue ID:").prompt() {
                            Ok(id) => id,
                            Err(_) => {
                                writeln!(handle, "Input cancelled").unwrap_or_default();
                                let _ = handle.flush();
                                process::exit(1);
                            }
                        },
                    };

                    let task_url = format!(
                        "https://api.clickup.com/api/v2/task/{}?include_markdown_description=true",
                        issue_id
                    );
                    match make_clickup_request(&client, &task_url, clickup_api_key.unwrap()).await {
                        Ok(task_response) => match task_response.get("markdown_description") {
                            Some(task_description) => {
                                let _ =
                                    prompt_claude(task_description.as_str().unwrap(), directory);
                            }
                            None => {
                                writeln!(handle, "{}", "Error fetching task for clickup")
                                    .unwrap_or_default();
                            }
                        },
                        Err(e) => {
                            writeln!(handle, "\nError fetching tasks: {}", e).unwrap_or_default();
                        }
                    }
                    let _ = handle.flush();
                }
            }
        }
        ("spaces", Some(arg)) => {
            debug!("Calling subcommnand workspaces {:?}", arg);

            match tool {
                IssueManagementTool::GitHub => {
                    let _ = extract_github_spaces_data(&directory, &client, github_api_token)
                        .await
                        .unwrap();
                }
                IssueManagementTool::Clickup => {
                    let _ = extract_clickup_spaces_data(&directory, &matches, &client)
                        .await
                        .unwrap();
                }
            }
        }
        ("issue", Some(arg)) => {
            info!("issue {:?}", arg);

            let issue_id = arg.value_of("issue").unwrap_or("");
            if issue_id.is_empty() {
                writeln!(handle, "Issue ID is required").unwrap_or_default();
                let _ = handle.flush();
                process::exit(1);
            }

            let url = format!(
                "https://api.clickup.com/api/v2/task/{}?include_markdown_description=true",
                issue_id
            );

            let body = match make_clickup_request(
                &client,
                &url,
                matches.value_of("clickup_api_key").unwrap(),
            )
            .await
            {
                Ok(b) => b,
                Err(e) => {
                    writeln!(handle, "Error fetching issue: {}", e).unwrap_or_default();
                    let _ = handle.flush();
                    process::exit(1);
                }
            };

            // Extract and output the description
            if let Some(description) = body.get("markdown_description") {
                writeln!(handle, "Issue Description:").unwrap_or_default();
                writeln!(
                    handle,
                    "{}",
                    description.as_str().unwrap_or("No description available")
                )
                .unwrap_or_default();
            } else if let Some(description) = body.get("description") {
                writeln!(handle, "Issue Description:").unwrap_or_default();
                writeln!(
                    handle,
                    "{}",
                    description.as_str().unwrap_or("No description available")
                )
                .unwrap_or_default();
            } else {
                writeln!(handle, "No description found for this issue").unwrap_or_default();
            }

            // Also output other useful information
            if let Some(name) = body.get("name") {
                writeln!(handle, "\nIssue Name: {}", name.as_str().unwrap_or("N/A"))
                    .unwrap_or_default();
            }

            if let Some(status) = body.get("status") {
                if let Some(status_name) = status.get("status") {
                    writeln!(handle, "Status: {}", status_name.as_str().unwrap_or("N/A"))
                        .unwrap_or_default();
                }
            }

            let _ = handle.flush();
        }
        _ => {}
    }
}
