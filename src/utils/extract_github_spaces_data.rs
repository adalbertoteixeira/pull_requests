use crate::storage::{GithubSpace, save_github_config};
use indicatif::ProgressBar;
use log::{debug, info};
use reqwest::Client;
use serde_json::json;
use std::{
    io::{self, Write},
    process,
    time::Duration,
};

#[derive(Debug, Clone)]
pub struct GithubIssue {
    pub id: i64,
    pub title: String,
    pub body: String,
    pub repository: String,
}

pub struct GithubSpaceData {
    pub spaces: Vec<GithubSpace>,
}

pub async fn make_github_request(
    client: &Client,
    url: &str,
    github_api_token: &str,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let mut authorization: String = "Bearer ".to_owned();
    authorization.push_str(github_api_token);

    let bar = ProgressBar::new_spinner();
    bar.enable_steady_tick(Duration::from_millis(100));
    let res = client
        .get(url)
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .header("Authorization", authorization)
        .header("User-Agent", "pull_requests")
        .send()
        .await?;

    bar.finish();
    let status = res.status();
    info!("Status: {}", status);

    if !status.is_success() {
        let error_body = res.text().await?;
        return Err(format!("API request failed with status {}: {}", status, error_body).into());
    }

    let body: serde_json::Value = res.json().await?;
    Ok(body)
}

pub async fn extract_github_spaces_data(
    directory: &str,
    client: &Client,
    github_api_token: &str,
) -> Result<Option<Vec<GithubSpace>>, String> {
    let stdout = io::stdout(); // get the global stdout entity
    let mut handle = io::BufWriter::new(&stdout); // optional: wrap that handle in a buffer
    let url: String = "https://api.github.com/user/repos".to_owned();

    let spaces = match make_github_request(&client, &url, github_api_token).await {
        Ok(b) => b,
        Err(e) => {
            writeln!(handle, "Error making API request: {}", e).unwrap_or_default();
            let _ = handle.flush();
            process::exit(1);
        }
    };
    let mut github_spaces: Vec<GithubSpace> = vec![];
    for space in spaces.as_array().ok_or("Spaces is not an array")? {
        debug!("{:?}", space);
        let github_space = GithubSpace {
            id: space
                .get("id")
                .ok_or("Space missing id")?
                .as_i64()
                .ok_or("Space id is not an integer")?,
            name: space
                .get("name")
                .ok_or("Space missing name")?
                .as_str()
                .ok_or("Space name is not a string")?
                .to_string(),
            full_name: space
                .get("full_name")
                .ok_or("Space missing full name")?
                .as_str()
                .ok_or("Space full name is not a string")?
                .to_string(),
            description: space
                .get("description")
                .unwrap_or(&json!(""))
                .as_str()
                .unwrap_or("")
                .to_string(),
            url: space
                .get("url")
                .ok_or("Space missing url")?
                .as_str()
                .ok_or("Space url is not a string")?
                .to_string(),
        };

        github_spaces.push(github_space);
    }
    info!("github spaces {:?}", github_spaces);

    let github_spaces_clone = github_spaces.to_vec();
    let _ = save_github_config(&directory, Some(github_spaces_clone));
    Ok(Some(github_spaces))
}

pub async fn get_github_user_issues(
    client: &Client,
    github_api_token: &str,
) -> Result<Vec<GithubIssue>, Box<dyn std::error::Error>> {
    let url = "https://api.github.com/issues?pulls=false";

    let body = make_github_request(client, url, github_api_token).await?;

    let mut github_issues: Vec<GithubIssue> = vec![];

    if let Some(issues_array) = body.as_array() {
        for issue in issues_array {
            let github_issue = GithubIssue {
                id: issue
                    .get("id")
                    .ok_or("Issue missing id")?
                    .as_i64()
                    .ok_or("Issue id is not an integer")?,
                title: issue
                    .get("title")
                    .ok_or("Issue missing title")?
                    .as_str()
                    .ok_or("Issue title is not a string")?
                    .to_string(),
                body: issue
                    .get("body")
                    .unwrap_or(&serde_json::json!(""))
                    .as_str()
                    .unwrap_or("")
                    .to_string(),
                repository: issue
                    .get("repository")
                    .unwrap_or(&json!(r#"{name: ""}"#))
                    .get("name")
                    .unwrap_or(&json!(""))
                    .as_str()
                    .unwrap_or("")
                    .to_string(),
            };

            github_issues.push(github_issue);
        }
    } else {
        return Err("Response is not an array".into());
    }

    Ok(github_issues)
}
