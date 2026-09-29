use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamInvitationEntry {
    pub username: String,
    pub original_teams: String,
    pub teams: Vec<TeamItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamItem {
    pub display_name: String,
    pub slug: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateEntry {
    pub key: Option<String>,
    pub repo: String,     // e.g. "ORGANIZATION-NAME/P0-LC1-Set-1"
    pub deadline: String, // e.g. "2026-12-31 23:59"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedTemplateInfo {
    pub full_repo: String,
    pub deadline: String,
    pub clean_repo_name: String,
    pub org: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvisionPlan {
    pub username: String,
    pub prefix: String,
    pub template_repo: String,
    pub deadline: String,
    pub deadline_iso: String,
    pub target_repo_name: String,
    pub full_target_repo: String,
}

/// Normalizes a team name into a GitHub team slug, identical to:
/// `echo "$TEAM_NAME" | tr '[:upper:]' '[:lower:]' | sed -E 's/[^a-z0-9]+/-/g' | sed -E 's/^-|-$//g'`
pub fn normalize_team_slug(name: &str) -> String {
    let lower = name.trim().to_lowercase();
    let mut slug = String::new();
    let mut last_dash = false;
    for c in lower.chars() {
        if c.is_ascii_alphanumeric() {
            slug.push(c);
            last_dash = false;
        } else if !last_dash && !slug.is_empty() {
            slug.push('-');
            last_dash = true;
        }
    }
    if slug.ends_with('-') {
        slug.pop();
    }
    slug
}

/// Normalizes a template repo name into a clean repo name by stripping `template` affix,
/// matching: `sed -E 's/(^|[-_])template([-_]|$)/\1/g; s/^[-_]//; s/[-_]$//'`
pub fn clean_template_name(repo_basename: &str) -> String {
    let re = Regex::new(r"(?i)(^|[-_])template([-_]|$)").unwrap();
    let replaced = re.replace_all(repo_basename, "$1");
    let re_dashes = Regex::new(r"[-_]{2,}").unwrap();
    let cleaned = re_dashes.replace_all(&replaced, "-");
    cleaned.trim_matches(|c| c == '-' || c == '_').to_string()
}

/// Formats deadline to ISO 8601 with Asia/Jakarta (+07:00) timezone if in `YYYY-MM-DD HH:MM` or `YYYY-MM-DDTHH:MM` format
pub fn format_deadline_iso(deadline: &str) -> String {
    let trimmed = deadline.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let re_datetime = Regex::new(r"^(\d{4}-\d{2}-\d{2})[\sT](\d{2}:\d{2})(:\d{2})?$").unwrap();
    let re_date_only = Regex::new(r"^(\d{4}-\d{2}-\d{2})$").unwrap();
    if let Some(caps) = re_datetime.captures(trimmed) {
        format!("{}T{}:00+07:00", &caps[1], &caps[2])
    } else if let Some(caps) = re_date_only.captures(trimmed) {
        format!("{}T23:59:00+07:00", &caps[1])
    } else {
        trimmed.to_string()
    }
}

/// Parses team invitation input (CSV, pipe-separated, or lines from 001_invite_teams.sh)
pub fn parse_team_invitations(input: &str) -> Vec<TeamInvitationEntry> {
    let mut results = Vec::new();

    for raw_line in input.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let (username, raw_teams) = if line.contains('|') {
            let mut parts = line.splitn(2, '|');
            let u = parts.next().unwrap_or("").trim();
            let t = parts.next().unwrap_or("").trim();
            (u, t)
        } else if line.contains(',') {
            let mut parts = line.splitn(2, ',');
            let u = parts.next().unwrap_or("").trim();
            let t = parts.next().unwrap_or("").trim();
            (u, t)
        } else {
            (line, "")
        };

        if username.is_empty() {
            continue;
        }

        let mut team_items = Vec::new();
        for t in raw_teams.split(',') {
            let t_trim = t.trim();
            if !t_trim.is_empty() {
                team_items.push(TeamItem {
                    display_name: t_trim.to_string(),
                    slug: normalize_team_slug(t_trim),
                });
            }
        }

        results.push(TeamInvitationEntry {
            username: username.to_string(),
            original_teams: raw_teams.to_string(),
            teams: team_items,
        });
    }

    results
}

/// Resolves template repo info from query string against the catalog, matching resolve_template_info in 002_create_repos.sh
pub fn resolve_template_info(
    query: &str,
    catalog: &[TemplateEntry],
    org_name: &str,
) -> ResolvedTemplateInfo {
    let query_clean = query.trim();
    let mut matched_repo = String::new();
    let mut matched_deadline = String::new();

    for t_entry in catalog {
        let cand_repo = t_entry.repo.trim();
        let cand_base = cand_repo.split('/').nth(1).unwrap_or(cand_repo).trim();
        let cand_clean = clean_template_name(cand_base);
        let cand_key = t_entry.key.as_deref().unwrap_or("").trim();

        if query_clean == cand_repo
            || query_clean == cand_base
            || query_clean == cand_clean
            || (!cand_key.is_empty() && query_clean == cand_key)
        {
            matched_repo = cand_repo.to_string();
            matched_deadline = t_entry.deadline.trim().to_string();
            break;
        }
    }

    if matched_repo.is_empty() {
        if query_clean.contains('/') {
            matched_repo = query_clean.to_string();
        } else if !org_name.trim().is_empty() {
            matched_repo = format!("{}/{}", org_name.trim(), query_clean);
        } else {
            matched_repo = query_clean.to_string();
        }
    }

    let org = if matched_repo.contains('/') {
        matched_repo
            .split('/')
            .next()
            .unwrap_or("")
            .trim()
            .to_string()
    } else {
        org_name.trim().to_string()
    };

    let repo_basename = if matched_repo.contains('/') {
        matched_repo
            .split('/')
            .nth(1)
            .unwrap_or(&matched_repo)
            .trim()
            .to_string()
    } else {
        matched_repo.clone()
    };

    let clean_repo = clean_template_name(&repo_basename);

    ResolvedTemplateInfo {
        full_repo: matched_repo,
        deadline: matched_deadline,
        clean_repo_name: clean_repo,
        org,
    }
}

/// Parses raw user assignments and generates ProvisionPlan list, replicating 002_create_repos.sh
pub fn parse_repo_assignments(
    input: &str,
    catalog: &[TemplateEntry],
    default_org: &str,
) -> Vec<ProvisionPlan> {
    // Stage 1: Collect raw (username, prefix, templates)
    let prefix_regex = Regex::new(r"^((BATCH|COHORT|CLASS|PREFIX)-\d{3}-(DEV|REM|ONL|[A-Za-z0-9]+)|(BATCH|COHORT|CLASS|PREFIX)-|[A-Za-z0-9]+-\d{3}-[A-Za-z0-9]+)").unwrap();
    let date_regex = Regex::new(r"^\d{4}-\d{2}-\d{2}").unwrap();

    let mut raw_assignments: Vec<(String, String, String)> = Vec::new();
    let mut dynamic_catalog = catalog.to_vec();

    for raw_line in input.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if line.contains('|') {
            let parts: Vec<&str> = line.split('|').map(|s| s.trim()).collect();
            if parts.len() >= 3 && date_regex.is_match(parts[1]) {
                // Template-centric: template|deadline|user1,user2,...
                let t_repo = parts[0];
                let t_deadline = parts[1];
                dynamic_catalog.push(TemplateEntry {
                    key: None,
                    repo: t_repo.to_string(),
                    deadline: t_deadline.to_string(),
                });

                for u_entry in parts[2].split(',') {
                    let u_trim = u_entry.trim();
                    if u_trim.is_empty() {
                        continue;
                    }
                    let (u_name, u_pref) = if u_trim.contains(':') {
                        let mut sub = u_trim.splitn(2, ':');
                        (sub.next().unwrap().trim(), sub.next().unwrap().trim())
                    } else if u_trim.contains('|') {
                        let mut sub = u_trim.splitn(2, '|');
                        (sub.next().unwrap().trim(), sub.next().unwrap().trim())
                    } else {
                        (u_trim, "")
                    };
                    raw_assignments.push((
                        u_name.to_string(),
                        u_pref.to_string(),
                        t_repo.to_string(),
                    ));
                }
            } else if parts.len() >= 3 {
                let u = parts[0].to_string();
                let p = parts[1].to_string();
                let t = parts[2..].join(",");
                raw_assignments.push((u, p, t));
            } else if parts.len() == 2 {
                let u = parts[0].to_string();
                if prefix_regex.is_match(parts[1]) {
                    raw_assignments.push((u, parts[1].to_string(), "all".to_string()));
                } else {
                    raw_assignments.push((u, String::new(), parts[1].to_string()));
                }
            } else {
                raw_assignments.push((parts[0].to_string(), String::new(), "all".to_string()));
            }
        } else if line.contains(',') {
            let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
            let u = parts[0].to_string();
            if parts.len() > 1 && prefix_regex.is_match(parts[1]) {
                let p = parts[1].to_string();
                let t = if parts.len() > 2 {
                    parts[2..].join(",")
                } else {
                    "all".to_string()
                };
                raw_assignments.push((u, p, t));
            } else {
                let p = String::new();
                let t = if parts.len() > 1 {
                    parts[1..].join(",")
                } else {
                    "all".to_string()
                };
                raw_assignments.push((u, p, t));
            }
        } else {
            raw_assignments.push((line.to_string(), String::new(), "all".to_string()));
        }
    }

    // Stage 2: Deduplicate unique users in order
    let mut unique_users = Vec::new();
    for (u, _, _) in &raw_assignments {
        if !u.is_empty() && !unique_users.contains(u) {
            unique_users.push(u.clone());
        }
    }

    // Default template keys for wildcard "all"
    let all_catalog_keys: Vec<String> = dynamic_catalog.iter().map(|t| t.repo.clone()).collect();

    // Stage 3: Resolve plans for each user
    let mut plans = Vec::new();

    for user in unique_users {
        let mut user_prefix = String::new();
        let mut user_templates = Vec::new();

        for (u, p, t_raw) in &raw_assignments {
            if u == &user {
                if !p.is_empty() {
                    user_prefix = p.clone();
                }

                let t_candidates: Vec<String> =
                    if t_raw == "all" || t_raw == "*" || t_raw.is_empty() {
                        all_catalog_keys.clone()
                    } else {
                        t_raw
                            .split(',')
                            .map(|s| s.trim().to_string())
                            .filter(|s| !s.is_empty())
                            .collect()
                    };

                for t_item in t_candidates {
                    if !user_templates.contains(&t_item) {
                        user_templates.push(t_item);
                    }
                }
            }
        }

        for t_query in user_templates {
            let resolved =
                resolve_template_info(&t_query, &dynamic_catalog, default_org);
            let target_repo_name = if !user_prefix.is_empty() {
                format!("{}-{}-{}", resolved.clean_repo_name, user_prefix, user)
            } else {
                format!("{}-{}", resolved.clean_repo_name, user)
            };
            let full_target_repo = format!("{}/{}", resolved.org, target_repo_name);
            let deadline_iso = format_deadline_iso(&resolved.deadline);

            plans.push(ProvisionPlan {
                username: user.clone(),
                prefix: user_prefix.clone(),
                template_repo: resolved.full_repo,
                deadline: resolved.deadline,
                deadline_iso,
                target_repo_name,
                full_target_repo,
            });
        }
    }

    plans
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slug_generation() {
        assert_eq!(normalize_team_slug("Phase 1 - Set 1"), "phase-1-set-1");
        assert_eq!(normalize_team_slug("phase-1-set-1"), "phase-1-set-1");
        assert_eq!(normalize_team_slug("  Phase  3 - Set 2  "), "phase-3-set-2");
        assert_eq!(normalize_team_slug("Team Alpha #1!"), "team-alpha-1");
    }

    #[test]
    fn test_clean_template_name() {
        assert_eq!(clean_template_name("P0-LC1-Set-1"), "P0-LC1-Set-1");
        assert_eq!(clean_template_name("template-P0-LC1"), "P0-LC1");
        assert_eq!(clean_template_name("P0-LC1-template"), "P0-LC1");
    }

    #[test]
    fn test_deadline_iso_formatting() {
        assert_eq!(
            format_deadline_iso("2026-12-31 23:59"),
            "2026-12-31T23:59:00+07:00"
        );
        assert_eq!(
            format_deadline_iso("2026-12-31T23:59"),
            "2026-12-31T23:59:00+07:00"
        );
        assert_eq!(
            format_deadline_iso("2026-12-31"),
            "2026-12-31T23:59:00+07:00"
        );
        assert_eq!(
            format_deadline_iso("2026-12-31T23:59:00Z"),
            "2026-12-31T23:59:00Z"
        );
    }

    #[test]
    fn test_parse_team_invitations() {
        let input = r#"
# Comment line
octocat,Phase 1 - Set 1
johndoe,Phase 1 - Set 1,Phase 2 - Set 1
janedoe,phase-1-set-1,phase-2-set-1
"#;
        let entries = parse_team_invitations(input);
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].username, "octocat");
        assert_eq!(entries[0].teams.len(), 1);
        assert_eq!(entries[0].teams[0].slug, "phase-1-set-1");
        assert_eq!(entries[1].username, "johndoe");
        assert_eq!(entries[1].teams.len(), 2);
        assert_eq!(entries[1].teams[1].slug, "phase-2-set-1");
    }

    #[test]
    fn test_parse_repo_assignments() {
        let catalog = vec![
            TemplateEntry {
                key: None,
                repo: "ORGANIZATION-NAME/P0-LC1-Set-1".to_string(),
                deadline: "2026-12-31T23:59".to_string(),
            },
            TemplateEntry {
                key: None,
                repo: "ORGANIZATION-NAME/P0-LC2-Set-1".to_string(),
                deadline: "2026-12-31T23:59".to_string(),
            },
        ];

        let input = "user1,BATCH-045-DEV,P0-LC1-Set-1,P0-LC2-Set-1";
        let plans = parse_repo_assignments(
            input,
            &catalog,
            "ORGANIZATION-NAME",
        );

        assert_eq!(plans.len(), 2);
        assert_eq!(plans[0].username, "user1");
        assert_eq!(plans[0].prefix, "BATCH-045-DEV");
        assert_eq!(
            plans[0].target_repo_name,
            "P0-LC1-Set-1-BATCH-045-DEV-user1"
        );
        assert_eq!(
            plans[0].full_target_repo,
            "ORGANIZATION-NAME/P0-LC1-Set-1-BATCH-045-DEV-user1"
        );
        assert_eq!(plans[0].deadline_iso, "2026-12-31T23:59:00+07:00");

        // Test without prefix on user assignment
        let input_no_prefix = "user2,P0-LC1-Set-1";
        let plans_no_prefix = parse_repo_assignments(
            input_no_prefix,
            &catalog,
            "ORGANIZATION-NAME",
        );
        assert_eq!(plans_no_prefix.len(), 1);
        assert_eq!(plans_no_prefix[0].target_repo_name, "P0-LC1-Set-1-user2");
    }
}
