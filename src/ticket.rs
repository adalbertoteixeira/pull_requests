use std::{
    io::{self, Write},
    process,
};

use clap::ArgMatches;
use log::{debug, info};

use crate::utils::extract_clickup_spaces_data::{
    extract_clickup_spaces_data, make_clickup_request,
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
            debug!("Calling subcommnand workspaces {:?}", arg);

            let _ = extract_clickup_spaces_data(&directory, &matches, &client)
                .await
                .unwrap();
        }
        ("issue", Some(arg)) => {
            info!("issue {:?}", arg);

            let issue_id = arg.value_of("issue").unwrap_or("");
            if issue_id.is_empty() {
                writeln!(handle, "Issue ID is required").unwrap_or_default();
                let _ = handle.flush();
                process::exit(1);
            }

            let url = format!(
                "https://api.clickup.com/api/v2/task/{}?include_markdown_description=true",
                issue_id
            );

            let body = match make_clickup_request(
                &client,
                &url,
                matches.value_of("clickup_api_key").unwrap(),
            )
            .await
            {
                Ok(b) => b,
                Err(e) => {
                    writeln!(handle, "Error fetching issue: {}", e).unwrap_or_default();
                    let _ = handle.flush();
                    process::exit(1);
                }
            };

            // Extract and output the description
            if let Some(description) = body.get("markdown_description") {
                writeln!(handle, "Issue Description:").unwrap_or_default();
                writeln!(
                    handle,
                    "{}",
                    description.as_str().unwrap_or("No description available")
                )
                .unwrap_or_default();
            } else if let Some(description) = body.get("description") {
                writeln!(handle, "Issue Description:").unwrap_or_default();
                writeln!(
                    handle,
                    "{}",
                    description.as_str().unwrap_or("No description available")
                )
                .unwrap_or_default();
            } else {
                writeln!(handle, "No description found for this issue").unwrap_or_default();
            }

            // Also output other useful information
            if let Some(name) = body.get("name") {
                writeln!(handle, "\nIssue Name: {}", name.as_str().unwrap_or("N/A"))
                    .unwrap_or_default();
            }

            if let Some(status) = body.get("status") {
                if let Some(status_name) = status.get("status") {
                    writeln!(handle, "Status: {}", status_name.as_str().unwrap_or("N/A"))
                        .unwrap_or_default();
                }
            }

            let _ = handle.flush();
        }
        _ => {}
    }
}
