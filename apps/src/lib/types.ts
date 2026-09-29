export interface GitHubUser {
  login: string;
  id: number;
  name: string | null;
  avatar_url: string | null;
  html_url: string;
}

export interface TeamItem {
  display_name: string;
  slug: string;
}

export interface TeamInvitationEntry {
  username: string;
  original_teams: string;
  teams: TeamItem[];
}

export interface TeamInviteProgress {
  current_index: number;
  total: number;
  username: string;
  team_name: string;
  team_slug: string;
  status: string;
  error: string | null;
  is_dry_run: boolean;
}

export interface InvitationSummary {
  total_users: number;
  total_invitations: number;
  success_count: number;
  failed_count: number;
  is_dry_run: boolean;
}

export interface TemplateEntry {
  key: string | null;
  repo: string;
  deadline: string;
}

export interface ProvisionPlan {
  username: string;
  prefix: string;
  template_repo: string;
  deadline: string;
  deadline_iso: string;
  target_repo_name: string;
  full_target_repo: string;
}

export interface RepoProvisionConfig {
  org_name: string;
  reviewers: string[];
  dry_run: boolean;
}

export interface RepoProvisionProgress {
  current_index: number;
  total: number;
  username: string;
  target_repo: string;
  template_repo: string;
  step: string;
  status: string;
  error: string | null;
  is_dry_run: boolean;
}

export interface ProvisionSummary {
  total_users: number;
  total_repos: number;
  success_count: number;
  failed_count: number;
  is_dry_run: boolean;
}

export interface LogMessage {
  timestamp: string;
  level: "info" | "success" | "warn" | "error";
  message: string;
}
