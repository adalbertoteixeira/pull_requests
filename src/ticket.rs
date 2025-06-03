use reqwest::Client;
use serde_json::{Value, json};
use std::{
    io::{self, Write},
    process,
};

use clap::ArgMatches;
use log::{debug, info};

use crate::storage::{
    ClickupMember, ClickupPriority, ClickupSpace, ClickupStatus, save_clickup_config,
};

pub async fn ticket(matches: ArgMatches<'static>, directory: &str) {
    info!("Ticket command");
    let stdout = io::stdout(); // get the global stdout entity
    let mut handle = io::BufWriter::new(&stdout); // optional: wrap that handle in a buffer
    debug!("matches {:?}", matches);
    if !matches.is_present("clickup_api_key") {
        info!("KEY: {:?}", matches.value_of("clickup_api_key"));
        writeln!(handle, "{}", "No Clickup api key present. Cannot continue.").unwrap_or_default();
        let _ = handle.flush();
        process::exit(1)
    }

    let client = reqwest::Client::new();
    match matches.subcommand() {
        ("spaces", Some(arg)) => {
            info!("workspaces {:?}", arg);

            let mut url: String = "https://api.clickup.com/api/v2/team/".to_owned();
            url.push_str(&matches.value_of("clickup_workspace_id").unwrap());
            url.push_str("/space");

            let mut authorization: String = "Bearer ".to_owned();
            debug!("{:?}", matches.value_of("clickup_api_key").unwrap());
            authorization.push_str(matches.value_of("clickup_api_key").unwrap());
            debug!("authorization: {:?}", authorization);
            let res = client
                .get(url)
                .header("Accept", "application/json")
                .header(
                    "Authorization",
                    matches.value_of("clickup_api_key").unwrap(),
                )
                .body("")
                .send()
                .await
                .unwrap();

            let status = res.status();
            info!("Status: {}", status);
            let body: serde_json::Value = res.json().await.unwrap();
            // let spaces: Vec<serde_json::Value> = serde_json::from_str(&body).unwrap();
            debug!("body: {:#?}", body);
            if body.get("spaces").is_none() {
                writeln!(handle, "{}", "No spaces found in Clickup").unwrap_or_default();
                let _ = handle.flush();
            }
            let spaces = body.get("spaces").unwrap();
            let mut clickup_spaces = vec![];
            for space in spaces.as_array().unwrap() {
                info!("{:?}", space.get("id"));
                info!("{:?}", space.get("name"));
                let clickup_space = ClickupSpace {
                    id: space.get("id").unwrap().to_string(),
                    name: space.get("name").unwrap().to_string(),
                };
                clickup_spaces.push(clickup_space);
            }
            info!("clickup spaces {:?}", clickup_spaces);

            let priorities = body.get("priorities").unwrap();
            let mut clickup_priorities = vec![];
            for priority in priorities.as_array().unwrap() {
                debug!("{:?}", priority.get("id"));
                debug!("{:?}", priority.get("priority"));
                let clickup_priority = ClickupPriority {
                    id: priority.get("id").unwrap().to_string(),
                    priority: priority.get("priority").unwrap().to_string(),
                };
                clickup_priorities.push(clickup_priority);
            }
            info!("clickup priorities {:?}", clickup_priorities);
            // statuses: id, status, type
            let statuses = body.get("statuses").unwrap();
            let mut clickup_statuses = vec![];
            for status in statuses.as_array().unwrap() {
                debug!("{:?}", status.get("id"));
                debug!("{:?}", status.get("status"));
                debug!("{:?}", status.get("type"));
                let clickup_status = ClickupStatus {
                    id: status.get("id").unwrap().to_string(),
                    status: status.get("name").unwrap().to_string(),
                    status_type: status.get("status").unwrap().to_string(),
                };
                clickup_statuses.push(clickup_status);
            }
            // memebers: id, initials, profilepicture, username
            let members = body.get("members").unwrap();
            let mut clickup_members = vec![];
            for member in members.as_array().unwrap() {
                info!("{:?}", member.get("id"));
                info!("{:?}", member.get("initials"));
                info!("{:?}", member.get("profilePicture"));
                info!("{:?}", member.get("username"));
                let clickup_member = ClickupMember {
                    id: member.get("id").unwrap().to_string(),
                    initials: member.get("initials").unwrap().to_string(),
                    profile_picture: member.get("profilePicture").unwrap().to_string(),
                    username: member.get("username").unwrap().to_string(),
                };
                clickup_members.push(clickup_member);
            }
            let _ = save_clickup_config(
                &directory,
                Some(clickup_spaces),
                Some(clickup_members),
                Some(clickup_statuses),
                Some(clickup_priorities),
            );
        }
        _ => {}
    }
}
