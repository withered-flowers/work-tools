use base64::prelude::*;
use chrono::Local;
use serde::{Deserialize, Serialize};
use std::process::Command;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tokio::time::sleep;

use crate::github_client::{GitHubClient, GitHubUser};
use crate::parser::{
    parse_repo_assignments, parse_team_invitations, ProvisionPlan, TeamInvitationEntry,
    TemplateEntry,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogMessage {
    pub timestamp: String,
    pub level: String, // "info", "success", "warn", "error"
    pub message: String,
}

pub fn emit_log(app: &AppHandle, level: &str, message: &str) {
    let now = Local::now().format("%H:%M:%S").to_string();
    let _ = app.emit(
        "app-log",
        LogMessage {
            timestamp: now,
            level: level.to_string(),
            message: message.to_string(),
        },
    );
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamInviteProgress {
    pub current_index: usize,
    pub total: usize,
    pub username: String,
    pub team_name: String,
    pub team_slug: String,
    pub status: String,
    pub error: Option<String>,
    pub is_dry_run: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvitationSummary {
    pub total_users: usize,
    pub total_invitations: usize,
    pub success_count: usize,
    pub failed_count: usize,
    pub is_dry_run: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoProvisionProgress {
    pub current_index: usize,
    pub total: usize,
    pub username: String,
    pub target_repo: String,
    pub template_repo: String,
    pub step: String,
    pub status: String,
    pub error: Option<String>,
    pub is_dry_run: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvisionSummary {
    pub total_users: usize,
    pub total_repos: usize,
    pub success_count: usize,
    pub failed_count: usize,
    pub is_dry_run: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoProvisionConfig {
    pub org_name: String,
    pub team_name: Option<String>,
    pub skip_sync: bool,
    pub reviewers: Vec<String>,
    pub dry_run: bool,
}

/// Attempts to get GitHub token from `gh auth token` CLI command if available
#[tauri::command]
pub fn get_gh_cli_token() -> Result<Option<String>, String> {
    match Command::new("gh").args(["auth", "token"]).output() {
        Ok(output) => {
            if output.status.success() {
                let token = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !token.is_empty() {
                    return Ok(Some(token));
                }
            }
            Ok(None)
        }
        Err(_) => Ok(None),
    }
}

/// Verifies a GitHub token against GET /user
#[tauri::command]
pub async fn verify_github_token(token: String) -> Result<GitHubUser, String> {
    let client = GitHubClient::new(&token)?;
    client.verify_auth().await
}

/// Previews team invitation parsing without executing
#[tauri::command]
pub fn preview_team_invitations(input_text: String) -> Result<Vec<TeamInvitationEntry>, String> {
    Ok(parse_team_invitations(&input_text))
}

/// Runs team invitations mirroring 001_invite_teams.sh
#[tauri::command]
pub async fn run_team_invitations(
    token: String,
    org_name: String,
    role: String,
    dry_run: bool,
    entries: Vec<TeamInvitationEntry>,
    app_handle: AppHandle,
) -> Result<InvitationSummary, String> {
    let client = if !dry_run {
        Some(GitHubClient::new(&token)?)
    } else {
        None
    };

    let total_users = entries.len();
    let mut total_invitations = 0;
    for e in &entries {
        total_invitations += e.teams.len();
    }

    emit_log(
        &app_handle,
        "info",
        &format!(
            "Starting Team Invitations in Org '{}' (Role: {}, Dry-Run: {})",
            org_name, role, dry_run
        ),
    );

    let mut success_count = 0;
    let mut failed_count = 0;
    let mut current_invitation = 0;

    for (u_idx, entry) in entries.iter().enumerate() {
        emit_log(
            &app_handle,
            "info",
            &format!(
                "[{}/{}] Processing user: {}",
                u_idx + 1,
                total_users,
                entry.username
            ),
        );

        for team in &entry.teams {
            current_invitation += 1;

            if dry_run {
                emit_log(
                    &app_handle,
                    "warn",
                    &format!(
                        "  -> Team: {} (Slug: {}) [DRY-RUN - WOULD INVITE]",
                        team.display_name, team.slug
                    ),
                );

                let _ = app_handle.emit(
                    "team-invite-progress",
                    TeamInviteProgress {
                        current_index: current_invitation,
                        total: total_invitations,
                        username: entry.username.clone(),
                        team_name: team.display_name.clone(),
                        team_slug: team.slug.clone(),
                        status: "WOULD INVITE (DRY-RUN)".to_string(),
                        error: None,
                        is_dry_run: true,
                    },
                );

                success_count += 1;
            } else {
                let client_ref = client.as_ref().unwrap();
                match client_ref
                    .invite_team_member(&org_name, &team.slug, &entry.username, &role)
                    .await
                {
                    Ok(resp) => {
                        let status_label = if resp.state == "active" {
                            "[ACTIVE MEMBER]".to_string()
                        } else {
                            format!("[INVITATION SENT ({})]", resp.state.to_uppercase())
                        };

                        emit_log(
                            &app_handle,
                            "success",
                            &format!(
                                "  -> Team: {} (Slug: {}) {}",
                                team.display_name, team.slug, status_label
                            ),
                        );

                        let _ = app_handle.emit(
                            "team-invite-progress",
                            TeamInviteProgress {
                                current_index: current_invitation,
                                total: total_invitations,
                                username: entry.username.clone(),
                                team_name: team.display_name.clone(),
                                team_slug: team.slug.clone(),
                                status: status_label,
                                error: None,
                                is_dry_run: false,
                            },
                        );

                        success_count += 1;
                    }
                    Err(err) => {
                        emit_log(
                            &app_handle,
                            "error",
                            &format!(
                                "  -> Team: {} (Slug: {}) [FAILED]: {}",
                                team.display_name, team.slug, err
                            ),
                        );

                        let _ = app_handle.emit(
                            "team-invite-progress",
                            TeamInviteProgress {
                                current_index: current_invitation,
                                total: total_invitations,
                                username: entry.username.clone(),
                                team_name: team.display_name.clone(),
                                team_slug: team.slug.clone(),
                                status: "FAILED".to_string(),
                                error: Some(err),
                                is_dry_run: false,
                            },
                        );

                        failed_count += 1;
                    }
                }
            }
        }
    }

    emit_log(
        &app_handle,
        "info",
        &format!(
            "Completed Team Invitations. Total: {}, Successful: {}, Failed: {}",
            total_invitations, success_count, failed_count
        ),
    );

    Ok(InvitationSummary {
        total_users,
        total_invitations,
        success_count,
        failed_count,
        is_dry_run: dry_run,
    })
}

/// Previews repository provisioning plans
#[tauri::command]
pub fn preview_repo_provisioning(
    input_text: String,
    catalog: Vec<TemplateEntry>,
    fallback_prefix: String,
    default_deadline: String,
    org_name: String,
) -> Result<Vec<ProvisionPlan>, String> {
    Ok(parse_repo_assignments(
        &input_text,
        &catalog,
        &fallback_prefix,
        &default_deadline,
        &org_name,
    ))
}

/// Runs repository provisioning replicating 002_create_repos.sh
#[tauri::command]
pub async fn run_repo_provisioning(
    token: String,
    config: RepoProvisionConfig,
    plans: Vec<ProvisionPlan>,
    app_handle: AppHandle,
) -> Result<ProvisionSummary, String> {
    let client = if !config.dry_run {
        Some(GitHubClient::new(&token)?)
    } else {
        None
    };

    let total_repos = plans.len();
    let mut unique_users = Vec::new();
    for p in &plans {
        if !unique_users.contains(&p.username) {
            unique_users.push(p.username.clone());
        }
    }
    let total_users = unique_users.len();

    emit_log(
        &app_handle,
        "info",
        &format!(
            "Starting Repository Provisioning for Org '{}' (Total Plans: {}, Dry-Run: {})",
            config.org_name, total_repos, config.dry_run
        ),
    );

    // Step 1: Team & Org Membership Sync (Optional)
    if !config.skip_sync && config.team_name.as_ref().map_or(false, |t| !t.trim().is_empty()) {
        let team_name = config.team_name.as_ref().unwrap().trim();
        let mut sync_members = unique_users.clone();
        for r in &config.reviewers {
            let r_trim = r.trim().to_string();
            if !r_trim.is_empty() && !sync_members.contains(&r_trim) {
                sync_members.push(r_trim);
            }
        }

        emit_log(
            &app_handle,
            "info",
            &format!("Synchronizing Team Memberships for team '{}'...", team_name),
        );

        if config.dry_run {
            emit_log(
                &app_handle,
                "warn",
                &format!(
                    "[DRY-RUN] Would ensure team '{}' exists and sync {} members",
                    team_name,
                    sync_members.len()
                ),
            );
        } else {
            let client_ref = client.as_ref().unwrap();
            match client_ref.get_or_create_team(&config.org_name, team_name).await {
                Ok(team_id) => {
                    for member in sync_members {
                        match client_ref.get_user_id(&member).await {
                            Ok(user_id) => {
                                match client_ref
                                    .invite_user_to_org(&config.org_name, user_id, Some(team_id))
                                    .await
                                {
                                    Ok(res) => {
                                        emit_log(
                                            &app_handle,
                                            "success",
                                            &format!("  -> Member: {} {}", member, res),
                                        );
                                    }
                                    Err(e) => {
                                        emit_log(
                                            &app_handle,
                                            "error",
                                            &format!("  -> Member: {} [INVITE ERROR]: {}", member, e),
                                        );
                                    }
                                }
                            }
                            Err(e) => {
                                emit_log(
                                    &app_handle,
                                    "error",
                                    &format!("  -> Member: {} [USER NOT FOUND]: {}", member, e),
                                );
                            }
                        }
                    }
                }
                Err(err) => {
                    emit_log(
                        &app_handle,
                        "error",
                        &format!("Failed to get or create team '{}': {}", team_name, err),
                    );
                }
            }
        }
    }

    // Step 2: Provision Repositories
    let mut success_count = 0;
    let mut failed_count = 0;

    for (idx, plan) in plans.iter().enumerate() {
        let current_index = idx + 1;
        emit_log(
            &app_handle,
            "info",
            &format!(
                "[{}/{}] Provisioning for user '{}' -> Target: '{}'",
                current_index, total_repos, plan.full_target_repo, plan.template_repo
            ),
        );

        if config.dry_run {
            emit_log(
                &app_handle,
                "warn",
                &format!(
                    "  [DRY-RUN - WOULD CREATE] Repo: {}, Template: {}, Deadline: {}",
                    plan.full_target_repo, plan.template_repo, plan.deadline
                ),
            );

            let _ = app_handle.emit(
                "repo-provision-progress",
                RepoProvisionProgress {
                    current_index,
                    total: total_repos,
                    username: plan.username.clone(),
                    target_repo: plan.full_target_repo.clone(),
                    template_repo: plan.template_repo.clone(),
                    step: "Complete".to_string(),
                    status: "WOULD CREATE (DRY-RUN)".to_string(),
                    error: None,
                    is_dry_run: true,
                },
            );

            success_count += 1;
        } else {
            let client_ref = client.as_ref().unwrap();

            // Extract template owner and repo
            let template_parts: Vec<&str> = plan.template_repo.split('/').collect();
            if template_parts.len() < 2 {
                let err_msg = format!("Invalid template repository: {}", plan.template_repo);
                emit_log(&app_handle, "error", &err_msg);
                let _ = app_handle.emit(
                    "repo-provision-progress",
                    RepoProvisionProgress {
                        current_index,
                        total: total_repos,
                        username: plan.username.clone(),
                        target_repo: plan.full_target_repo.clone(),
                        template_repo: plan.template_repo.clone(),
                        step: "Validation".to_string(),
                        status: "FAILED".to_string(),
                        error: Some(err_msg),
                        is_dry_run: false,
                    },
                );
                failed_count += 1;
                continue;
            }

            let t_owner = template_parts[0];
            let t_repo = template_parts[1];

            // 1. Generate repository from template
            emit_log(
                &app_handle,
                "info",
                &format!("  1. Generating repo from template '{}'...", plan.template_repo),
            );
            if let Err(e) = client_ref
                .generate_repo_from_template(t_owner, t_repo, &config.org_name, &plan.target_repo_name)
                .await
            {
                emit_log(&app_handle, "error", &format!("  [FAILED TO CREATE]: {}", e));
                let _ = app_handle.emit(
                    "repo-provision-progress",
                    RepoProvisionProgress {
                        current_index,
                        total: total_repos,
                        username: plan.username.clone(),
                        target_repo: plan.full_target_repo.clone(),
                        template_repo: plan.template_repo.clone(),
                        step: "Create Repo".to_string(),
                        status: "FAILED".to_string(),
                        error: Some(e),
                        is_dry_run: false,
                    },
                );
                failed_count += 1;
                continue;
            }

            // Wait for repository initialization
            emit_log(&app_handle, "info", "  Waiting for repo initialization...");
            let default_branch = client_ref
                .wait_for_repo(&config.org_name, &plan.target_repo_name, 15)
                .await
                .unwrap_or_else(|_| "main".to_string());

            // 2. Assign Write Access to User ('push')
            emit_log(
                &app_handle,
                "info",
                &format!("  2. Assigning 'push' permission to '{}'...", plan.username),
            );
            let _ = client_ref
                .add_collaborator(&config.org_name, &plan.target_repo_name, &plan.username, "push")
                .await;

            // 2b. Assign Access to Reviewers ('maintain')
            for rev in &config.reviewers {
                let rev_clean = rev.trim();
                if !rev_clean.is_empty() {
                    emit_log(
                        &app_handle,
                        "info",
                        &format!("  Assigning 'maintain' permission to reviewer '{}'...", rev_clean),
                    );
                    let _ = client_ref
                        .add_collaborator(
                            &config.org_name,
                            &plan.target_repo_name,
                            rev_clean,
                            "maintain",
                        )
                        .await;
                }
            }

            // 3. Update description with deadline
            emit_log(&app_handle, "info", "  3. Updating repo description with deadline...");
            let desc = format!(
                "Assignment Repository for {}. Deadline: {}",
                plan.username, plan.deadline
            );
            let _ = client_ref
                .update_repo_description(&config.org_name, &plan.target_repo_name, &desc)
                .await;

            // 4. Create Milestone with Deadline
            emit_log(&app_handle, "info", "  4. Creating milestone 'Assignment Deadline'...");
            let milestone_num = client_ref
                .create_milestone(
                    &config.org_name,
                    &plan.target_repo_name,
                    "Assignment Deadline",
                    &plan.deadline_iso,
                )
                .await
                .ok();

            sleep(Duration::from_millis(500)).await;

            // 5. Create Issue linked to Milestone
            if let Some(m_num) = milestone_num {
                emit_log(&app_handle, "info", "  5. Creating issue linked to milestone...");
                let issue_body = format!(
                    "Hi @{}, please be reminded that the deadline for this assignment is **{}**.",
                    plan.username, plan.deadline
                );
                let _ = client_ref
                    .create_issue(
                        &config.org_name,
                        &plan.target_repo_name,
                        "Assignment Deadline",
                        &issue_body,
                        Some(m_num),
                    )
                    .await;
            }

            // 6. Create Feedback Pull Request
            emit_log(&app_handle, "info", "  6. Setting up feedback branch and PR...");
            match client_ref
                .get_branch_sha(&config.org_name, &plan.target_repo_name, &default_branch)
                .await
            {
                Ok(sha) => {
                    // Create branch refs/heads/feedback
                    let _ = client_ref
                        .create_branch(&config.org_name, &plan.target_repo_name, "feedback", &sha)
                        .await;

                    // Add .github/FEEDBACK_HINT.md
                    let hint_text = "This Pull Request is created for feedback purposes.";
                    let hint_b64 = BASE64_STANDARD.encode(hint_text);
                    let _ = client_ref
                        .create_file(
                            &config.org_name,
                            &plan.target_repo_name,
                            ".github/FEEDBACK_HINT.md",
                            "Setup feedback PR",
                            &hint_b64,
                            &default_branch,
                        )
                        .await;

                    // Create PR
                    let pr_body = format!(
                        "Hi @{}, this Pull Request is created for your feedback and grading. Please do not close this PR.",
                        plan.username
                    );
                    match client_ref
                        .create_pull_request(
                            &config.org_name,
                            &plan.target_repo_name,
                            "Feedback",
                            &pr_body,
                            &default_branch,
                            "feedback",
                        )
                        .await
                    {
                        Ok(pr_num) => {
                            if !config.reviewers.is_empty() {
                                let _ = client_ref
                                    .request_reviewers(
                                        &config.org_name,
                                        &plan.target_repo_name,
                                        pr_num,
                                        &config.reviewers,
                                    )
                                    .await;
                            }
                        }
                        Err(e) => {
                            emit_log(&app_handle, "warn", &format!("  Feedback PR creation notice: {}", e));
                        }
                    }
                }
                Err(e) => {
                    emit_log(&app_handle, "warn", &format!("  Could not resolve default branch SHA: {}", e));
                }
            }

            emit_log(
                &app_handle,
                "success",
                &format!("  [SUCCESSFULLY PROVISIONED] {}", plan.full_target_repo),
            );

            let _ = app_handle.emit(
                "repo-provision-progress",
                RepoProvisionProgress {
                    current_index,
                    total: total_repos,
                    username: plan.username.clone(),
                    target_repo: plan.full_target_repo.clone(),
                    template_repo: plan.template_repo.clone(),
                    step: "Complete".to_string(),
                    status: "SUCCESS".to_string(),
                    error: None,
                    is_dry_run: false,
                },
            );

            success_count += 1;
        }
    }

    emit_log(
        &app_handle,
        "info",
        &format!(
            "Repository provisioning finished. Successful: {}, Failed: {}",
            success_count, failed_count
        ),
    );

    Ok(ProvisionSummary {
        total_users,
        total_repos,
        success_count,
        failed_count,
        is_dry_run: config.dry_run,
    })
}

const SAMPLE_USERS_FALLBACK: &str = include_str!("../../../001_xdummy_users.csv");
const SAMPLE_ASSIGNMENTS_FALLBACK: &str = include_str!("../../../002_xdummy_assignments.csv");

/// Reads sample files from disk if available, or falls back to embedded sample data
#[tauri::command]
pub fn load_sample_file(sample_type: String) -> Result<String, String> {
    match sample_type.as_str() {
        "team_invitations" => {
            let candidates = [
                "001_xdummy_users.csv",
                "../001_xdummy_users.csv",
                "../../001_xdummy_users.csv",
                "../../../001_xdummy_users.csv",
            ];
            for path in candidates {
                if let Ok(content) = std::fs::read_to_string(path) {
                    if !content.trim().is_empty() {
                        return Ok(content);
                    }
                }
            }
            Ok(SAMPLE_USERS_FALLBACK.to_string())
        }
        "repo_assignments" => {
            let candidates = [
                "002_xdummy_assignments.csv",
                "../002_xdummy_assignments.csv",
                "../../002_xdummy_assignments.csv",
                "../../../002_xdummy_assignments.csv",
            ];
            for path in candidates {
                if let Ok(content) = std::fs::read_to_string(path) {
                    if !content.trim().is_empty() {
                        return Ok(content);
                    }
                }
            }
            Ok(SAMPLE_ASSIGNMENTS_FALLBACK.to_string())
        }
        _ => Err(format!("Unknown sample type: {}", sample_type)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_sample_file() {
        let team_sample = load_sample_file("team_invitations".to_string()).expect("Should load team invitations sample");
        assert!(team_sample.contains("octocat"));
        assert!(team_sample.contains("Phase 1 - Set 1"));

        let repo_sample = load_sample_file("repo_assignments".to_string()).expect("Should load repo assignments sample");
        assert!(repo_sample.contains("user1"));
        assert!(repo_sample.contains("FTDS-045-HCK"));
    }
}
