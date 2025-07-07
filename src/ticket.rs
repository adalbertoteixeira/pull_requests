use std::{
    fs,
    io::{self, Write},
    path::Path,
    process::{self},
};

use clap::ArgMatches;
use inquire::{Confirm, Select, Text};
use log::{debug, info};
use reqwest::Client;
use serde_json;

use crate::{
    branch_utils, prompts,
    storage::{
        self, BranchYamlConfig, get_branch_config, load_clickup_config, load_github_config,
        save_branch_config,
    },
    types::github_types::GithubIssue,
    utils::{
        claude::{prompt_claude, prompt_claude_one_off},
        extract_clickup_spaces_data::{extract_clickup_spaces_data, make_clickup_request},
        extract_github_spaces_data::{extract_github_spaces_data, get_github_user_issues},
    },
};

#[derive(Debug, Clone)]
pub enum IssueManagementTool {
    GitHub,
    Clickup,
}

fn define_issue_management_tool(
    github_api_token: Option<&str>,
    clickup_api_key: Option<&str>,
) -> IssueManagementTool {
    let stdout = io::stdout();
    let mut handle = io::BufWriter::new(&stdout);

    let has_github = !github_api_token.is_some_and(|x| x.len() > 0);
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
    mcp_config: Option<&str>,
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

    debug!("Asking for suggestions ---- {}, {} 2", directory, issue_id);
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
            let cmd_arg = format!(
                "cd {directory} && gh issue view {issue_id} --json assignees,author,body,closed,closedAt,closedByPullRequestsReferences,comments,createdAt,id,isPinned,labels,milestone,number,projectCards,reactionGroups,state,stateReason,title,updatedAt,url",
            ).to_owned();
            debug!("cmd arg {}", cmd_arg);
            let gh_output = std::process::Command::new("sh")
                .arg("-c")
                .arg(cmd_arg)
                .output();

            debug!("{:?}", gh_output);
            match gh_output {
                Ok(output) => {
                    if output.status.success() {
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        debug!("GitHub issue data: {}", stdout);

                        // Parse the JSON response into GithubIssue struct
                        match serde_json::from_str::<GithubIssue>(&stdout) {
                            Ok(github_issue) => {
                                // Extract description from body
                                issue_description = github_issue.body.clone();

                                // Extract title as name
                                issue_name = github_issue.title.clone();
                            }
                            Err(e) => {
                                debug!("Failed to parse GitHub issue JSON: {}", e);
                            }
                        }
                    } else {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        writeln!(handle, "{}", stderr).unwrap_or_default();
                        let _ = handle.flush();
                        process::exit(1);
                    }
                }
                Err(_) => {
                    writeln!(handle, "{}", "Couldn't get the issue").unwrap_or_default();
                    let _ = handle.flush();
                    process::exit(1);
                }
            }

            println!(
                "{:?}, {:?}, {:?}",
                issue_description, issue_name, git_branch
            );
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
        let prompt_header = "You are a technical product manager.\nGiven the following  GitHub issue text, extend the issue to support the developer implementing it.\n\nAdd whatever could be useful:\n- debug steps;\n- file paths to potentially look into;\n- helpful notes to keep in mind;\n- whatever might be helpful context.\n\n\n\n
\"";
        let prompt_text = &issue_description.clone().unwrap();

        let claude_suggestion_prompt_result =
            prompt_claude_one_off(&prompt_header, &prompt_text, directory, mcp_config)
                .expect("should get Claude response");
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

    let prompt_issue_description = issue_description.clone().unwrap();
    let prompt_claude_suggestion = claude_suggestion.clone().unwrap();
    let prompt_text = format!(
        r#"Given the following issue description, implement all the changes required to the codebase:\n{:?}\n{:?}"#,
        &prompt_issue_description, &prompt_claude_suggestion
    );

    let _ = prompt_claude(&prompt_text, directory, mcp_config);
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

pub async fn ticket(
    matches: ArgMatches<'static>,
    directory: &str,
    github_api_token: Option<&str>,
    mcp_config: Option<&str>,
    _has_gh: bool,
    git_branch: &str,
) {
    info!("Ticket command");
    let stdout = io::stdout(); // get the global stdout entity
    let mut handle = io::BufWriter::new(&stdout); // optional: wrap that handle in a buffer
    debug!("matches {:?}", matches);

    let clickup_api_key = matches.value_of("clickup_api_key");
    let tool = IssueManagementTool::GitHub;

    let client = reqwest::Client::new();
    let clickup_api_key = matches.value_of("clickup_api_key").unwrap();
    match matches.subcommand() {
        ("update_pr", Some(_arg)) => {
            let stored_config = storage::get_branch_config(git_branch, directory)
                .expect("Should hv a stored branch");
            if stored_config
                .as_ref()
                .is_none_or(|c| c.pr_template.is_none())
            {
                writeln!(handle, "No pr template built.").unwrap_or_default();
                let _ = handle.flush();
                process::exit(1);
            }
            let pr_template = stored_config.unwrap().pr_template.unwrap();
            let _ = branch_utils::update_pull_request(directory, &pr_template);
        }
        ("create_pr_template", Some(_arg)) => {
            let issue_id = branch_utils::issue_id(&git_branch);

            let use_claude = matches.is_present("claude");
            let pr_template = Some(prompts::pr_template_prompt(
                &issue_id,
                use_claude,
                &directory,
                &git_branch,
            ));

            let _ = storage::save_branch_config(
                git_branch,
                directory,
                pr_template.clone(),
                None,
                None,
                None,
                None,
                None,
                None,
                None,
            );

            // Ask if user wants to update the PR in GitHub
            let update_pr_prompt = Confirm::new("Update ticket in Github?")
                .with_default(true)
                .prompt();

            match update_pr_prompt {
                Ok(true) => {
                    if let Some(template) = pr_template {
                        let _ = branch_utils::update_pull_request(directory, &template);
                    } else {
                        writeln!(handle, "No PR template found to update with").unwrap_or_default();
                        let _ = handle.flush();
                    }
                }
                Ok(false) => {
                    process::exit(0);
                }
                Err(_) => {
                    writeln!(handle, "Input cancelled").unwrap_or_default();
                    let _ = handle.flush();
                    process::exit(1);
                }
            }
        }
        ("create", Some(arg)) => {
            // Extract OWNER/REPO from directory/.github/config
            let github_config_path = Path::new(directory).join(".git").join("config");
            debug!("github_config_path {:?}", github_config_path);
            let mut owner_repo: Option<String> = None;
            if github_config_path.exists() {
                match fs::read_to_string(&github_config_path) {
                    Ok(config_content) => {
                        debug!("github_config_path {:?}", config_content);
                        // Parse the config to extract owner/repo
                        // This is a simplified parser - you might need to adjust based on actual config format
                        owner_repo = config_content
                            .lines()
                            .find(|line| line.contains("url") || line.contains("remote"))
                            .and_then(|line| {
                                debug!("line {:?}", line);
                                if line.contains("github.com") {
                                    let parts: Vec<&str> = line.split('/').collect();
                                    if parts.len() >= 2 {
                                        let owner = parts[parts.len() - 2];
                                        let repo = parts[parts.len() - 1].replace(".git", "");
                                        Some(format!("{}/{}", owner, repo))
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                }
                            })
                    }
                    Err(_) => {}
                }
            };

            debug!("ONWER/REPO from config file {:?}", owner_repo);
            if owner_repo.is_none() {
                owner_repo = match Text::new("Enter the owner/repo (e.g., wearebenlabs/repo-name):")
                    .prompt()
                {
                    Ok(owner_repo) => Some(owner_repo),
                    Err(_) => {
                        writeln!(handle, "Input cancelled").unwrap_or_default();
                        let _ = handle.flush();
                        process::exit(1);
                    }
                }
            }

            debug!("ONWER/REPO from prompt {:?}", owner_repo);
            // Get milestones using the owner/repo
            let milestones_output = std::process::Command::new("gh")
                .arg("api")
                .arg(format!("repos/{}/milestones", owner_repo.as_ref().unwrap()))
                .current_dir(directory)
                .output();

            debug!("milestones_output {:?}", milestones_output);
            let pr_title = match Text::new("What is the PR title?").prompt() {
                Ok(title) => title,
                Err(_) => {
                    writeln!(handle, "Input cancelled").unwrap_or_default();
                    let _ = handle.flush();
                    process::exit(1);
                }
            };
            debug!("pr title {:?}", pr_title);
        }
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

                                    let _ = prompt_claude(&prompt_text, directory, mcp_config);
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
                                    &task_description.as_str().unwrap()
                                );

                                let _ = prompt_claude(&prompt_text, directory, mcp_config);
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

                    let _ = automation_from_issue_id(
                        directory,
                        &issue_id,
                        &client,
                        &clickup_api_key,
                        mcp_config,
                    );
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

            let _ = automation_from_issue_id(
                directory,
                issue_id,
                &client,
                &clickup_api_key,
                mcp_config,
            )
            .await;
        }
        _ => {}
    }
}
