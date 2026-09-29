use reqwest::{header, Client, StatusCode};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::sleep;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubUser {
    pub login: String,
    pub id: u64,
    pub name: Option<String>,
    pub avatar_url: Option<String>,
    pub html_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamMembershipResponse {
    pub state: String, // "active" or "pending"
    pub role: String,
}

#[derive(Clone)]
pub struct GitHubClient {
    client: Client,
    base_url: String,
}

impl GitHubClient {
    pub fn new(token: &str) -> Result<Self, String> {
        let mut headers = header::HeaderMap::new();
        let auth_val = format!("Bearer {}", token.trim());
        let mut auth_header = header::HeaderValue::from_str(&auth_val)
            .map_err(|e| format!("Invalid authorization header: {}", e))?;
        auth_header.set_sensitive(true);

        headers.insert(header::AUTHORIZATION, auth_header);
        headers.insert(
            header::USER_AGENT,
            header::HeaderValue::from_static("Tauri-GitHub-Org-Manager/1.0"),
        );
        headers.insert(
            header::ACCEPT,
            header::HeaderValue::from_static("application/vnd.github+json"),
        );
        headers.insert(
            header::HeaderName::from_static("x-github-api-version"),
            header::HeaderValue::from_static("2022-11-28"),
        );

        let client = Client::builder()
            .default_headers(headers)
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

        Ok(Self {
            client,
            base_url: "https://api.github.com".to_string(),
        })
    }

    /// Verifies authentication and returns user profile
    pub async fn verify_auth(&self) -> Result<GitHubUser, String> {
        let url = format!("{}/user", self.base_url);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Connection error: {}", e))?;

        if resp.status().is_success() {
            let user = resp
                .json::<GitHubUser>()
                .await
                .map_err(|e| format!("Failed to parse user profile: {}", e))?;
            Ok(user)
        } else {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            Err(format!("GitHub API Error (HTTP {}): {}", status, body))
        }
    }

    /// Invites or adds a user to a team: PUT /orgs/{org}/teams/{team_slug}/memberships/{username}
    pub async fn invite_team_member(
        &self,
        org: &str,
        team_slug: &str,
        username: &str,
        role: &str,
    ) -> Result<TeamMembershipResponse, String> {
        let url = format!(
            "{}/orgs/{}/teams/{}/memberships/{}",
            self.base_url, org, team_slug, username
        );
        let payload = serde_json::json!({
            "role": role
        });

        let resp = self
            .client
            .put(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if resp.status().is_success() {
            let data = resp
                .json::<TeamMembershipResponse>()
                .await
                .map_err(|e| format!("Failed to parse membership response: {}", e))?;
            Ok(data)
        } else {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            Err(format!("HTTP {}: {}", status, body))
        }
    }

    /// Checks if a team exists, and if not creates it (closed privacy)
    pub async fn get_or_create_team(&self, org: &str, team_name: &str) -> Result<u64, String> {
        let slug = crate::parser::normalize_team_slug(team_name);
        let check_url = format!("{}/orgs/{}/teams/{}", self.base_url, org, slug);

        let check_resp = self
            .client
            .get(&check_url)
            .send()
            .await
            .map_err(|e| format!("Failed to check team: {}", e))?;

        if check_resp.status().is_success() {
            let val: serde_json::Value = check_resp
                .json()
                .await
                .map_err(|e| format!("Failed to parse team JSON: {}", e))?;
            if let Some(id) = val.get("id").and_then(|v| v.as_u64()) {
                return Ok(id);
            }
        }

        // If not found or couldn't get ID, attempt create
        let create_url = format!("{}/orgs/{}/teams", self.base_url, org);
        let payload = serde_json::json!({
            "name": team_name,
            "privacy": "closed"
        });

        let create_resp = self
            .client
            .post(&create_url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("Failed to create team: {}", e))?;

        if create_resp.status().is_success() || create_resp.status() == StatusCode::CREATED {
            let val: serde_json::Value = create_resp
                .json()
                .await
                .map_err(|e| format!("Failed to parse created team JSON: {}", e))?;
            val.get("id")
                .and_then(|v| v.as_u64())
                .ok_or_else(|| "Created team response missing 'id'".to_string())
        } else {
            let status = create_resp.status();
            let body = create_resp.text().await.unwrap_or_default();
            Err(format!("Failed to create team (HTTP {}): {}", status, body))
        }
    }

    /// Fetches a GitHub user's ID
    pub async fn get_user_id(&self, username: &str) -> Result<u64, String> {
        let url = format!("{}/users/{}", self.base_url, username);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if resp.status().is_success() {
            let val: serde_json::Value = resp
                .json()
                .await
                .map_err(|e| format!("Failed to parse user JSON: {}", e))?;
            val.get("id")
                .and_then(|v| v.as_u64())
                .ok_or_else(|| format!("User '{}' found but missing id field", username))
        } else {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            Err(format!("User '{}' not found (HTTP {}): {}", username, status, body))
        }
    }

    /// Invites a user to organization and adds to team
    pub async fn invite_user_to_org(
        &self,
        org: &str,
        user_id: u64,
        team_id: Option<u64>,
    ) -> Result<String, String> {
        let url = format!("{}/orgs/{}/invitations", self.base_url, org);
        let mut payload = serde_json::json!({
            "invitee_id": user_id,
            "role": "direct_member"
        });

        if let Some(tid) = team_id {
            payload["team_ids"] = serde_json::json!([tid]);
        }

        let resp = self
            .client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        let status = resp.status();
        if status.is_success() || status == StatusCode::CREATED {
            Ok("[INVITED/SYNCED]".to_string())
        } else if status == StatusCode::UNPROCESSABLE_ENTITY {
            // Already member or invitation pending
            Ok("[ALREADY IN ORG/PENDING]".to_string())
        } else {
            let body = resp.text().await.unwrap_or_default();
            Err(format!("HTTP {}: {}", status, body))
        }
    }

    /// Generates repository from template: POST /repos/{template_owner}/{template_repo}/generate
    pub async fn generate_repo_from_template(
        &self,
        template_owner: &str,
        template_repo: &str,
        new_owner: &str,
        new_name: &str,
    ) -> Result<(), String> {
        let url = format!(
            "{}/repos/{}/{}/generate",
            self.base_url, template_owner, template_repo
        );
        let payload = serde_json::json!({
            "owner": new_owner,
            "name": new_name,
            "private": true
        });

        let resp = self
            .client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if resp.status().is_success() || resp.status() == StatusCode::CREATED {
            Ok(())
        } else {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            Err(format!("Generate repository failed (HTTP {}): {}", status, body))
        }
    }

    /// Waits for a freshly generated repository to become available and returns default branch
    pub async fn wait_for_repo(&self, owner: &str, repo: &str, max_retries: u32) -> Result<String, String> {
        let url = format!("{}/repos/{}/{}", self.base_url, owner, repo);
        for attempt in 1..=max_retries {
            let resp = self.client.get(&url).send().await;
            if let Ok(res) = resp {
                if res.status().is_success() {
                    let val: serde_json::Value = res.json().await.unwrap_or_default();
                    let default_branch = val
                        .get("default_branch")
                        .and_then(|v| v.as_str())
                        .unwrap_or("main")
                        .to_string();
                    return Ok(default_branch);
                }
            }
            if attempt < max_retries {
                sleep(Duration::from_secs(2)).await;
            }
        }
        Ok("main".to_string())
    }

    /// Adds a collaborator with given permission ("push" or "maintain")
    pub async fn add_collaborator(
        &self,
        owner: &str,
        repo: &str,
        username: &str,
        permission: &str,
    ) -> Result<(), String> {
        let url = format!(
            "{}/repos/{}/{}/collaborators/{}",
            self.base_url, owner, repo, username
        );
        let payload = serde_json::json!({
            "permission": permission
        });

        let resp = self
            .client
            .put(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if resp.status().is_success()
            || resp.status() == StatusCode::CREATED
            || resp.status() == StatusCode::NO_CONTENT
        {
            Ok(())
        } else {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            Err(format!("HTTP {}: {}", status, body))
        }
    }

    /// Updates repository description: PATCH /repos/{owner}/{repo}
    pub async fn update_repo_description(
        &self,
        owner: &str,
        repo: &str,
        description: &str,
    ) -> Result<(), String> {
        let url = format!("{}/repos/{}/{}", self.base_url, owner, repo);
        let payload = serde_json::json!({
            "description": description
        });

        let resp = self
            .client
            .patch(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if resp.status().is_success() {
            Ok(())
        } else {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            Err(format!("HTTP {}: {}", status, body))
        }
    }

    /// Creates milestone: POST /repos/{owner}/{repo}/milestones
    pub async fn create_milestone(
        &self,
        owner: &str,
        repo: &str,
        title: &str,
        due_on: &str,
    ) -> Result<u64, String> {
        let url = format!("{}/repos/{}/{}/milestones", self.base_url, owner, repo);
        let payload = serde_json::json!({
            "title": title,
            "due_on": due_on
        });

        let resp = self
            .client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if resp.status().is_success() || resp.status() == StatusCode::CREATED {
            let val: serde_json::Value = resp
                .json()
                .await
                .map_err(|e| format!("Failed to parse milestone response: {}", e))?;
            val.get("number")
                .and_then(|v| v.as_u64())
                .ok_or_else(|| "Milestone response missing number".to_string())
        } else {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            Err(format!("HTTP {}: {}", status, body))
        }
    }

    /// Creates issue linked to milestone: POST /repos/{owner}/{repo}/issues
    pub async fn create_issue(
        &self,
        owner: &str,
        repo: &str,
        title: &str,
        body: &str,
        milestone: Option<u64>,
    ) -> Result<u64, String> {
        let url = format!("{}/repos/{}/{}/issues", self.base_url, owner, repo);
        let mut payload = serde_json::json!({
            "title": title,
            "body": body
        });
        if let Some(m) = milestone {
            payload["milestone"] = serde_json::json!(m);
        }

        let resp = self
            .client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if resp.status().is_success() || resp.status() == StatusCode::CREATED {
            let val: serde_json::Value = resp
                .json()
                .await
                .map_err(|e| format!("Failed to parse issue response: {}", e))?;
            val.get("number")
                .and_then(|v| v.as_u64())
                .ok_or_else(|| "Issue response missing number".to_string())
        } else {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            Err(format!("HTTP {}: {}", status, body))
        }
    }

    /// Retrieves SHA of a branch head: GET /repos/{owner}/{repo}/git/ref/heads/{branch}
    pub async fn get_branch_sha(
        &self,
        owner: &str,
        repo: &str,
        branch: &str,
    ) -> Result<String, String> {
        let url = format!(
            "{}/repos/{}/{}/git/ref/heads/{}",
            self.base_url, owner, repo, branch
        );
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if resp.status().is_success() {
            let val: serde_json::Value = resp
                .json()
                .await
                .map_err(|e| format!("Failed to parse git ref: {}", e))?;
            val.get("object")
                .and_then(|o| o.get("sha"))
                .and_then(|s| s.as_str())
                .map(|s| s.to_string())
                .ok_or_else(|| "Git ref response missing object.sha".to_string())
        } else {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            Err(format!("HTTP {}: {}", status, body))
        }
    }

    /// Creates a new git branch: POST /repos/{owner}/{repo}/git/refs
    pub async fn create_branch(
        &self,
        owner: &str,
        repo: &str,
        branch_name: &str,
        sha: &str,
    ) -> Result<(), String> {
        let url = format!("{}/repos/{}/{}/git/refs", self.base_url, owner, repo);
        let payload = serde_json::json!({
            "ref": format!("refs/heads/{}", branch_name),
            "sha": sha
        });

        let resp = self
            .client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if resp.status().is_success() || resp.status() == StatusCode::CREATED {
            Ok(())
        } else {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            Err(format!("HTTP {}: {}", status, body))
        }
    }

    /// Creates or updates a file in the repo: PUT /repos/{owner}/{repo}/contents/{path}
    pub async fn create_file(
        &self,
        owner: &str,
        repo: &str,
        path: &str,
        message: &str,
        content_b64: &str,
        branch: &str,
    ) -> Result<(), String> {
        let url = format!("{}/repos/{}/{}/contents/{}", self.base_url, owner, repo, path);
        let payload = serde_json::json!({
            "message": message,
            "content": content_b64,
            "branch": branch
        });

        let resp = self
            .client
            .put(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if resp.status().is_success() || resp.status() == StatusCode::CREATED {
            Ok(())
        } else {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            Err(format!("HTTP {}: {}", status, body))
        }
    }

    /// Creates a Pull Request: POST /repos/{owner}/{repo}/pulls
    pub async fn create_pull_request(
        &self,
        owner: &str,
        repo: &str,
        title: &str,
        body: &str,
        head: &str,
        base: &str,
    ) -> Result<u64, String> {
        let url = format!("{}/repos/{}/{}/pulls", self.base_url, owner, repo);
        let payload = serde_json::json!({
            "title": title,
            "body": body,
            "head": head,
            "base": base
        });

        let resp = self
            .client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if resp.status().is_success() || resp.status() == StatusCode::CREATED {
            let val: serde_json::Value = resp
                .json()
                .await
                .map_err(|e| format!("Failed to parse PR response: {}", e))?;
            val.get("number")
                .and_then(|v| v.as_u64())
                .ok_or_else(|| "PR response missing number".to_string())
        } else {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            Err(format!("HTTP {}: {}", status, body))
        }
    }

    /// Requests reviewers on a PR: POST /repos/{owner}/{repo}/pulls/{number}/requested_reviewers
    pub async fn request_reviewers(
        &self,
        owner: &str,
        repo: &str,
        pull_number: u64,
        reviewers: &[String],
    ) -> Result<(), String> {
        if reviewers.is_empty() {
            return Ok(());
        }
        let url = format!(
            "{}/repos/{}/{}/pulls/{}/requested_reviewers",
            self.base_url, owner, repo, pull_number
        );
        let payload = serde_json::json!({
            "reviewers": reviewers
        });

        let resp = self
            .client
            .post(&url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if resp.status().is_success() || resp.status() == StatusCode::CREATED {
            Ok(())
        } else {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            Err(format!("HTTP {}: {}", status, body))
        }
    }
}
