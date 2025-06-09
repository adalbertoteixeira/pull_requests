use std::{
    fs,
    io::{self, Write},
    path::Path,
    process::{self, Command, Stdio},
    time::Duration,
};

use clap::ArgMatches;
use indicatif::ProgressBar;
use inquire::{Select, Text};
use log::{debug, info};
use reqwest::Client;

use crate::{
    storage::{
        BranchYamlConfig, get_branch_config, load_branch_config, load_clickup_config,
        load_github_config, save_branch_config,
    },
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

async fn automation_from_issue_id(
    directory: &str,
    issue_id: &str,
    client: &Client,
    clickup_api_key: &str,
) {
    let stdout = io::stdout(); // get the global stdout entity
    let mut handle = io::BufWriter::new(&stdout); // optional: wrap that handle in a buffer
    let mut existing_branch: Option<BranchYamlConfig> = None;
    // Search for files in .commit_message/ directory that start with issue_id
    let commit_message_dir = Path::new(directory).join(".commit_message");
    if commit_message_dir.exists() {
        let matching_files: Vec<_> = fs::read_dir(&commit_message_dir)
            .unwrap()
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let file_name = entry.file_name().into_string().ok()?;
                if file_name.starts_with(issue_id) && file_name.ends_with(".yaml") {
                    Some(entry.path())
                } else {
                    None
                }
            })
            .collect();

        if matching_files.len() > 1 {
            writeln!(handle, "More than one file with the same id was found.").unwrap_or_default();
            writeln!(handle, "Please delete all but one").unwrap_or_default();
            writeln!(handle, "\nFound files:").unwrap_or_default();
            for file in matching_files {
                writeln!(handle, "  - {}", file.display()).unwrap_or_default();
            }
            let _ = handle.flush();
            process::exit(1);
        } else if matching_files.len() == 1 {
            // Extract branch name from filename (remove .yaml extension)
            let file_path = &matching_files[0];
            let file_name = file_path.file_stem().unwrap().to_str().unwrap();

            // Call load_branch_config with the extracted branch name
            existing_branch = get_branch_config(file_name, directory).expect("branch to be loaded");
            let _ = handle.flush();
        }
    }

    let mut issue_description = None;
    let mut issue_name = None;
    let mut claude_suggestion = None;
    let mut git_branch = None;
    match existing_branch.is_some() {
        true => {
            let branch_data = existing_branch.unwrap();
            issue_description = branch_data.issue_description;
            issue_name = branch_data.issue_name;
            claude_suggestion = branch_data.claude_suggestion;
            git_branch = Some(branch_data.branch_name);
        }
        false => {
            let url = format!(
                "https://api.clickup.com/api/v2/task/{}?include_markdown_description=true",
                issue_id
            );

            let body = match make_clickup_request(&client, &url, clickup_api_key).await {
                Ok(b) => b,
                Err(e) => {
                    writeln!(handle, "Error fetching issue: {}", e).unwrap_or_default();
                    let _ = handle.flush();
                    process::exit(1);
                }
            };

            // Extract and output the description
            if let Some(description) = body.get("markdown_description") {
                issue_description = Some(description.as_str().unwrap_or("N/A").to_string());
            } else if let Some(description) = body.get("description") {
                issue_description = Some(description.as_str().unwrap_or("N/A").to_string());
            }

            // Also output other useful information
            if let Some(name) = body.get("name") {
                issue_name = Some(name.as_str().unwrap_or("N/A").to_string());
            }

            if let Some(status) = body.get("status") {
                if let Some(status_name) = status.get("status") {
                    writeln!(handle, "Status: {}", status_name.as_str().unwrap_or("N/A"))
                        .unwrap_or_default();
                }
            }

            let _ = handle.flush();
            let name_clone = issue_name.clone().unwrap();
            let built_git_branch = create_git_branch(issue_id, &name_clone);
            git_branch = Some(built_git_branch.clone());

            let _ = save_branch_config(
                &built_git_branch.clone(),
                directory,
                None,
                None,
                None,
                None,
                Some(issue_id.to_string()),
                issue_name.clone(),
                issue_description.clone(),
                None,
            );
        }
    }

    if claude_suggestion.is_none() {
        debug!("Asking for suggestion");
        let prompt_text = format!(
            "Given the following issue description, define the best approach to implement these changes\n{:?}.\nWrite the output outlining all the files the developer might need to look into, suggesting the best viable path to implement the changes.\nOutput the result starting with: \"Our suggestion to implement the needed changes is the following\"",
            &issue_description
        );
        let claude_suggestion_prompt_result =
            prompt_claude_one_off(&prompt_text, directory).expect("result from prompt analyzis");
        claude_suggestion = Some(claude_suggestion_prompt_result.clone());
        debug!("Getting suggestion {:?}", claude_suggestion);
        let _ = save_branch_config(
            &git_branch.unwrap(),
            directory,
            None,
            None,
            None,
            None,
            Some(issue_id.to_string()),
            issue_name,
            issue_description.clone(),
            Some(claude_suggestion_prompt_result),
        );
    }

    println!("suggestion is{:?}", claude_suggestion);
    let prompt_text = format!(
        "Given the following issue description, implement all the changes required to the codebase:\n{:?}\n{:?}",
        &issue_description, &claude_suggestion
    );

    let _ = prompt_claude(&prompt_text, directory);
}

fn create_git_branch(issue_id: &str, issue_name: &str) -> String {
    // Convert to lowercase, replace spaces with dashes, and keep only alphanumeric and dash characters
    let parsed_name: String = issue_name
        .to_lowercase()
        .chars()
        .map(|c| if c.is_whitespace() { '-' } else { c })
        .filter(|c| c.is_alphanumeric() || *c == '-')
        .take(50)
        .collect();

    format!("{}-{}", issue_id, parsed_name)
}

fn prompt_claude_one_off(prompt_text: &str, directory: &str) -> Result<String, io::Error> {
    let stdout = io::stdout();
    let mut handle = io::BufWriter::new(&stdout);

    let bar = ProgressBar::new_spinner();
    bar.enable_steady_tick(Duration::from_millis(100));
    // Spawn an interactive shell
    let output = Command::new("claude")
        .arg("-p")
        .arg(prompt_text)
        .current_dir(&directory)
        // .stdin(Stdio::inherit())
        // .stdout(Stdio::inherit())
        // .stderr(Stdio::inherit())
        // .spawn()
        // .expect("Failed to spawn interactive shell");
        .output()
        .unwrap();
    // Wait for the shell to exit
    // let status = child.wait().expect("Failed to wait for shell");
    // info!("Shell exited with status: {:?}", status);
    // writeln!(
    //     handle,
    //     "{}",
    //     "Work is done. We are working to implement the next automations in the future."
    // )
    // .unwrap_or_default();
    // let _ = handle.flush();
    bar.finish();
    if !output.status.success() {
        let error = str::from_utf8(&output.stderr).unwrap();
        writeln!(handle, "{}", error,).unwrap_or_default();
        let _ = handle.flush();
        process::exit(1)
    }

    let result_stdout_string = str::from_utf8(&output.stdout).unwrap();
    println!("{:?}", result_stdout_string);
    Ok(result_stdout_string.to_string())
}

fn prompt_claude(prompt_text: &str, directory: &str) {
    let stdout = io::stdout();
    let mut handle = io::BufWriter::new(&stdout);

    // Spawn an interactive shell
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
    let clickup_api_key = matches.value_of("clickup_api_key").unwrap();
    match matches.subcommand() {
        ("issues", Some(arg)) => {
            match tool {
                IssueManagementTool::GitHub => {
                    let github_config = load_github_config(directory).ok();
                    if github_config.is_some() {
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
                                    let prompt_text = format!(
                                        "Given the following issue description, implement all the changes required to the codebase:\n{:?}",
                                        &selected_issue.body
                                    );

                                    let _ = prompt_claude(&prompt_text, directory);
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
                    let clickup_config = load_clickup_config(directory)
                        .ok()
                        .expect("Config should be available");
                    if clickup_config.is_none() {
                        extract_clickup_spaces_data(&directory, &matches, &client)
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
                    match make_clickup_request(&client, &task_url, clickup_api_key).await {
                        Ok(task_response) => match task_response.get("markdown_description") {
                            Some(task_description) => {
                                let prompt_text = format!(
                                    "Given the following issue description, implement all the changes required to the codebase:\n{:?}",
                                    &task_description
                                );

                                let _ = prompt_claude(&prompt_text, directory);
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

                    let _ =
                        automation_from_issue_id(directory, &issue_id, &client, &clickup_api_key);
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

            let issue_id = arg.value_of("issue-id").unwrap_or("");
            if issue_id.is_empty() {
                writeln!(handle, "Issue ID is required").unwrap_or_default();
                let _ = handle.flush();
                process::exit(1);
            }

            let _ = automation_from_issue_id(directory, issue_id, &client, &clickup_api_key);
        }
        _ => {}
    }
}
