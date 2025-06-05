use std::{
    io::{self, Write},
    process,
};

use log::{debug, info};

use crate::{branch_utils, prompts};

pub fn commit_and_push(
    directory: &str,
    commit_message: String,
    commit_message_additional_messages: Vec<String>,
    git_branch: &str,
    pr_template: Option<String>,
    no_verify: bool,
    cowboy_mode: bool,
) {
    let stdout = io::stdout(); // get the global stdout entity
    let mut handle = io::BufWriter::new(&stdout); // optional: wrap that handle in a buffer
    info!("Will commit pr");
    let commit_pr_exit_code = branch_utils::commit_pr(
        directory,
        &commit_message,
        commit_message_additional_messages.clone(),
        &git_branch,
        &pr_template,
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
        let mut pr_template_message = "\n\x1b[1;32mYour PR template is below. You can copy it and add it to Github PR descripton:\x1b[1;0m\n\n".to_owned();
        pr_template_message.push_str(&pr_template.unwrap());
        writeln!(handle, "{}", pr_template_message).unwrap_or_default();
    }
    let will_push_pr;
    if cowboy_mode == true {
        will_push_pr = true;
    } else {
        will_push_pr = prompts::push_pr_prompt();
    }
    if will_push_pr == true {
        let _ = branch_utils::push_pr(directory, no_verify);
    }
}
