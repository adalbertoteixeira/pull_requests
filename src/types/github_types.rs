use serde::{Deserialize, Serialize};

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
    #[serde(rename = "end date")]
    pub end_date: Option<String>,
    pub id: Option<String>,
    pub labels: Option<Vec<String>>,
    pub milestone: Option<ProjectMilestone>,
    pub repository: Option<String>,
    #[serde(rename = "start date")]
    pub start_date: Option<String>,
    pub status: Option<String>,
    pub title: Option<String>,
    #[serde(rename = "shipped date")]
    pub shipped_date: Option<String>,
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
    #[serde(rename = "dueOn")]
    pub due_on: Option<String>,
    pub closed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GithubProjectResponse {
    pub items: Option<Vec<GithubProjectItem>>,
    #[serde(rename = "totalCount")]
    pub total_count: Option<i16>,
}
