use chrono::prelude::*;
use homedir::my_home;
use std::{
    fs::{self, File},
    io::{self, Write},
    path::{Path, PathBuf},
    process::{self},
};

use log::info;
use serde::{Deserialize, Serialize};

use crate::{prompts, ux_utils};

#[derive(Serialize, Deserialize, Debug)]
struct ClickupYamlConfig {
    clickup_spaces: Option<Vec<ClickupSpace>>,
    created_at: String,
    updated_at: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ClickupSpace {
    pub id: String,
    pub name: String,
    pub priorities: Option<Vec<ClickupPriority>>,
    pub members: Option<Vec<ClickupMember>>,
    pub statuses: Option<Vec<ClickupStatus>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ClickupPriority {
    pub id: String,
    pub priority: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ClickupStatus {
    pub id: String,
    pub status: String,
    pub status_type: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ClickupMember {
    pub id: i64,
    pub username: String,
    pub initials: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct BranchYamlConfig {
    branch_name: String,
    created_at: String,
    updated_at: Option<String>,
    pr_template: Option<String>,
    commit_message: Option<String>,
    additional_message: Option<Vec<String>>,
    last_commit_exit_code: Option<i32>,
}

#[derive(Serialize, Deserialize, Debug)]
struct ToolYamlConfig {
    editor: Option<String>,
}

pub fn setup_commit_tool(config_directory_matches: &str, config_editor_matches: &str) {
    let user_home_dir = if config_directory_matches.len() > 0 {
        PathBuf::from(config_directory_matches)
    } else {
        my_home().unwrap().expect("User's $HOME should be set")
    };

    if !user_home_dir.is_dir() {
        panic!(
            "User home directory not found. Please set the environment variable HOME or run the command in a directory with a HOME environment variable set."
        );
    }
    info!("User home directory: {:?}", user_home_dir);
    let config_dir_path = Path::new(&user_home_dir).join(".config");
    if !config_dir_path.exists() {
        fs::create_dir_all(&config_dir_path).unwrap();
    }
    let config_path = Path::new(&user_home_dir)
        .join(".config")
        .join("commit_tool.yaml");

    info!("Config path is: {:?}", config_path);
    // let mut editor = String::new();
    if config_path.exists() {
        info!("Config path exists");
        if config_editor_matches.len() > 0 {
            return;
        }
        // let file = fs::File::open(&config_path).expect("Failed to open file");
        // let reader = io::BufReader::new(file);
        // let file_read: ToolYamlConfig =
        //     serde_yml::from_reader(reader).expect("Failed to parse YAML");
        // editor = file_read.editor.clone().unwrap();
    } else {
        info!("Config path does not exist");
        let _ = prompts::editor_prompt();
        let mut yaml_tool_config = ToolYamlConfig { editor: None };
        yaml_tool_config.editor = Some("".to_owned());
        let file = File::create(&config_path).expect("Failed to create file");
        serde_yml::to_writer(file, &yaml_tool_config).expect("Failed to write YAML");
    }
    // match Command::new("sh")
    //     .arg("-c")
    //     .arg(format!("export EDITOR={}", editor))
    //     .output()
    // {
    //     Ok(_) => info!("Set editor"),
    //     Err(_) => info!("Could not set Editor"),
    // }
    // return editor;
}

pub fn setup_branch_env(git_branch: &str, directory: &str) -> Result<bool, io::Error> {
    let mut is_new_branch = true;
    let path = Path::new(directory).join(".commit_message");
    info!("path: {:?}", path);
    if !path.exists() {
        fs::create_dir_all(&path).unwrap();
    }
    let file_path = Path::new(&path).join(format!("{}.yaml", &git_branch));
    info!("file_path: {:?}", file_path);
    if file_path.exists() {
        is_new_branch = false;
    };

    Ok(is_new_branch)
}
pub fn load_commit_template(git_branch: &str, directory: &str) {
    let stdout = io::stdout(); // get the global stdout entity
    let mut handle = io::BufWriter::new(&stdout); // optional: wrap that handle in a buffer
    let path = Path::new(directory).join(".commit_message");
    let file_path = Path::new(&path).join(format!("{}.yaml", &git_branch));
    match fs::File::open(&file_path) {
        Ok(file) => {
            let reader = io::BufReader::new(file);
            let file_read: BranchYamlConfig =
                serde_yml::from_reader(reader).expect("Failed to parse YAML");
            if file_read.pr_template.is_none() {
                writeln!(handle, "{}", "No saved commit template found").unwrap_or_default();
                let _ = handle.flush();
                process::exit(0);
            }

            writeln!(handle, "{}", file_read.pr_template.unwrap()).unwrap_or_default();
            let _ = handle.flush();
            process::exit(0);
        }
        Err(_) => {
            writeln!(handle, "{}", "No previous file found.").unwrap_or_default();
            let _ = handle.flush();
            process::exit(0)
        }
    }
}

pub fn load_branch_config(git_branch: &str, directory: &str, no_verify: bool, cowboy_mode: bool) {
    let stdout = io::stdout(); // get the global stdout entity
    let mut handle = io::BufWriter::new(&stdout); // optional: wrap that handle in a buffer
    let path = Path::new(directory).join(".commit_message");
    let file_path = Path::new(&path).join(format!("{}.yaml", &git_branch));
    let file = fs::File::open(&file_path).expect("Failed to open file");
    let reader = io::BufReader::new(file);
    let file_read: BranchYamlConfig = serde_yml::from_reader(reader).expect("Failed to parse YAML");
    if file_read.last_commit_exit_code.is_some_and(|i| i != 0) {
        let previous_commit_message = file_read.commit_message.as_ref().unwrap().to_owned();
        let previous_commit_message_additional_messages =
            file_read.additional_message.as_ref().unwrap().to_owned();
        let previous_pr_template = file_read
            .pr_template
            .as_ref()
            .unwrap_or(&"".to_string())
            .to_owned();
        let mut proposed_ouput_message = "\x1b[1;33mFound a failed commit:\n".to_owned();
        proposed_ouput_message.push_str("\n");
        proposed_ouput_message.push_str(&previous_commit_message);
        for addition_message in &previous_commit_message_additional_messages {
            proposed_ouput_message.push_str("\n");
            proposed_ouput_message.push_str(&addition_message);
        }
        proposed_ouput_message.push_str("\n");
        writeln!(handle, "{}", proposed_ouput_message).unwrap_or_default();
        let _ = handle.flush();
        let will_commit_pr = prompts::commit_pr_prompt();
        if will_commit_pr == true {
            ux_utils::commit_and_push(
                directory,
                previous_commit_message,
                previous_commit_message_additional_messages,
                git_branch,
                Some(previous_pr_template),
                no_verify,
                cowboy_mode,
            );
            process::exit(0);
        }
    }
}

pub fn save_branch_config(
    git_branch: &str,
    directory: &str,
    pr_template: Option<String>,
    commit_message: Option<String>,
    additional_message: Option<Vec<String>>,
    commit_exit_code: Option<i32>,
) -> Result<(), io::Error> {
    info!(
        "Saving branch config for branch: {} in {}",
        &git_branch, &directory
    );
    let path = Path::new(directory).join(".commit_message");
    if !path.exists() {
        info!("Creating directory: {:?}", &path);
        fs::create_dir_all(&path).unwrap();
    }

    let file_path = Path::new(&path).join(format!("{}.yaml", &git_branch));

    let local_time: DateTime<Local> = Local::now();
    let local_time_string = local_time.format("%Y-%m-%d %H:%M:%S").to_string();
    if file_path.exists() {
        let file = fs::File::open(&file_path).expect("Failed to open file");
        let reader = io::BufReader::new(file);
        let mut file_read: BranchYamlConfig =
            serde_yml::from_reader(reader).expect("Failed to parse YAML");
        info!("File: {:?}", file_read);
        file_read.updated_at = Some(local_time_string);
        if pr_template.is_some() {
            file_read.pr_template = pr_template;
        }
        if commit_message.is_some() {
            file_read.commit_message = commit_message;
        }
        if additional_message.is_some() {
            file_read.additional_message = additional_message;
        }

        if commit_exit_code.is_some() {
            file_read.last_commit_exit_code = commit_exit_code;
        }

        let file = File::create(&file_path).expect("Failed to create file");
        serde_yml::to_writer(file, &file_read).expect("Failed to write YAML");
    } else {
        // Create new YAML config
        let mut yaml_config = BranchYamlConfig {
            branch_name: git_branch.to_string(),
            created_at: local_time_string.clone(),
            updated_at: Some(local_time_string),
            pr_template: None,
            commit_message: None,
            additional_message: None,
            last_commit_exit_code: Some(1),
        };
        if pr_template.is_some() {
            yaml_config.pr_template = pr_template;
        }
        if commit_message.is_some() {
            yaml_config.commit_message = commit_message;
        }
        if additional_message.is_some() {
            yaml_config.additional_message = additional_message;
        }
        if commit_exit_code.is_some() {
            yaml_config.last_commit_exit_code = commit_exit_code;
        }
        let file = File::create(&file_path).expect("Failed to create file");
        serde_yml::to_writer(file, &yaml_config).expect("Failed to write YAML");
    };
    Ok(())
}

pub fn load_clickup_config(directory: &str) -> Result<Option<Vec<ClickupSpace>>, io::Error> {
    let path = Path::new(directory).join(".commit_message");
    let file_path = Path::new(&path).join("clickup.yaml");

    if !file_path.exists() {
        info!("No clickup config file found at: {:?}", file_path);
        return Ok(None);
    }

    match fs::File::open(&file_path) {
        Ok(file) => {
            let reader = io::BufReader::new(file);
            match serde_yml::from_reader::<_, ClickupYamlConfig>(reader) {
                Ok(file_read) => {
                    info!("Successfully loaded clickup config: {:?}", file_read);
                    Ok(file_read.clickup_spaces)
                }
                Err(e) => {
                    info!("Failed to parse clickup YAML: {}", e);
                    Err(io::Error::new(io::ErrorKind::InvalidData, e))
                }
            }
        }
        Err(e) => {
            info!("Failed to open clickup config file: {}", e);
            Err(e)
        }
    }
}

pub fn save_clickup_config(
    directory: &str,
    clickup_spaces: Option<Vec<ClickupSpace>>,
) -> Result<(), io::Error> {
    info!(
        "Saving clickup spaces for clickup: {:?} in {}",
        &clickup_spaces, &directory
    );
    let path = Path::new(directory).join(".commit_message");
    if !path.exists() {
        info!("Creating directory: {:?}", &path);
        fs::create_dir_all(&path).unwrap();
    }

    let file_path = Path::new(&path).join("clickup.yaml");

    let mut yaml_config;
    let local_time: DateTime<Local> = Local::now();
    let local_time_string = local_time.format("%Y-%m-%d %H:%M:%S").to_string();
    if file_path.exists() {
        let file = fs::File::open(&file_path).expect("Failed to open file");
        let reader = io::BufReader::new(file);
        yaml_config = serde_yml::from_reader(reader).expect("Failed to parse YAML");
        info!("File: {:?}", yaml_config);
    } else {
        yaml_config = ClickupYamlConfig {
            clickup_spaces: None, // clickup_name: git_clickup.to_string(),
            created_at: local_time_string.clone(),
            updated_at: None,
        };
    };

    if clickup_spaces.is_some() {
        yaml_config.clickup_spaces = Some(clickup_spaces.unwrap());
    }

    yaml_config.updated_at = Some(local_time_string);
    let file = File::create(&file_path).expect("Failed to create file");
    serde_yml::to_writer(file, &yaml_config).expect("Failed to write YAML");
    Ok(())
}
