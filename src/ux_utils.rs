use std::{
    io::{self, Write},
    process,
};

use log::{debug, info};

use crate::{branch_utils, prompts};

pub async fn commit_and_push(
    directory: &str,
    commit_message: String,
    commit_message_additional_messages: Vec<String>,
    git_branch: &str,
    pr_template: Option<String>,
    no_verify: bool,
    ci_mode: bool,
    github_api_token: Option<&str>,
    has_gh: bool,
) {
    let stdout = io::stdout(); // get the global stdout entity
    let mut handle = io::BufWriter::new(&stdout); // optional: wrap that handle in a buffer
    info!("Will commit pr");
    let commit_pr_exit_code = branch_utils::commit_pr(
        directory,
        &commit_message,
        commit_message_additional_messages.clone(),
        &git_branch,
        pr_template.clone(),
        no_verify,
    );
    info!("Will commit pr exit code");
    let commit_fail_message =
        "\n\x1b[1;31mCommit failed. Please fix the issue before commiting.\x1b[1;0m\n\n".to_owned();
    if commit_pr_exit_code.is_err() {
        debug!("Commit PR exit code is err");
        writeln!(handle, "{}", &commit_fail_message).unwrap_or_default();
        let _ = handle.flush();
        process::exit(1);
    }
    debug!("commit_pr_exit_code {:?}", commit_pr_exit_code);
    let commit_pr_exit_code_result = &commit_pr_exit_code.unwrap();
    debug!("commit_pr_exit_code {:?}", commit_pr_exit_code_result);
    if commit_pr_exit_code_result.is_none() || commit_pr_exit_code_result.is_some_and(|x| x != 0) {
        writeln!(handle, "{}", commit_fail_message).unwrap_or_default();
        let _ = handle.flush();
        process::exit(1);
    }
    if pr_template.is_some() {
        let pr_template_message =
            "\x1b[1;32mThere is a PR template available.\x1b[1;0m Use `commit --show-pr-template` to display it.".to_owned();
        writeln!(handle, "{}", pr_template_message).unwrap_or_default();
        let _ = handle.flush();
    }
    let will_push_pr = match ci_mode {
        true => true,
        false => prompts::push_pr_prompt(),
    };
    info!("Will push pr? {}", will_push_pr);
    if will_push_pr == true {
        info!("Will push pr? {}", will_push_pr);
        let _ = branch_utils::push_pr(
            directory,
            no_verify,
            ci_mode,
            github_api_token,
            git_branch,
            Some(&commit_message),
            pr_template,
            has_gh,
        )
        .await
        .unwrap();
    }
}
