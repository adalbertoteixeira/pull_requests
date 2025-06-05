use crate::storage::{
    ClickupMember, ClickupPriority, ClickupSpace, ClickupStatus, ClickupTask, ClickupYamlConfig,
    GithubSpace, save_clickup_config, save_github_config,
};
use clap::ArgMatches;
use log::{debug, info};
use reqwest::Client;
use serde_json::json;
use std::{
    io::{self, Write},
    process,
};

pub struct GithubSpaceData {
    pub spaces: Vec<GithubSpace>,
}
pub struct ClickupSpacesData {
    pub spaces: Vec<ClickupSpace>,
}

pub async fn make_clickup_request(
    client: &Client,
    url: &str,
    api_key: &str,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let res = client
        .get(url)
        .header("Accept", "application/json")
        .header("Authorization", api_key)
        .send()
        .await?;

    let status = res.status();
    info!("Status: {}", status);

    if !status.is_success() {
        let error_body = res.text().await?;
        return Err(format!("API request failed with status {}: {}", status, error_body).into());
    }

    let body: serde_json::Value = res.json().await?;
    debug!("Github body {:?}", body);
    Ok(body)
}

pub async fn extract_clickup_spaces_data(
    directory: &str,
    matches: &ArgMatches<'static>,
    client: &Client,
) -> Result<Option<ClickupYamlConfig>, String> {
    debug!("Calling subcommnand workspaces function {:?}", matches);
    let stdout = io::stdout(); // get the global stdout entity
    let mut handle = io::BufWriter::new(&stdout); // optional: wrap that handle in a buffer
    let mut url: String = "https://api.clickup.com/api/v2/team/".to_owned();
    url.push_str(&matches.value_of("clickup_workspace_id").unwrap());
    url.push_str("/space");

    let mut authorization: String = "Bearer ".to_owned();
    debug!("{:?}", matches.value_of("clickup_api_key").unwrap());
    authorization.push_str(matches.value_of("clickup_api_key").unwrap());
    debug!("authorization: {:?}", authorization);
    let body =
        match make_clickup_request(&client, &url, matches.value_of("clickup_api_key").unwrap())
            .await
        {
            Ok(b) => b,
            Err(e) => {
                writeln!(handle, "Error making API request: {}", e).unwrap_or_default();
                let _ = handle.flush();
                process::exit(1);
            }
        };
    debug!("body: {:#?}", body);
    // Extract spaces
    let spaces = body.get("spaces").ok_or("No spaces found in response")?;

    let mut clickup_spaces = vec![];
    for space in spaces.as_array().ok_or("Spaces is not an array")? {
        let mut clickup_space = ClickupSpace {
            id: space
                .get("id")
                .ok_or("Space missing id")?
                .as_str()
                .ok_or("Space id is not a string")?
                .to_string(),
            name: space
                .get("name")
                .ok_or("Space missing name")?
                .as_str()
                .ok_or("Space name is not a string")?
                .to_string(),
            priorities: None,
            members: None,
            statuses: None,
        };

        // Extract members
        if let Some(serde_members) = space.get("members") {
            let mut space_members: Vec<ClickupMember> = vec![];
            let serde_members_data = serde_members.as_array().unwrap();

            for member_parent in serde_members_data {
                let member = member_parent.get("user").unwrap();
                let space_member = ClickupMember {
                    id: member.get("id").unwrap().as_i64().unwrap(),
                    initials: member
                        .get("initials")
                        .unwrap()
                        .as_str()
                        .unwrap()
                        .to_string(),
                    username: member
                        .get("username")
                        .unwrap()
                        .as_str()
                        .unwrap()
                        .to_string(),
                };
                space_members.push(space_member);
            }
            clickup_space.members = Some(space_members);
        }

        // Extract statuses
        if let Some(serde_statuses) = space.get("statuses") {
            let mut space_statuses: Vec<ClickupStatus> = vec![];
            let serde_statuses_data = serde_statuses.as_array().unwrap();

            for status in serde_statuses_data {
                let space_status = ClickupStatus {
                    id: status.get("id").unwrap().as_str().unwrap().to_string(),
                    status: status.get("status").unwrap().as_str().unwrap().to_string(),
                    status_type: status.get("type").unwrap().as_str().unwrap().to_string(),
                };
                space_statuses.push(space_status);
            }
            clickup_space.statuses = Some(space_statuses);
        }

        if let Some(features) = space.get("features") {
            // Extract priorities
            if let Some(serde_priorities) = features.get("priorities") {
                let mut space_priorities: Vec<ClickupPriority> = vec![];
                if serde_priorities.get("enabled").is_some_and(|x| x == true) {
                    let serde_priorities_data = serde_priorities
                        .get("priorities")
                        .unwrap()
                        .as_array()
                        .unwrap();

                    for priority in serde_priorities_data {
                        let space_priority = ClickupPriority {
                            id: priority.get("id").unwrap().as_str().unwrap().to_string(),
                            priority: priority
                                .get("priority")
                                .unwrap()
                                .as_str()
                                .unwrap()
                                .to_string(),
                        };
                        space_priorities.push(space_priority);
                    }
                    clickup_space.priorities = Some(space_priorities);
                }
            }
        }
        clickup_spaces.push(clickup_space);
    }
    info!("clickup spaces {:?}", clickup_spaces);

    let clickup_yaml_config = save_clickup_config(&directory, Some(clickup_spaces))
        .ok()
        .expect("Should have save the Clickup config");
    Ok(clickup_yaml_config)
}
