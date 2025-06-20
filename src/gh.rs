use std::process::Command;

pub fn validate_gh() -> bool {
    let output = Command::new("gh")
        .arg("auth")
        .arg("status")
        .output();

    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            stdout.contains("Logged in to github.com account")
        }
        Err(_) => false,
    }
}