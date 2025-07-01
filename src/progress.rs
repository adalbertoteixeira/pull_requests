use chrono::{Datelike, Duration, Local, NaiveDate, Weekday};
use clap::ArgMatches;
use inquire::MultiSelect;
use log::{debug, info};
use serde::{Deserialize, Serialize};
use std::io::{self, Write};
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GithubIssue {
    pub body: Option<String>,
    pub closed: Option<bool>,
    #[serde(rename = "closedAt")]
    pub closed_at: Option<String>,
    pub labels: Option<Vec<serde_json::Value>>,
    pub milestone: Option<Milestone>,
    pub number: Option<u32>,
    #[serde(rename = "projectCards")]
    pub project_cards: Option<Vec<serde_json::Value>>,
    #[serde(rename = "projectItems")]
    pub project_items: Option<Vec<GithubProjectItem>>,
    #[serde(rename = "reactionGroups")]
    pub reaction_groups: Option<Vec<serde_json::Value>>,
    pub title: Option<String>,
    pub url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Milestone {
    pub number: Option<u32>,
    pub title: Option<String>,
    pub description: Option<String>,
    #[serde(rename = "dueOn")]
    pub due_on: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GithubProjectItem {
    pub assignees: Option<Vec<String>>,
    pub content: Option<ProjectContent>,
    #[serde(rename = "end Date")]
    pub end_date: Option<String>,
    pub id: Option<String>,
    pub labels: Option<Vec<String>>,
    pub milestone: Option<ProjectMilestone>,
    pub repository: Option<String>,
    #[serde(rename = "start date")]
    pub start_date: Option<String>,
    pub status: Option<String>,
    pub title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectContent {
    pub body: Option<String>,
    pub number: Option<u32>,
    pub repository: Option<String>,
    pub title: Option<String>,
    #[serde(rename = "type")]
    pub content_type: Option<String>,
    pub url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMilestone {
    pub description: Option<String>,
    #[serde(rename = "dueOn")]
    pub due_on: Option<String>,
    pub title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GithubProjectStatus {
    #[serde(rename = "optionId")]
    pub option_id: Option<String>,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GithubMilestone {
    pub url: Option<String>,
    pub html_url: Option<String>,
    pub labels_url: Option<String>,
    pub id: Option<u64>,
    pub node_id: Option<String>,
    pub number: Option<u32>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub creator: Option<serde_json::Value>,
    pub open_issues: Option<u32>,
    pub closed_issues: Option<u32>,
    pub state: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub due_on: Option<String>,
    pub closed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GithubProjectResponse {
    pub items: Option<Vec<GithubProjectItem>>,
    #[serde(rename = "totalCount")]
    pub total_count: Option<i16>,
}

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

    format!("- [{}] {} - (#{})[{}]\n", milestone, title, id, url)
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
    info!("Computing last Thursday date");
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
    info!("Computing this Thursday date");
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

    let date_to_use = last_thursday_string();

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
                            "Successfully fetched {} items from milestone {}: {:#?}",
                            temp_items.len(),
                            "11",
                            temp_items
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

    let mut closed_issues: Vec<GithubProjectItem> = Vec::new();
    let mut in_progress_issues: Vec<GithubProjectItem> = Vec::new();
    let mut blocked_issues: Vec<GithubProjectItem> = Vec::new();
    let mut next_week_issues: Vec<GithubProjectItem> = Vec::new();
    for milestone_item in &milestone_issues {
        if let Some(status) = &milestone_item.status {
            if status == "In Progress" {
                in_progress_issues.push(milestone_item.clone());
            }
            if status == "Done" {
                if let Some(labels) = &milestone_item.labels {
                    if labels.contains(&"shipped".to_string()) {
                        closed_issues.push(milestone_item.clone());
                    } else {
                        in_progress_issues.push(milestone_item.clone());
                    }
                } else {
                    in_progress_issues.push(milestone_item.clone());
                }
            }
        }
        if let Some(labels) = &milestone_item.labels {
            if labels.contains(&"blocked".to_string()) {
                blocked_issues.push(milestone_item.clone());
            }
        }

        let last_thursday_date = last_thursday();
        let next_thursday_date = next_thursday();

        let mut should_add_to_next_week = false;

        if let Some(start_date_str) = &milestone_item.start_date {
            if let Ok(start_date) = chrono::NaiveDate::parse_from_str(start_date_str, "%Y-%m-%d") {
                if start_date >= last_thursday_date && start_date <= next_thursday_date {
                    should_add_to_next_week = true;
                }
            }
        }

        if let Some(end_date_str) = &milestone_item.end_date {
            if let Ok(end_date) = chrono::NaiveDate::parse_from_str(end_date_str, "%Y-%m-%d") {
                if end_date >= last_thursday_date && end_date <= next_thursday_date {
                    should_add_to_next_week = true;
                }
            }
        }

        if should_add_to_next_week {
            next_week_issues.push(milestone_item.clone());
        }
    }

    progress_output.push_str(&format!("\n## Week of {}\n### 🚢 Done\n", &date_to_use));
    let sorted_closed_issues = sort_by_milestone(closed_issues);
    for item in &sorted_closed_issues {
        let formatted_issue = format_issue_display(item);

        progress_output.push_str(&formatted_issue);
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

    progress_output.push_str("```\n");
    write!(handle, "{}", progress_output).unwrap_or_default();
    let _ = handle.flush();

    // info!("Found {:?} total issues:", github_milestones);
}
