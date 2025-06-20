use indicatif::ProgressBar;
use inquire::Confirm;
use log::{debug, error, info};
use std::collections::HashMap;
use std::time::Duration;
use std::{
    io::{self, Write},
    process::{self, Command},
    str,
};

use lazy_static::lazy_static;
use regex::Regex;

use crate::storage;
use crate::utils::extract_github_spaces_data::{make_github_post, make_github_request};
use reqwest::Client;

#[derive(Debug)]
pub struct GithubRepoParts {
    pub path: Option<String>,
    pub owner: Option<String>,
    pub repo: Option<String>,
    pub owner_and_path: Option<String>,
}

lazy_static! {
    static ref ISSUE_REGEX: Regex = Regex::new(r"^(\w*)(?:-)?(.*)?$").unwrap();
    static ref TEST_REGEX: Regex =
        Regex::new(r"\.spec\.|\.test\.|\.jest\.|\.config\.|jest\.unit").unwrap();
    static ref DOCS_REGEX: Regex = Regex::new(r"\.md$").unwrap();
    static ref BUILD_REGEX: Regex = Regex::new(r"package.json|yarn.lock$").unwrap();
    static ref CI_REGEX: Regex = Regex::new(r"^\.|Dockerfile|/iac/").unwrap();
}

pub fn validate_branch(git_branch: &str) {
    let protected_branches = vec!["main", "production"];
    let stdout = io::stdout(); // get the global stdout entity
    let mut handle = io::BufWriter::new(&stdout); // optional: wrap that handle in a buffer
    if protected_branches.contains(&git_branch) {
        writeln!(handle, "Branch is {}, refusing to continue.", &git_branch).unwrap_or_default();
        let _ = handle.flush();
        process::exit(1);
    }
}

pub fn issue_id(git_branch: &str) -> String {
    debug!("Git branch is {}", git_branch);
    let Some(caps) = ISSUE_REGEX.captures(git_branch) else {
        info!(
            "Issue id not found in branch name. Returning empty string. Git branch: {}",
            git_branch
        );
        return "".to_owned();
    };
    info!("Captures: {:?}", caps);
    let caps_entry = caps.get(1).unwrap().as_str();
    info!("Captures entries{:?}", caps_entry);
    if caps_entry.is_empty() {
        info!(
            "No issue id found in branch name. Returning empty string. Git branch: {}",
            git_branch
        );
        return "".to_owned();
    }

    return caps_entry.to_string();
}

pub fn branch_name(git_branch: &str) -> String {
    debug!("Git branch is {}", git_branch);
    let Some(caps) = ISSUE_REGEX.captures(git_branch) else {
        info!(
            "Issue id not found in branch name. Returning empty string. Git branch: {}",
            git_branch
        );
        return "".to_owned();
    };

    let caps_entry = caps.get(2).unwrap().as_str();
    info!("Branch name captures: {:?}", caps_entry);
    // if caps_entry.is_none() {
    //     return "".to_owned();
    // }
    let mut commit_suggestion = caps_entry.replace("-", " ");
    commit_suggestion = commit_suggestion.trim().to_owned();
    let mut commit_suggestion_trimmed_length = Vec::with_capacity(55);
    for (index, c) in commit_suggestion.char_indices() {
        if index < 56 {
            commit_suggestion_trimmed_length.push(c);
        } else {
            break;
        }
    }
    return commit_suggestion_trimmed_length.iter().collect::<String>();
}

pub fn changed_file_names(directory: &str) -> Vec<String> {
    let cmd_arg = format!("cd {directory} && git diff --cached --name-only");
    let output = Command::new("sh").arg("-c").arg(cmd_arg).output().unwrap();
    if !output.status.success() {
        let error = str::from_utf8(&output.stderr).unwrap();
        error!("{:?}", error);
        debug!("Couldn't find changed files.");
        process::exit(1)
    }
    let files_as_string = String::from_utf8_lossy(&output.stdout);
    let files: Vec<&str> = files_as_string.split('\n').collect();
    return files.iter().map(|s| s.trim().to_owned()).collect();
}

fn suggest_type(used_types: &Vec<&str>, is_new_branch: &bool) -> Option<String> {
    info!("Trying to suggest a PR type");
    if used_types.len() == 1 {
        let single_type = used_types.first().unwrap();
        info!("Found only one type: {}", single_type);
        if single_type == &"feat" && is_new_branch != &true {
            return Some("refactor".to_string());
        }
        return Some(single_type.to_string());
    }
    if used_types.len() > 1 && is_new_branch == &true {
        info!("New branch and more than one type: suggesting feature");
        return Some("feat".to_string());
    }
    if used_types.len() > 1 && is_new_branch != &true && used_types.first().unwrap() == &"test" {
        info!("Existing branch and more than one type but more used in test: suggesting test");
        return Some("test".to_string());
    }
    if used_types.len() > 1 && is_new_branch != &true {
        info!("Existing branch and more than one type: suggesting refactor");
        return Some("refactor".to_string());
    }

    return None; //"".to_string();
}

pub fn find_changed_file_types(directory: &str, is_new_branch: &bool) -> (Option<String>, usize) {
    let mut changed_types = HashMap::from([
        ("test", 0),
        ("docs", 0),
        ("build", 0),
        ("ci", 0),
        ("code", 0),
        // ?
        ("feat", 0),
        ("fix", 0),
        ("style", 0),
        ("refactor", 0),
        ("perf", 0),
        ("chore", 0),
        ("revert", 0),
    ]);
    let files_changed = changed_file_names(directory);
    if files_changed.is_empty() || files_changed.len() == 1 && files_changed[0].len() == 0 {
        let no_staged_files_prompt = Confirm::new(
            "No staged files were found, nothing will be added. Do you wish to continue?",
        )
        .with_default(false)
        .prompt();

        match no_staged_files_prompt {
            Ok(response) => {
                if response == false {
                    process::exit(0);
                }
            }
            Err(_) => {}
        }
    }
    info!("Files changed: {:?}", files_changed);
    for file in &files_changed {
        if file.is_empty() {
            continue;
        }
        if CI_REGEX.is_match(file) {
            info!("Matches CI");
            changed_types.entry("ci").and_modify(|count| *count += 1);
            continue;
        }
        if TEST_REGEX.is_match(file) {
            info!("Matches test");
            changed_types.entry("test").and_modify(|count| *count += 1);
            continue;
        }
        if DOCS_REGEX.is_match(file) {
            info!("Matches docs");
            changed_types.entry("docs").and_modify(|count| *count += 1);
            continue;
        }
        if BUILD_REGEX.is_match(file) {
            info!("Matches build");
            changed_types.entry("build").and_modify(|count| *count += 1);
            continue;
        }
        info!("No matches");
        changed_types.entry("feat").and_modify(|count| *count += 1);
    }

    changed_types.retain(|_, v| *v != 0);
    info!("Changed types: {:?}", changed_types);
    let mut sorted_changed_types: Vec<_> = changed_types.into_iter().collect();
    sorted_changed_types.sort_by(|a, b| b.0.cmp(&a.0));
    info!("Changed types: {:?}", sorted_changed_types);
    let used_types: Vec<&str> = sorted_changed_types.into_iter().map(|(k, _)| k).collect();
    // let used_types: Vec<&str> = sorted_changed_types.keys().cloned().collect();
    let proposed_type = suggest_type(&used_types, &is_new_branch);
    info!("Proposed type: {:?}", proposed_type);
    if proposed_type.is_none() {
        return (None, used_types.len());
    }

    let proposed_string = Option::expect(proposed_type, "Expected a proposed reason");
    return (Some(proposed_string), used_types.len());
}

pub fn commit_pr(
    directory: &str,
    commit_message: &str,
    additional_commit_message: Vec<String>,
    git_branch: &str,
    pr_template: Option<String>,
    no_verify: bool,
) -> Result<Option<i32>, io::Error> {
    let mut cmd_arg = format!(
        r#"cd {} && git commit -m  "{}""#,
        &directory, &commit_message
    );
    if no_verify {
        cmd_arg.push_str(" --no-verify");
    }
    if additional_commit_message.len() > 0 {
        for message in &additional_commit_message {
            cmd_arg.push_str(&format!(r#" -m "{}""#, message));
        }
    }
    info!("Executing command: {}", cmd_arg);
    let stdout = io::stdout(); // get the global stdout entity
    let mut handle = io::BufWriter::new(&stdout); // optional: wrap that handle in a buffer
    writeln!(
        handle,
        "{}",
        "Running git commit. This might take some time depending on the pre-commit hooks."
    )
    .unwrap_or_default();
    let _ = handle.flush();
    let bar = ProgressBar::new_spinner();
    bar.enable_steady_tick(Duration::from_millis(100));
    let output = Command::new("sh")
        .arg("-c")
        .arg(cmd_arg)
        .output()
        .expect("Failed to run  process");

    bar.finish();
    io::stderr().write_all(&output.stderr).unwrap();
    io::stdout().write_all(&output.stdout).unwrap();
    debug!("Commit message result is {:?}", output.status.code());
    let branch_config_save_result = storage::save_branch_config(
        &git_branch,
        &directory,
        pr_template.clone(),
        Some(commit_message.to_string()),
        Some(additional_commit_message.clone()),
        output.status.code(),
        None,
        None,
        None,
        None,
    );
    debug!(
        "Save config result is {:?}",
        branch_config_save_result.is_ok()
    );
    Ok(output.status.code())
}

pub async fn push_pr(
    directory: &str,
    no_verify: bool,
    ci_mode: bool,
    github_api_token: Option<&str>,
    git_branch: &str,
    commit_message: Option<&str>,
    pr_template: Option<String>,
    has_gh: bool,
) -> Option<i32> {
    info!("Starting pr push");
    let mut cmd_arg = format!(r#"cd {} && git push"#, &directory);
    if no_verify {
        cmd_arg.push_str(" --no-verify");
    }
    info!("Executing command: {}", cmd_arg);
    let stdout = io::stdout(); // get the global stdout entity
    let mut handle = io::BufWriter::new(&stdout); // optional: wrap that handle in a buffer
    let mut push_message = "Pushing branch.".to_owned();
    if no_verify {
        push_message.push_str(" Skipping pre-push hooks.");
    } else {
        push_message.push_str(" This might take some time depending on the pre-push hooks.");
    }
    writeln!(handle, "{}", push_message).unwrap_or_default();
    let _ = handle.flush();
    let bar = ProgressBar::new_spinner();
    bar.enable_steady_tick(Duration::from_millis(100));
    let output = Command::new("sh")
        .arg("-c")
        .arg(cmd_arg)
        .output()
        .expect("Failed to run  process");

    bar.finish();

    let cmd_arg_status_code = output.status.code();
    info!("Resulting status code is: {:?}", cmd_arg_status_code);

    // Check if the command failed
    if cmd_arg_status_code.is_none() || cmd_arg_status_code.is_some_and(|x| x != 0) {
        let stderr = str::from_utf8(&output.stderr).unwrap();

        // Check if the error is about missing upstream branch
        if stderr.contains("has no upstream branch")
            && stderr.contains("set the remote as upstream")
        {
            writeln!(handle, "\n{}", "The current branch has no upstream branch.")
                .unwrap_or_default();
            writeln!(handle, "{}", stderr).unwrap_or_default();
            let _ = handle.flush();

            let set_upstream_prompt = match ci_mode {
                true => Ok(true),
                false => Confirm::new(
                    "Do you want to push and set the current branch as upstream on origin?",
                )
                .with_default(true)
                .prompt(),
            };

            match set_upstream_prompt {
                Ok(response) => {
                    let bar = ProgressBar::new_spinner();
                    bar.enable_steady_tick(Duration::from_millis(100));
                    if response {
                        // Check if current branch exists on remote
                        let branch_cmd = format!("cd {} && git ls-remote --heads origin {}", directory, git_branch);
                        let branch_output = Command::new("sh")
                            .arg("-c")
                            .arg(branch_cmd)
                            .output()
                            .expect("Failed to check remote branch");

                        let branch_exists_on_remote = match branch_output.status.code() {
                            Some(0) => {
                                let stdout = String::from_utf8_lossy(&branch_output.stdout);
                                stdout.contains(git_branch)
                            }
                            _ => false,
                        };

                        if !branch_exists_on_remote {
                            // Push with --set-upstream
                            let mut upstream_cmd = format!(
                                "cd {} && git push --set-upstream origin {}",
                                directory, git_branch
                            );
                            if no_verify {
                                upstream_cmd.push_str(" --no-verify");
                            }
                            info!("Executing command: {}", upstream_cmd);

                            writeln!(handle, "{}", "Setting upstream and pushing...")
                                .unwrap_or_default();
                            let _ = handle.flush();

                            let upstream_output = Command::new("sh")
                                .arg("-c")
                                .arg(upstream_cmd)
                                .output()
                                .expect("Failed to run upstream push");

                            bar.finish();
                            io::stderr().write_all(&upstream_output.stderr).unwrap();
                            io::stdout().write_all(&upstream_output.stdout).unwrap();

                            return upstream_output.status.code();
                        } else {
                            // Branch exists on remote, just push
                            let mut push_cmd = format!("cd {} && git push", directory);
                            if no_verify {
                                push_cmd.push_str(" --no-verify");
                            }
                            info!("Executing command: {}", push_cmd);

                            writeln!(handle, "{}", "Branch exists on remote, pushing...")
                                .unwrap_or_default();
                            let _ = handle.flush();

                            let push_output = Command::new("sh")
                                .arg("-c")
                                .arg(push_cmd)
                                .output()
                                .expect("Failed to run push");

                            bar.finish();
                            io::stderr().write_all(&push_output.stderr).unwrap();
                            io::stdout().write_all(&push_output.stdout).unwrap();

                            return push_output.status.code();
                        }
                    } else {
                        writeln!(handle, "{}", "Push cancelled by user.").unwrap_or_default();
                        let _ = handle.flush();
                    }
                }
                Err(_) => {
                    writeln!(handle, "{}", "Failed to get user input. Push cancelled.")
                        .unwrap_or_default();
                    let _ = handle.flush();
                }
            }
        } else {
            // Other error, show the original output
            print!("OUTPUT: {:?}", cmd_arg_status_code);
            print!("OUTPUT: {:?}", output);
            print!("OUTPUT: {:?}", stderr);
            io::stderr().write_all(&output.stderr).unwrap();
            io::stdout().write_all(&output.stdout).unwrap();
        }
    } else {
        // Success case - show output normally
        io::stderr().write_all(&output.stderr).unwrap();
        io::stdout().write_all(&output.stdout).unwrap();
    }

    info!("Will try updating the PR {:?}", has_gh);
    if has_gh == true {
        let parts = get_branch_origin_parts(directory);
        info!("Parts: {:?}", parts);
        // process::exit(1);
        let pr_exists = check_existing_pr(directory);

        if pr_exists {
            info!("\n\nExisting PR found");
        }
        info!("No pr created\nCommit message: {:?}", commit_message);
        // if there's no pr, create it
        //
        create_pr(
            directory,
            github_api_token,
            git_branch,
            commit_message,
            pr_template,
            &parts.unwrap().owner.unwrap(),
        )
        .await
        .expect("PR should be created");
        // when there's a pr:
        // - update assignee
        // - ask to push template
    }

    cmd_arg_status_code
}

pub fn get_branch_origin_parts(directory: &str) -> Result<GithubRepoParts, io::Error> {
    let mut repo_parts = GithubRepoParts {
        path: None,
        owner: None,
        repo: None,
        owner_and_path: None,
    };
    let cmd_arg = format!("cd {} && git remote get-url origin", directory);
    let output = Command::new("sh").arg("-c").arg(cmd_arg).output()?;

    if !output.status.success() {
        return Ok(repo_parts);
    }

    let remote_url = String::from_utf8_lossy(&output.stdout).trim().to_string();

    let repo_full_path = match remote_url.contains("github.com") {
        true => {
            if remote_url.starts_with("git@github.com:") {
                remote_url
                    .strip_prefix("git@github.com:")
                    .unwrap_or("")
                    .strip_suffix(".git")
            } else if remote_url.starts_with("https://github.com/") {
                remote_url
                    .strip_prefix("https://github.com/")
                    .unwrap_or("")
                    .strip_suffix(".git")
            } else {
                None
            }
        }
        false => None,
    };

    if repo_full_path.is_some() {
        repo_parts.path = Some(remote_url.clone());
        repo_parts.owner_and_path = Some(repo_full_path.unwrap().to_owned());
        let parts: Vec<&str> = repo_full_path.unwrap().split('/').collect();
        repo_parts.owner = Some(parts[0].to_owned());
        repo_parts.repo = Some(parts[1].to_owned());
    }

    Ok(repo_parts)
}

pub async fn create_pr(
    directory: &str,
    github_api_token: Option<&str>,
    git_branch: &str,
    commit_message: Option<&str>,
    pr_template: Option<String>,
    owner: &str,
) -> Result<Option<serde_json::Value>, Box<dyn std::error::Error>> {
    if github_api_token.is_none_or(|x| x.len() == 0) {
        return Ok(None);
    }

    let repo_parts = get_branch_origin_parts(directory).expect("should have a response");
    if repo_parts.owner_and_path.is_none() {
        return Ok(None);
    }
    let owner_and_path = repo_parts.owner_and_path.unwrap();
    let client = Client::new();
    let url = format!("https://api.github.com/repos/{}/pulls", owner_and_path);
    let mut body = HashMap::new();
    body.insert(
        "title",
        commit_message.expect("commit message should be set"),
    );
    body.insert("head", "main");
    let pr_body = match pr_template.is_some() {
        true => pr_template.to_owned().unwrap(),
        false => "".to_owned(),
    };
    body.insert("body", &pr_body);
    let head = &format!("{}:{}", owner, git_branch).to_owned();
    body.insert("head", head);
    info!("pull url {:?}", url);
    match make_github_post(&client, &url, github_api_token, body).await {
        Ok(response) => {
            if let Some(prs) = response.as_array() {
                if !prs.is_empty() {
                    // Return the first PR found
                    return Ok(Some(prs[0].clone()));
                }
            }
            Ok(None)
        }
        Err(_) => Ok(None),
    }
}
pub fn check_existing_pr(directory: &str) -> bool {
    let cmd_arg = format!("cd {} && gh pr view", directory);
    let output = Command::new("sh").arg("-c").arg(cmd_arg).output();

    match output {
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("no git remotes found") {
                return false;
            }
            match output.status.code() {
                Some(0) => true,
                _ => false,
            }
        }
        Err(_) => false,
    }
}
