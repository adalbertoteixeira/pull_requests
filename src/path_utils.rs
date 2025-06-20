use log::{debug, error, info};
use std::process;
use std::process::Command;
use std::str;

pub fn top_level(directory: &str) {
    let cmd_arg = format!("cd {directory} && git rev-parse --show-toplevel");
    let repo_root_output = Command::new("sh").arg("-c").arg(cmd_arg).output().unwrap();
    debug!("Repository root is {:?}", repo_root_output.status);
    if !repo_root_output.status.success() {
        let error = str::from_utf8(&repo_root_output.stderr).unwrap();
        error!("{:?}", error);
        debug!("Path is not valid");
        process::exit(1)
    }
}

// Get current branch
pub fn git_branch(directory: &str) -> String {
    let cmd_arg = format!("cd {directory} && git rev-parse --abbrev-ref HEAD");
    let output = Command::new("sh").arg("-c").arg(cmd_arg).output().unwrap();
    if !output.status.success() {
        let error = str::from_utf8(&output.stderr).unwrap();
        error!("{:?}", error);
        debug!("Couldn't get the branch.");
        process::exit(1)
    }
    let current_branch = str::from_utf8(&output.stdout).unwrap().trim();
    info!("Current branch is: {}", current_branch);
    return current_branch.to_owned();
}
