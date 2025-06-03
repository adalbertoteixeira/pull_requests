use std::{
    collections::HashMap,
    io::{self, Write},
    process::{self, Command},
};

use crate::branch_utils;
use inquire::{formatter::OptionFormatter, validator::Validation, Confirm, Editor, Select, Text};
use log::info;

pub fn editor_prompt() {
    let stdout = io::stdout(); // get the global stdout entity
    let mut handle = io::BufWriter::new(&stdout);

    let editor_echo: Option<String> =
        match Command::new("sh").arg("-c").arg("echo $EDITOR").output() {
            Ok(x) => {
                if x.status.success() {
                    Some(String::from_utf8_lossy(&x.stdout).trim().to_string())
                } else {
                    None
                }
            }
            Err(_) => None,
        };
    info!("Editor echo: {:?}", editor_echo);
    let mut output_text = "".to_owned();
    let mut editor_echo_str = "".to_owned();
    output_text.push_str(
        "\n\x1b[1;33mIn order to use multi-line text, we need the $EDITOR in the $PATH.\x1b[1;0m\n\n",
    );
    output_text.push_str("If you want to use VSCode, do the following:\n1. Open \x1b[1;32mVSCode\x1b[1;0m;\n2. Navigate to the Command Pallete and select \x1b[1;32m`Shell Command: Install ‘code’ command in PATH`\x1b[1;0m\n3. Add \x1b[1;32m`code`\x1b[1;0m as an editor: in your shell profile (tipically `~/.bashrc` , `~/.bash_profile` or ` ~/.zshrc`), add \x1b[1;32m`export EDITOR=\"code\"`\x1b[1;0m\n");

    writeln!(handle, "{}", output_text).unwrap_or_default();
    let _ = handle.flush();
    if editor_echo.is_some() {
        editor_echo_str = editor_echo.unwrap().to_owned();
    }

    let mut selection_text = "".to_owned();
    if editor_echo_str.is_empty() {
        selection_text.push_str("\x1b[1;31mNo editor found in the $PATH.\x1b[1;0m\n\n");
        selection_text.push_str("Please configure your editor to use multi-line text. You can do this by following the instructions above and rerun the tool afterwards.");

        writeln!(handle, "{}", selection_text).unwrap_or_default();
        let _ = handle.flush();
        process::exit(0);
    } else {
        selection_text.push_str("Found an editor in the $PATH: ");
        selection_text.push_str(&editor_echo_str);

        writeln!(handle, "{}", selection_text).unwrap_or_default();
        let _ = handle.flush();
        let accept_editor_prompt = Confirm::new("Do you want to use this editor?")
            .with_default(true)
            .prompt();

        let answer = match accept_editor_prompt {
            Ok(response) => response,
            Err(_) => process::exit(1),
        };

        if answer == false {
            let suggestion_text= "Please configure your editor to use multi-line text. You can do this by following the instructions above and rerun the tool afterwards.";
            writeln!(handle, "{}", suggestion_text).unwrap_or_default();
            let _ = handle.flush();
            process::exit(0);
        }
    }
}

pub fn issue_id_prompt(issue_id: &str) -> String {
    let selected_issue_id_prompt = Text::new("Select issue ID")
        .with_default(&issue_id)
        .with_validator(|input: &str| {
            let length = input.chars().count();
            if length == 0 {
                Ok(Validation::Invalid(format!("An id is required").into()))
            } else {
                Ok(Validation::Valid)
            }
        })
        .prompt();

    let selected_issue_id = match selected_issue_id_prompt {
        Ok(issue_id) => issue_id,
        Err(_) => {
            println!("An error happened when selecting the issue id, try again.");
            std::process::exit(1);
        }
    };
    return selected_issue_id;
}

pub fn team_prefix_prompt(team_prefix: &str) -> String {
    let selected_team_prefix_prompt = Text::new("Select team prefix")
        .with_default(&team_prefix)
        .prompt();

    let selected_team_prefix = match selected_team_prefix_prompt {
        Ok(team) => team,
        Err(_) => {
            println!("An error happened when selecting the team, try again.");
            std::process::exit(1);
        }
    };
    return selected_team_prefix;
}

pub fn select_types_prompt(proposed_type: &Option<String>) -> String {
    let type_options: Vec<&str> = vec![
        "feat: A new feature",
        "fix: Bug (feature related) or code (linting, typecheck, etc) fixes",
        "test: Adding missing tests or correcting existing tests",
        "refactor: A code change that improves performance or code quality",
        "docs: Documentation only changes",
        "build: Changes that affect the build system or external dependencies (example scopes: gulp, broccoli, npm)",
        "ci: Changes to our CI configuration files and scripts (example scopes: Travis, Circle, BrowserStack, SauceLabs)",
        "revert: Reverts a previous commit",
    ];

    fn get_short_type(type_str: &str) -> String {
        let parts = type_str.split(": ").collect::<Vec<&str>>();
        let type_short = match parts.get(0) {
            Some(x) => x,
            None => "Unknown",
        };
        return type_short.to_string();
    }

    let type_formatter: OptionFormatter<&str> = &|i| {
        return get_short_type(i.value);
    };

    let mut starting_cursor = 0;

    if proposed_type.is_some() {
        let index = type_options
            .iter()
            .position(|&x| x.starts_with(&proposed_type.clone().unwrap()));
        info!(
            "proposed_type starting cursor {:?}, {:?}, {:?}",
            starting_cursor, proposed_type, index
        );
        if index.is_some() {
            starting_cursor = index.unwrap();
        }
    }

    let selected_types_propmpt = Select::new("Select change type", type_options)
        .with_formatter(type_formatter)
        .with_starting_cursor(starting_cursor)
        .prompt();

    let selected_type = match selected_types_propmpt {
        Ok(type_str) => get_short_type(type_str),
        Err(_) => {
            println!("An error happened when selecting the team, try again.");
            std::process::exit(1);
        }
    };
    return selected_type;
}

pub fn select_additional_message_prompt() -> Option<String> {
    let should_add_additional_message = Confirm::new("Do you want to add an additional message?")
        .with_default(false)
        .prompt();
    let should_add_additional_message_answer = match should_add_additional_message {
        Ok(should_add) => should_add,
        Err(_) => process::exit(1),
    };

    if should_add_additional_message_answer == false {
        return None;
    }
    let additional_message_prompt = Editor::new("Enter additional message").prompt();
    let message_to_use = match additional_message_prompt {
        Ok(additional_message) => {
            info!("Additional message: {}", additional_message);
            additional_message
        }
        Err(_) => process::exit(1),
    };

    return Some(message_to_use);
}

pub fn select_message_prompt(git_branch: &str) -> String {
    let default_message_name = branch_utils::branch_name(&git_branch);

    let message_prompt = Text::new("Enter commit message")
        .with_default(&default_message_name)
        .with_validator(|input: &str| {
            let length = input.chars().count();
            if length > 55 {
                Ok(Validation::Invalid(
                    format!(
                        "Commit message limit is 55 characters. You have {}.",
                        length
                    )
                    .into(),
                ))
            } else {
                Ok(Validation::Valid)
            }
        })
        .prompt();

    let message = match message_prompt {
        Ok(message) => message,
        Err(_) => {
            println!("An error happened when selecting the commit message, try again.");
            std::process::exit(1);
        }
    };
    return message;
}

pub fn commit_pr_prompt() -> bool {
    let commit_pr_prompt_answer = Confirm::new("Commit this PR?").with_default(true).prompt();

    let answer = match commit_pr_prompt_answer {
        Ok(response) => response,
        Err(_) => process::exit(1),
    };

    return answer;
}

pub fn push_pr_prompt() -> bool {
    let push_pr_prompt_answer = Confirm::new("Push this branch?")
        .with_default(false)
        .prompt();

    let answer = match push_pr_prompt_answer {
        Ok(response) => response,
        Err(_) => process::exit(1),
    };

    return answer;
}

pub fn pr_template_prompt(issue_id: &str) -> String {
    let mut pr_template = "".to_owned();
    let mut has_description = false;

    let pr_description_prompt =
        Editor::new("Write a description for your PR and explain why it's important").prompt();
    let pr_description: Option<String> = match pr_description_prompt {
        Ok(x) => Some(x),
        Err(_) => None,
    };
    if pr_description.is_some() {
        pr_template += &"\n";
        pr_template += &pr_description.unwrap();
        pr_template += &"\n";
        has_description = true;
    } else {
        pr_template += &"Because..."
    }

    let risk_options: Vec<&str> = vec!["High", "Medium", "Low", "Trivial"];
    let risk_factor_prompt = Select::new("Select risk factor", risk_options).prompt();

    let risk_factor_map = HashMap::from([
        ("High", "🚨HIGH🚨"),
        ("Medium", "⚠️MEDIUM⚠️"),
        ("Low", "👍LOW👍"),
        ("Trivial", "✅TRIVIAL✅"),
    ]);
    let selected_risk_factor = match risk_factor_prompt {
        Ok(risk_factor) => {
            if risk_factor_map.contains_key(&risk_factor) {
                Some(risk_factor_map.get(&risk_factor).unwrap().to_string())
            } else {
                None
            }
        }
        _ => None,
    };

    if selected_risk_factor.is_some() {
        pr_template += &format!(
            "\n# 🚦 This is a {} risk PR\n",
            selected_risk_factor.unwrap()
        );
    }
    //
    let risk_factor_description_prompt = Editor::new("Describe why this risk factor was selected")
        .with_help_message("Describe why this risk factor was selected..")
        .prompt();
    let risk_factor_description: Option<String> = match risk_factor_description_prompt {
        Ok(x) => Some(x),
        Err(_) => None,
    };
    if risk_factor_description.is_some() {
        pr_template += &"Because...";
        pr_template += &"\n";
        pr_template += &risk_factor_description.unwrap();
        pr_template += &"\n";
    } else {
        pr_template += &"Because...";
        pr_template += &"\n";
    }

    pr_template += &"\n## 🧪 How to manually test this PR";
    let manual_testing_description_prompt =
        Editor::new("Describe how to manually test this PR").with_help_message("Create a simple, bullet pointed list, step by step on how to test. Make sure you call out the need for any extra config/services. Make it EASY for the person reviewing your PR").prompt();
    let manual_testing_description: Option<String> = match manual_testing_description_prompt {
        Ok(x) => Some(x),
        Err(_) => None,
    };
    if manual_testing_description.is_some() {
        pr_template += &"\n";
        pr_template += &manual_testing_description.unwrap();
        pr_template += &"\n";
    } else {
        pr_template += &"1.";
        pr_template += &"\n";
    }

    pr_template += "\n## Good PR check list\n";

    let has_description_x = match has_description {
        true => "x",
        false => " ",
    };
    pr_template += &format!(
        "- [{}] ✍️ I wrote an easy-to-read, short description at the top, with a good title\n",
        has_description_x
    )
    .to_string();
    pr_template += &format!(
        "- [x] 🔗 I linked this PR to an issue (which is in progress): fixes #{}",
        issue_id
    )
    .to_string();
    pr_template += &"
- [ ] 📋 I filled out the risk level, how to test, impact, what the PR does
- [ ] 🏷️ I added the right labels. [api? BENApp? someOtherApp?]
- [ ] 🥸 I assigned myself to the PR and others (as needed)
- [ ] 🚀 I moved the PR into ready state - it's ready to be reviewed!
- [ ] 🤖 I enabled auto merge"
        .to_string();
    info!("PR template:\n{}", pr_template);
    return pr_template.to_owned();
}
