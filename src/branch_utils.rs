use indicatif::ProgressBar;
use inquire::Confirm;
use log::{debug, error, info, log};
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

lazy_static! {
    static ref ISSUE_REGEX: Regex = Regex::new(r"^(\w*)(?:-)?(.*)?$").unwrap();
    static ref TEST_REGEX: Regex =
        Regex::new(r"\.spec\.|\.test\.|\.jest\.|\.config\.|jest\.unit").unwrap();
    static ref DOCS_REGEX: Regex = Regex::new(r"\.md$").unwrap();
    static ref BUILD_REGEX: Regex = Regex::new(r"package.json|yarn.lock$").unwrap();
    static ref CI_REGEX: Regex = Regex::new(r"^\.|Dockerfile|/iac/").unwrap();
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
        println!("Couldn't find changed files.");
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
    pr_template: &Option<String>,
) -> Result<Option<i32>, io::Error> {
    let mut cmd_arg = format!(
        r#"cd {} && git commit -m  "{}""#,
        &directory, &commit_message
    );
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
    );
    debug!(
        "Save config result is {:?}",
        branch_config_save_result.is_ok()
    );
    Ok(output.status.code())
}

pub fn push_pr(directory: &str) -> Option<i32> {
    let cmd_arg = format!(r#"cd {} && git push"#, &directory);
    info!("Executing command: {}", cmd_arg);
    let stdout = io::stdout(); // get the global stdout entity
    let mut handle = io::BufWriter::new(&stdout); // optional: wrap that handle in a buffer
    writeln!(
        handle,
        "{}",
        "Pushing branch. This might take some time depending on the pre-commit hooks."
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

    let cmd_arg_status_code = output.status.code();
    if cmd_arg_status_code.is_none() || cmd_arg_status_code.is_some_and(|x| x != 0) {
        print!("OUTPUT: {:?}", cmd_arg_status_code);
        print!("OUTPUT: {:?}", output);
        let stderr = str::from_utf8(&output.stderr).unwrap();
        print!("OUTPUT: {:?}", stderr);
    }
    bar.finish();
    io::stderr().write_all(&output.stderr).unwrap();
    io::stdout().write_all(&output.stdout).unwrap();
    cmd_arg_status_code
}
