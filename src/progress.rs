use chrono::{Datelike, Duration, Local, NaiveDate, Weekday};
use clap::ArgMatches;
use log::info;
use regex::Regex;
use std::collections::HashMap;
use std::convert::TryInto;
use std::io::{self, Write};
use std::process::Command;

use crate::types::github_types::{
    GithubIssue, GithubMilestone, GithubProjectItem, GithubProjectResponse, GithubProjectStatus,
    Milestone, ProjectContent, ProjectMilestone,
};

fn format_issue_display(item: &GithubProjectItem) -> String {
    let milestone = item
        .milestone
        .as_ref()
        .and_then(|m| m.title.as_ref())
        .unwrap_or(&String::from("No milestone"))
        .clone();
    let title = item.title.as_deref().unwrap_or("No title");
    let id = item.content.as_ref().and_then(|c| c.number).unwrap_or(0);
    let url = item
        .content
        .as_ref()
        .and_then(|c| c.url.as_deref())
        .unwrap_or("No URL");

    let mut message = format!("- [{}] {} - [#{}]({})\n", milestone, title, id, url);
    if &item.content.as_ref().is_some_and(|c| c.body.is_some()) == &true {
        let body = item.clone().content.unwrap().body.unwrap();
        let re: Regex =
            Regex::new(r"(?s)BUSINESS WRITE UP START.*?-->(.*?)<!--.*?BUSINESS WRITE UP END")
                .unwrap();
        if let Some(capture) = re.captures(&body) {
            let business_write_up = &capture[1];
            let lines: Vec<String> = business_write_up
                .split("\n")
                .map(|s| s.trim().to_string())
                .collect();

            for line in lines {
                if line.len() > 0 {
                    message.push_str(&format!("> {}\n", line));
                }
            }
        }
    }

    message
}

fn sort_by_milestone(mut items: Vec<GithubProjectItem>) -> Vec<GithubProjectItem> {
    items.sort_by(|a, b| {
        let milestone_a = a
            .milestone
            .as_ref()
            .and_then(|m| m.title.as_ref())
            .unwrap_or(&String::from("No milestone"))
            .clone();
        let milestone_b = b
            .milestone
            .as_ref()
            .and_then(|m| m.title.as_ref())
            .unwrap_or(&String::from("No milestone"))
            .clone();
        milestone_a.cmp(&milestone_b)
    });
    items
}

pub fn last_thursday() -> NaiveDate {
    let today = Local::now().date_naive();

    // Find how many days to go back to reach Monday of current week
    let days_to_monday = match today.weekday() {
        Weekday::Mon => 0,
        Weekday::Tue => 1,
        Weekday::Wed => 2,
        Weekday::Thu => 3,
        Weekday::Fri => 4,
        Weekday::Sat => 5,
        Weekday::Sun => 6,
    };

    // Get this week's Monday, then go back 4 days to get last Thursday
    let this_monday = today - Duration::days(days_to_monday);
    this_monday - Duration::days(4)
}

pub fn this_thursday() -> NaiveDate {
    let today = Local::now().date_naive();

    // Find how many days to go back to reach Monday of current week
    let days_to_monday = match today.weekday() {
        Weekday::Mon => 0,
        Weekday::Tue => 1,
        Weekday::Wed => 2,
        Weekday::Thu => 3,
        Weekday::Fri => 4,
        Weekday::Sat => 5,
        Weekday::Sun => 6,
    };

    // Get this week's Monday, then add 3 days to get this Thursday
    let this_monday = today - Duration::days(days_to_monday);
    this_monday + Duration::days(3)
}

pub fn next_thursday() -> NaiveDate {
    let thursday = this_thursday();
    thursday + Duration::days(7)
}
pub fn this_thursday_string() -> String {
    let thursday = this_thursday();

    thursday.format("%Y-%m-%d").to_string()
}

pub fn last_thursday_string() -> String {
    let last_thursday = last_thursday();

    last_thursday.format("%Y-%m-%d").to_string()
}

pub async fn progress(matches: ArgMatches<'static>) {
    info!("Progress function called");

    let stdout = io::stdout(); // get the global stdout entity
    let mut handle = io::BufWriter::new(&stdout); // optional: wrap that handle in a buffer
    let mut progress_output = "```{markdown}".to_owned();
    let projects_str = matches
        .value_of("projects")
        .expect("projects should be provide");
    let projects: Vec<String> = projects_str
        .split(',')
        .map(|s| s.trim().to_string())
        .collect();

    let date_to_use = this_thursday_string();

    let mut milestone_issues: Vec<GithubProjectItem> = Vec::new();
    for project in &projects {
        let output = Command::new("gh")
            .arg("project")
            .arg("item-list")
            .arg(project)
            .arg("--owner")
            .arg("wearebenlabs")
            .arg("--format=json")
            .arg("-L")
            .arg("100")
            .output()
            .expect("Failed to execute gh project item-list command");

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            match serde_json::from_str::<GithubProjectResponse>(&stdout) {
                Ok(response) => {
                    if let Some(temp_items) = response.items {
                        milestone_issues.extend(temp_items.clone());
                        info!(
                            "Successfully fetched {} items from milestone {}",
                            temp_items.len(),
                            "11",
                        );
                    }
                }
                Err(e) => {
                    info!(
                        "Failed to parse project items JSON for milestone {}: {}",
                        "11", e
                    );
                }
            }
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            info!(
                "Github project item-list command failed for milestone {}: {}",
                "11", stderr
            );
        }
    }

    let mut blocked_issues: Vec<GithubProjectItem> = Vec::new();
    let mut status_counts: HashMap<&str, i32> = HashMap::from([
        ("closed", 0),
        ("shipped", 0),
        ("progress", 0),
        ("blocked", 0),
        ("support", 0),
    ]);
    let mut closed_issues: Vec<GithubProjectItem> = Vec::new();
    let mut in_progress_issues: Vec<GithubProjectItem> = Vec::new();
    let mut next_week_issues: Vec<GithubProjectItem> = Vec::new();
    for milestone_item in &milestone_issues {
        if let Some(labels) = &milestone_item.labels {
            if labels.contains(&"blocked".to_string()) {
                blocked_issues.push(milestone_item.clone());
                *status_counts.get_mut("blocked").unwrap() += 1;
                continue;
            }
        }
        let this_thursday_date = this_thursday();
        let last_thursday_date = last_thursday();
        let next_thursday_date = next_thursday();

        if let Some(status) = &milestone_item.status {
            if status == "In Progress" {
                *status_counts.get_mut("progress").unwrap() += 1;
                in_progress_issues.push(milestone_item.clone());
            }
            if status == "Done" {
                if milestone_item.shipped_date.as_ref().is_some() {
                    let shipped_date = chrono::NaiveDate::parse_from_str(
                        milestone_item
                            .shipped_date
                            .as_ref()
                            .expect("should be a string"),
                        "%Y-%m-%d",
                    )
                    .expect("should have a string date");

                    if shipped_date >= last_thursday_date && shipped_date <= this_thursday_date {
                        closed_issues.push(milestone_item.clone());
                        *status_counts.get_mut("shipped").unwrap() += 1;
                        continue;
                    }
                }
                if milestone_item.shipped_date.as_ref().is_none() {
                    *status_counts.get_mut("closed").unwrap() += 1;
                    continue;
                }
                if let Some(labels) = &milestone_item.labels {
                    if labels.contains(&"support".to_string()) {
                        *status_counts.get_mut("support").unwrap() += 1;
                        continue;
                    }
                }
            }
        }

        let mut should_add_to_next_week = false;

        if let Some(end_date_str) = &milestone_item.end_date {
            if let Ok(end_date) = chrono::NaiveDate::parse_from_str(end_date_str, "%Y-%m-%d") {
                if end_date > this_thursday_date && end_date <= next_thursday_date {
                    should_add_to_next_week = true;
                }
            }
        }
        if let Some(start_date_str) = &milestone_item.start_date {
            if let Ok(start_date) = chrono::NaiveDate::parse_from_str(start_date_str, "%Y-%m-%d") {
                if start_date > this_thursday_date && start_date <= next_thursday_date {
                    should_add_to_next_week = true;
                }
            }
        }
        if should_add_to_next_week {
            next_week_issues.push(milestone_item.clone());
        }
    }

    progress_output.push_str(&format!(
        "\n## Week of {}\n### 🚢 Shipped Features\n",
        &date_to_use
    ));
    let sorted_closed_issues = sort_by_milestone(closed_issues);
    for item in &sorted_closed_issues {
        let formatted_issue = format_issue_display(item);

        progress_output.push_str(&formatted_issue);
    }

    let shipped_len: i8 = sorted_closed_issues.len().try_into().unwrap_or(0);
    let closed_len: i8 = status_counts["closed"].try_into().unwrap_or(0);
    if shipped_len < closed_len {
        let diff = closed_len - shipped_len;
        progress_output.push_str("> [!NOTE]\n");
        progress_output.push_str(&format!(
            "> Not displaying {} closed issues not considered user facing features\n",
            diff,
        ));
    }

    progress_output.push_str(&"\n### 🏗️ In Progress\n");
    let sorted_in_progress_issues = sort_by_milestone(in_progress_issues);
    for item in &sorted_in_progress_issues {
        let formatted_issue = format_issue_display(item);

        progress_output.push_str(&formatted_issue);
    }

    progress_output.push_str(&"\n### 🚧 Blockers/Needs\n");
    let sorted_blocked_issues = sort_by_milestone(blocked_issues);
    for item in &sorted_blocked_issues {
        let formatted_issue = format_issue_display(item);
        progress_output.push_str(&formatted_issue);
    }

    progress_output.push_str(&"\n### 🎯 Next Week Focus\n");
    let sorted_next_week_issues = sort_by_milestone(next_week_issues);
    for item in &sorted_next_week_issues {
        let formatted_issue = format_issue_display(item);
        progress_output.push_str(&formatted_issue);
    }

    progress_output.push_str(&"\n### 📊 Metrics\n");
    for (key, value) in &status_counts {
        if *value == 0 {
            continue;
        }
        let metric_entry_label = match *key {
            "progress" => "issues being worked on",
            "support" => "customer issues addressed",
            "blocked" => "issues blocked",
            "closed" => "issues closed",
            "shipped" => "features shipped",
            _ => key,
        };

        progress_output.push_str(&format!("- {} {}\n", value, metric_entry_label));
    }
    progress_output.push_str("```\n");
    write!(handle, "{}", progress_output).unwrap_or_default();
    let _ = handle.flush();
}
