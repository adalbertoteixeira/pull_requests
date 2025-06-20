use indicatif::ProgressBar;
use log::info;
use serde_json::Value;
use std::{
    fs,
    io::{self, Write},
    process::{self, Command, Stdio},
    time::Duration,
};

pub fn parse_claude_response(stdout: &str) -> Result<Value, io::Error> {
    let result: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let result_json = result.get("result").unwrap();

    let result_json_str = result_json.as_str().unwrap();
    let mut start_bytes = result_json_str.find("```json\n").unwrap();
    start_bytes += 7;
    let end_bytes = result_json_str.rfind("```").unwrap();

    let result_sjon = &result_json_str[start_bytes..end_bytes];
    let result_sjon_replace = result_sjon.replace("\n", "");
    let final_json: serde_json::Value = serde_json::from_str(&result_sjon_replace).unwrap();
    Ok(final_json)
}

pub fn prompt_claude_one_off(
    prompt_header: &str,
    prompt_text: &str,
    directory: &str,
    mcp_config: Option<&str>,
) -> Result<String, io::Error> {
    let stdout = io::stdout();
    let mut handle = io::BufWriter::new(&stdout);
    let bar = ProgressBar::new_spinner();
    bar.enable_steady_tick(Duration::from_millis(100));

    let initial_prompt = format!(r#"{}"#, prompt_text).replace("'", "\'");

    // Save prompt to system tmp file
    let tmp_file_path = format!("/tmp/claude_prompt_{}.txt", std::process::id());
    fs::write(&tmp_file_path, &initial_prompt).expect("Failed to write prompt to tmp file");

    let mut cmd_arg = format!(
        r#"cd {} &&  cat {} | claude --model sonnet --output-format json "#,
        &directory, tmp_file_path,
    );

    if mcp_config.is_some() {
        cmd_arg.push_str(&format!(r#"--mcp-config={}"#, mcp_config.unwrap()));
    }

    cmd_arg.push_str(&format!(r#" -p {}"#, prompt_header));

    let output = Command::new("sh")
        .arg("-c")
        .arg(cmd_arg)
        .output()
        .expect("Failed to run  process");

    bar.finish();
    if !output.status.success() {
        let error = str::from_utf8(&output.stderr).unwrap_or_default();
        let message = str::from_utf8(&output.stdout).unwrap_or_default();
        writeln!(
            handle,
            "There was an issue: \x1b[1;31m{} {}\x1b[1;0m",
            error, message
        )
        .unwrap_or_default();
        let _ = handle.flush();
        process::exit(1)
    }

    let result_stdout_string = str::from_utf8(&output.stdout).unwrap();

    let result_json: serde_json::Value = serde_json::from_str(&result_stdout_string).unwrap();
    let result = result_json.get("result").unwrap().as_str().unwrap();

    info!("done");
    Ok(result.to_string())
}

pub fn prompt_claude(prompt_text: &str, directory: &str, mcp_config: Option<&str>) {
    let stdout = io::stdout();
    let mut handle = io::BufWriter::new(&stdout);

    let mut extra_args = "".to_owned();
    if mcp_config.is_some() {
        extra_args.push_str(&format!("--mcp-config={}", mcp_config.unwrap()));
    }
    // Spawn an interactive shell
    let mut child = Command::new("claude")
        .arg(extra_args)
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
