#!/usr/bin/env bash
#
# 002_create_repos.sh - Provision assignment repositories for GitHub users from template repositories.
#
# Usage:
#   1. Batch mode using USERS and TEMPLATES inside this script:
#        ./002_create_repos.sh
#        ./002_create_repos.sh --dry-run
#
#   2. CLI Arguments mode (single or multiple users with specific templates and prefixes):
#        ./002_create_repos.sh -u "user1|BATCH-045-DEV" -t "P0-LC1-Set-1,P0-LC2-Set-1"
#        ./002_create_repos.sh -u "user1,user2" -p "BATCH-059-REM" -t "P0-LC2-Set-1"
#        ./002_create_repos.sh -u "user1|BATCH-045-DEV,user2|BATCH-059-REM" -t "P0-LC1-Set-1" --dry-run
#
#   3. File / CSV mode (username,prefix,template1,template2,...):
#        ./002_create_repos.sh --file assignments.csv
#        ./002_create_repos.sh --file assignments.csv --dry-run
#

set -uo pipefail

# ==============================================================================
# Global Configuration
# ==============================================================================
ORG_NAME="ORGANIZATION-NAME"
TEAM_NAME="" # Leave empty ("") if team sync is not needed
DEFAULT_DEADLINE="2026-12-31 23:59"

# Reviewers to assign with 'maintain' permission
REVIEWERS=(
  "reviewer1"
  "reviewer2"
)

# ==============================================================================
# Template Catalog (Batch Mode)
# Format options:
#   1. "organization/repository|YYYY-MM-DD HH:MM"
#   2. "KEY|organization/repository|YYYY-MM-DD HH:MM"
#   3. "organization/repository|YYYY-MM-DD HH:MM|user1,user2,..." (Template-centric mapping)
# ==============================================================================
TEMPLATES=(
  "ORGANIZATION-NAME/P0-LC1-Set-1|2026-12-31 23:59"
  "ORGANIZATION-NAME/P0-LC2-Set-1|2026-12-31 23:59"
  "ORGANIZATION-NAME/P0-LC3-Set-1|2026-12-31 23:59"
)

# ==============================================================================
# User Assignments (Batch Mode)
# Format options:
#   "username|prefix|template_1,template_2,..." -> Specific templates and prefix
#   "username|prefix"                           -> Prefix specified, all templates
#   "username||template_1,template_2,..."       -> Specific templates, fallback prefix
#   "username"                                  -> All templates, fallback prefix
#
# Prefix format: BATCH-XXX-YYY (e.g. "BATCH-045-DEV" or "BATCH-059-REM")
# ==============================================================================
USERS=(
  "user1|BATCH-045-DEV|P0-LC1-Set-1,P0-LC2-Set-1"
  "user2|BATCH-059-REM|P0-LC2-Set-1"
  "user3|BATCH-044-DEV|P0-LC1-Set-1,P0-LC3-Set-1"
)

# ANSI Color codes
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m' # No Color

# Helper: Get all template keys / repositories as comma-separated string
get_all_template_keys() {
  local keys=()
  if [ ${#TEMPLATES[@]} -gt 0 ]; then
    for t_entry in "${TEMPLATES[@]}"; do
      local clean_entry
      clean_entry=$(echo "$t_entry" | sed 's/,$//')
      local field1 field2 t_repo
      field1=$(echo "$clean_entry" | cut -d'|' -f1 | xargs)
      field2=$(echo "$clean_entry" | cut -d'|' -f2 | xargs)
      if [[ "$field1" == *"/"* ]]; then
        t_repo="$field1"
      elif [[ "$field2" == *"/"* ]]; then
        t_repo="$field2"
      else
        t_repo="$field1"
      fi
      [ -n "$t_repo" ] && keys+=("$t_repo")
    done
  fi
  (IFS=,; echo "${keys[*]}")
}

# Helper: Find template repository, deadline, clean name
# Returns: "FULL_REPO|DEADLINE|CLEAN_REPO_NAME|ORG"
resolve_template_info() {
  local query="$1"
  local matched_repo=""
  local matched_deadline=""

  if [ ${#TEMPLATES[@]} -gt 0 ]; then
    for t_entry in "${TEMPLATES[@]}"; do
      local clean_entry
      clean_entry=$(echo "$t_entry" | sed 's/,$//')

      local f1 f2 f3
      f1=$(echo "$clean_entry" | cut -d'|' -f1 | xargs)
      f2=$(echo "$clean_entry" | cut -d'|' -f2 | xargs)
      f3=$(echo "$clean_entry" | cut -d'|' -f3 2>/dev/null | xargs || true)

      local cand_repo=""
      local cand_deadline=""
      local cand_key=""

      if [[ "$f1" == *"/"* ]]; then
        cand_repo="$f1"
        cand_deadline="$f2"
      elif [[ "$f2" == *"/"* ]]; then
        cand_key="$f1"
        cand_repo="$f2"
        cand_deadline="$f3"
      else
        cand_repo="$f1"
        cand_deadline="$f2"
      fi

      local cand_base cand_clean
      cand_base=$(echo "$cand_repo" | cut -d'/' -f2)
      [ -z "$cand_base" ] && cand_base="$cand_repo"
      cand_clean=$(echo "$cand_base" | sed -E 's/(^|[-_])template([-_]|$)/\1/g; s/^[-_]//; s/[-_]$//')

      if [ "$query" = "$cand_repo" ] || [ "$query" = "$cand_base" ] || [ "$query" = "$cand_clean" ] || ([ -n "$cand_key" ] && [ "$query" = "$cand_key" ]); then
        matched_repo="$cand_repo"
        matched_deadline="$cand_deadline"
        break
      fi
    done
  fi

  # Fallback if not found in catalog
  if [ -z "$matched_repo" ]; then
    if [[ "$query" == *"/"* ]]; then
      matched_repo="$query"
    elif [ -n "$ORG_NAME" ]; then
      matched_repo="${ORG_NAME}/${query}"
    else
      matched_repo="$query"
    fi
    matched_deadline="${CLI_DEADLINE:-$DEFAULT_DEADLINE}"
  fi

  if [ -z "$matched_deadline" ]; then
    matched_deadline="${CLI_DEADLINE:-$DEFAULT_DEADLINE}"
  elif [ -n "${CLI_DEADLINE:-}" ]; then
    matched_deadline="$CLI_DEADLINE"
  fi

  local org repo_basename clean_repo_name
  org=$(echo "$matched_repo" | cut -d'/' -f1)
  repo_basename=$(echo "$matched_repo" | cut -d'/' -f2)
  [ -z "$repo_basename" ] && repo_basename="$matched_repo"
  clean_repo_name=$(echo "$repo_basename" | sed -E 's/(^|[-_])template([-_]|$)/\1/g; s/^[-_]//; s/[-_]$//')

  echo "${matched_repo}|${matched_deadline}|${clean_repo_name}|${org}"
}

# Helper: Parse a raw user assignment string into "USERNAME|PREFIX|TEMPLATES"
parse_user_entry() {
  local entry="$1"
  local fallback_prefix="${2:-}"
  local u="" p="" t=""
  local count
  count=$(echo "$entry" | awk -F'|' '{print NF}')

  if [ "$count" -ge 3 ]; then
    u=$(echo "$entry" | cut -d'|' -f1 | xargs)
    p=$(echo "$entry" | cut -d'|' -f2 | xargs)
    t=$(echo "$entry" | cut -d'|' -f3- | xargs)
    [ -z "$p" ] && p="$fallback_prefix"
  elif [ "$count" -eq 2 ]; then
    u=$(echo "$entry" | cut -d'|' -f1 | xargs)
    local f2
    f2=$(echo "$entry" | cut -d'|' -f2 | xargs)
    if [[ "$f2" =~ ^(BATCH|COHORT|CLASS)-[0-9]{3}-(DEV|REM)$ ]] || [[ "$f2" =~ ^[A-Za-z0-9]+-[0-9]{3}-[A-Za-z0-9]+$ ]] || [[ "$f2" =~ ^(BATCH|COHORT|CLASS)- ]]; then
      p="$f2"
      t="all"
    else
      p="$fallback_prefix"
      t="$f2"
    fi
  else
    u=$(echo "$entry" | xargs)
    p="$fallback_prefix"
    t="all"
  fi

  [ -z "$t" ] && t="all"
  echo "${u}|${p}|${t}"
}

# 1. Validasi keberadaan GitHub CLI (gh)
if ! command -v gh &>/dev/null; then
  echo -e "${RED}Error: GitHub CLI ('gh') tidak ditemukan. Silakan pasang GitHub CLI terlebih dahulu.${NC}"
  echo -e "macOS: brew install gh"
  exit 1
fi

# 2. Validasi autentikasi GitHub CLI
if ! gh auth status &>/dev/null; then
  echo -e "${RED}Error: GitHub CLI ('gh') belum terautentikasi. Silakan jalankan 'gh auth login'.${NC}"
  exit 1
fi

# Parse CLI Arguments
DRY_RUN=false
CLI_USERNAME=""
CLI_TEMPLATES=""
CLI_DEADLINE=""
CLI_PREFIX=""
CLI_TEAM=""
CLI_REVIEWERS=""
CLI_ORG=""
INPUT_FILE=""
SKIP_SYNC=false

while [[ $# -gt 0 ]]; do
  case "$1" in
    --dry-run)
      DRY_RUN=true
      shift
      ;;
    -u|--username|--users)
      CLI_USERNAME="$2"
      shift 2
      ;;
    -t|--templates)
      CLI_TEMPLATES="$2"
      shift 2
      ;;
    -d|--deadline)
      CLI_DEADLINE="$2"
      shift 2
      ;;
    -p|--prefix)
      CLI_PREFIX="$2"
      shift 2
      ;;
    --team)
      CLI_TEAM="$2"
      shift 2
      ;;
    -r|--reviewers)
      CLI_REVIEWERS="$2"
      shift 2
      ;;
    -o|--org)
      CLI_ORG="$2"
      shift 2
      ;;
    -f|--file)
      INPUT_FILE="$2"
      shift 2
      ;;
    --skip-sync)
      SKIP_SYNC=true
      shift
      ;;
    -h|--help)
      echo "Usage: ./002_create_repos.sh [OPTIONS]"
      echo ""
      echo "Options:"
      echo "  -u, --username USER(S)     GitHub user(s) (format: 'user', 'user|prefix', or 'user|prefix|tpl1,tpl2')"
      echo "  -t, --templates TEMPLATES  Comma-separated templates (e.g. 'P0-LC1-Set-1,P0-LC2-Set-1')"
      echo "  -d, --deadline DEADLINE    Deadline override (format: 'YYYY-MM-DD HH:MM')"
      echo "  -p, --prefix PREFIX        Fallback repository prefix (e.g. 'BATCH-059-REM' or 'BATCH-045-DEV')"
      echo "      --team TEAM_NAME       GitHub team name for grouping members"
      echo "  -r, --reviewers REVIEWERS  Comma-separated list of reviewers (e.g. 'rev1,rev2')"
      echo "  -o, --org ORG_NAME         GitHub organization name override"
      echo "  -f, --file FILE_PATH       Path to CSV/TXT file with assignments"
      echo "      --skip-sync            Skip team and org membership synchronization"
      echo "      --dry-run              Simulate provisioning without calling GitHub API"
      echo "  -h, --help                 Show this help message"
      exit 0
      ;;
    *)
      echo -e "${RED}Unknown argument: $1${NC}"
      echo "Run with -h or --help for instructions."
      exit 1
      ;;
  esac
done

# Apply CLI overrides if provided
[ -n "$CLI_TEAM" ] && TEAM_NAME="$CLI_TEAM"
[ -n "$CLI_ORG" ] && ORG_NAME="$CLI_ORG"
if [ -n "$CLI_REVIEWERS" ]; then
  IFS="," read -r -a REVIEWERS <<< "$CLI_REVIEWERS"
fi

# Prepare raw assignments: array of "username|prefix|template1,template2,..."
RAW_ASSIGNMENTS=()

if [ -n "$CLI_USERNAME" ] || [ -n "$CLI_TEMPLATES" ]; then
  # CLI Mode
  if [ -n "$CLI_USERNAME" ]; then
    IFS="," read -r -a CLI_USERS_ARR <<< "$CLI_USERNAME"
    for cu in "${CLI_USERS_ARR[@]}"; do
      cu_trimmed=$(echo "$cu" | xargs)
      [ -z "$cu_trimmed" ] && continue

      parsed=$(parse_user_entry "$cu_trimmed" "$CLI_PREFIX")
      IFS="|" read -r p_user p_pref p_tpls <<< "$parsed"

      if [ -n "$CLI_TEMPLATES" ]; then
        RAW_ASSIGNMENTS+=("${p_user}|${p_pref}|${CLI_TEMPLATES}")
      else
        RAW_ASSIGNMENTS+=("${p_user}|${p_pref}|${p_tpls}")
      fi
    done
  elif [ -n "$CLI_TEMPLATES" ]; then
    # Only templates provided: assign these templates to all USERS configured in script
    if [ ${#USERS[@]} -eq 0 ]; then
      echo -e "${RED}Error: Parameter --username (-u) wajib disertakan jika array USERS kosong.${NC}"
      exit 1
    fi
    for u_item in "${USERS[@]}"; do
      parsed=$(parse_user_entry "$u_item" "$CLI_PREFIX")
      IFS="|" read -r p_user p_pref p_tpls <<< "$parsed"
      [ -n "$p_user" ] && RAW_ASSIGNMENTS+=("${p_user}|${p_pref}|${CLI_TEMPLATES}")
    done
  fi

elif [ -n "$INPUT_FILE" ]; then
  # File / CSV Mode
  if [ ! -f "$INPUT_FILE" ]; then
    echo -e "${RED}Error: File '${INPUT_FILE}' tidak ditemukan.${NC}"
    exit 1
  fi
  while IFS= read -r line || [ -n "$line" ]; do
    [[ "$line" =~ ^[[:space:]]*# ]] && continue
    [[ -z "${line// }" ]] && continue

    if [[ "$line" == *"|"* ]]; then
      # Check if line is template-centric: template|deadline|user1,user2
      pipe_count=$(echo "$line" | awk -F'|' '{print NF}')
      f1=$(echo "$line" | cut -d'|' -f1 | xargs)
      f2=$(echo "$line" | cut -d'|' -f2 | xargs)
      f3=$(echo "$line" | cut -d'|' -f3- | xargs)

      # If f1 is an org/repo or clean template name and f2 looks like a date/time
      if [ "$pipe_count" -ge 3 ] && [[ "$f2" =~ ^[0-9]{4}-[0-9]{2}-[0-9]{2} ]]; then
        TEMPLATES+=("${f1}|${f2}")
        IFS="," read -r -a u_split <<< "$f3"
        for us in "${u_split[@]}"; do
          us_trim=$(echo "$us" | xargs)
          [ -z "$us_trim" ] && continue
          # User could be "username:prefix" or "username|prefix" or "username"
          p_us=$(echo "$us_trim" | tr ':' '|' | cut -d'|' -f1 | xargs)
          p_pr=$(echo "$us_trim" | tr ':' '|' | cut -d'|' -f2 2>/dev/null | xargs || true)
          [ -z "$p_pr" ] && p_pr="$CLI_PREFIX"
          RAW_ASSIGNMENTS+=("${p_us}|${p_pr}|${f1}")
        done
      else
        parsed=$(parse_user_entry "$line" "$CLI_PREFIX")
        RAW_ASSIGNMENTS+=("$parsed")
      fi
    elif [[ "$line" == *","* ]]; then
      f1=$(echo "$line" | cut -d',' -f1 | xargs)
      f2=$(echo "$line" | cut -d',' -f2 | xargs)
      if [[ "$f2" =~ ^(BATCH|COHORT|CLASS)-[0-9]{3}-(DEV|REM)$ ]] || [[ "$f2" =~ ^[A-Za-z0-9]+-[0-9]{3}-[A-Za-z0-9]+$ ]] || [[ "$f2" =~ ^(BATCH|COHORT|CLASS)- ]]; then
        csv_pref="$f2"
        csv_tpls=$(echo "$line" | cut -d',' -f3- | xargs)
      else
        csv_pref="$CLI_PREFIX"
        csv_tpls=$(echo "$line" | cut -d',' -f2- | xargs)
      fi
      [ -z "$csv_tpls" ] && csv_tpls="all"
      RAW_ASSIGNMENTS+=("${f1}|${csv_pref}|${csv_tpls}")
    else
      # Single username without delimiter
      u_single=$(echo "$line" | xargs)
      RAW_ASSIGNMENTS+=("${u_single}|${CLI_PREFIX}|all")
    fi
  done < "$INPUT_FILE"

else
  # Batch Mode: Parse TEMPLATES and USERS from script variables

  # 1. Check if TEMPLATES has template-centric user assignments (e.g. repo|deadline|user1,user2)
  if [ ${#TEMPLATES[@]} -gt 0 ]; then
    for t_entry in "${TEMPLATES[@]}"; do
      clean_entry=$(echo "$t_entry" | sed 's/,$//')
      pipe_count=$(echo "$clean_entry" | awk -F'|' '{print NF}')
      if [ "$pipe_count" -ge 3 ]; then
        t_repo=$(echo "$clean_entry" | cut -d'|' -f1 | xargs)
        t_users=$(echo "$clean_entry" | cut -d'|' -f3- | xargs)
        IFS="," read -r -a u_split <<< "$t_users"
        for us in "${u_split[@]}"; do
          us_trim=$(echo "$us" | xargs)
          [ -z "$us_trim" ] && continue
          p_us=$(echo "$us_trim" | tr ':' '|' | cut -d'|' -f1 | xargs)
          p_pr=$(echo "$us_trim" | tr ':' '|' | cut -d'|' -f2 2>/dev/null | xargs || true)
          [ -z "$p_pr" ] && p_pr="$CLI_PREFIX"
          RAW_ASSIGNMENTS+=("${p_us}|${p_pr}|${t_repo}")
        done
      fi
    done
  fi

  # 2. Check USERS array
  if [ ${#USERS[@]} -gt 0 ]; then
    for u_entry in "${USERS[@]}"; do
      u_clean=$(echo "$u_entry" | xargs)
      [ -z "$u_clean" ] && continue
      parsed=$(parse_user_entry "$u_clean" "$CLI_PREFIX")
      RAW_ASSIGNMENTS+=("$parsed")
    done
  fi
fi

# Group and deduplicate templates and prefixes per user
ALL_USER_NAMES=()
if [ ${#RAW_ASSIGNMENTS[@]} -gt 0 ]; then
  for entry in "${RAW_ASSIGNMENTS[@]}"; do
    u=$(echo "$entry" | cut -d'|' -f1 | xargs)
    [ -n "$u" ] && ALL_USER_NAMES+=("$u")
  done
fi

# Unique user list preserving order
UNIQUE_USERS=()
if [ ${#ALL_USER_NAMES[@]} -gt 0 ]; then
  UNIQUE_USERS=($(printf "%s\n" "${ALL_USER_NAMES[@]}" | awk '!seen[$0]++'))
fi

TARGET_ASSIGNMENTS=()
if [ ${#UNIQUE_USERS[@]} -gt 0 ]; then
  for u in "${UNIQUE_USERS[@]}"; do
    user_prefix=""
    tpl_collector=()
    for entry in "${RAW_ASSIGNMENTS[@]}"; do
      curr_u=$(echo "$entry" | cut -d'|' -f1 | xargs)
      if [ "$curr_u" = "$u" ]; then
        curr_p=$(echo "$entry" | cut -d'|' -f2 | xargs)
        [ -n "$curr_p" ] && user_prefix="$curr_p"

        raw_tpls=$(echo "$entry" | cut -d'|' -f3- | xargs)
        if [ "$raw_tpls" = "all" ] || [ "$raw_tpls" = "*" ] || [ -z "$raw_tpls" ]; then
          raw_tpls=$(get_all_template_keys)
        fi
        IFS="," read -r -a t_split <<< "$raw_tpls"
        for t in "${t_split[@]}"; do
          t_trim=$(echo "$t" | xargs)
          [ -n "$t_trim" ] && tpl_collector+=("$t_trim")
        done
      fi
    done
    unique_tpls=($(printf "%s\n" "${tpl_collector[@]}" | awk '!seen[$0]++'))
    tpls_joined=$(IFS=,; echo "${unique_tpls[*]}")
    [ -n "$tpls_joined" ] && TARGET_ASSIGNMENTS+=("${u}|${user_prefix}|${tpls_joined}")
  done
fi

TOTAL_USERS=${#TARGET_ASSIGNMENTS[@]}

if [ "$TOTAL_USERS" -eq 0 ]; then
  echo -e "${YELLOW}Tidak ada data penugasan repositori untuk diproses.${NC}"
  echo -e "Silakan gunakan salah satu cara berikut:"
  echo -e "  1. Jalankan dengan argumen: ${BOLD}./002_create_repos.sh -u \"<username>|<prefix>\" -t \"P0-LC1-Set-1,P0-LC2-Set-1\"${NC}"
  echo -e "  2. Jalankan dengan file CSV: ${BOLD}./002_create_repos.sh --file assignments.csv${NC}"
  echo -e "  3. Isi array ${BOLD}USERS${NC} dan ${BOLD}TEMPLATES${NC} di dalam file skrip ini."
  exit 0
fi

# Display Header Banner
echo -e "${BOLD}${CYAN}=====================================================${NC}"
echo -e "${BOLD}${CYAN} 02. Repository Provisioning in GitHub Org: ${ORG_NAME}${NC}"
echo -e " Team:      ${BOLD}${TEAM_NAME:-'(None)'}${NC}"
echo -e " Reviewers: ${BOLD}${REVIEWERS[*]:-'(None)'}${NC}"
if [ -n "$CLI_PREFIX" ]; then
  echo -e " Fallback Prefix: ${BOLD}${CLI_PREFIX}${NC}"
fi
if [ "$DRY_RUN" = true ]; then
  echo -e "${YELLOW} >>> [DRY-RUN MODE ACTIVE] No changes will be written <<<${NC}"
fi
echo -e "${BOLD}${CYAN}=====================================================${NC}"
echo ""

# Step 1: Optional Team & Org Membership Synchronization
if [ "$SKIP_SYNC" = false ] && [ -n "$TEAM_NAME" ]; then
  SYNC_MEMBERS=()
  for entry in "${TARGET_ASSIGNMENTS[@]}"; do
    u=$(echo "$entry" | cut -d'|' -f1 | xargs)
    [ -n "$u" ] && SYNC_MEMBERS+=("$u")
  done
  if [ ${#REVIEWERS[@]} -gt 0 ]; then
    for r in "${REVIEWERS[@]}"; do
      [ -n "$r" ] && SYNC_MEMBERS+=("$r")
    done
  fi
  UNIQUE_SYNC_MEMBERS=()
  if [ ${#SYNC_MEMBERS[@]} -gt 0 ]; then
    UNIQUE_SYNC_MEMBERS=($(printf "%s\n" "${SYNC_MEMBERS[@]}" | awk '!seen[$0]++'))
  fi

  echo -e "${BOLD}${CYAN}-----------------------------------------------------${NC}"
  echo -e "${BOLD}Synchronizing Team Memberships: ${TEAM_NAME}${NC}"
  echo -e "${BOLD}${CYAN}-----------------------------------------------------${NC}"

  if [ "$DRY_RUN" = true ]; then
    echo -e "${YELLOW}[DRY-RUN] Would ensure team '${TEAM_NAME}' exists in org '${ORG_NAME}'${NC}"
    echo -e "${YELLOW}[DRY-RUN] Would sync ${#UNIQUE_SYNC_MEMBERS[@]} members: ${UNIQUE_SYNC_MEMBERS[*]:-}${NC}"
  else
    TEAM_ID=$(gh api "orgs/${ORG_NAME}/teams/${TEAM_NAME}" -q '.id' 2>/dev/null || true)
    if ! [[ "$TEAM_ID" =~ ^[0-9]+$ ]]; then
      echo "Team '${TEAM_NAME}' does not exist. Creating it..."
      TEAM_ID=$(gh api -X POST "orgs/${ORG_NAME}/teams" -f name="${TEAM_NAME}" -f privacy="closed" -q '.id' 2>/dev/null || true)
    fi

    for MEMBER in "${UNIQUE_SYNC_MEMBERS[@]}"; do
      printf "  -> Member: %-25s " "$MEMBER"
      USER_ID=$(gh api "users/$MEMBER" -q '.id' 2>/dev/null || true)
      if [ -n "$USER_ID" ] && [ "$USER_ID" != "null" ]; then
        if [ -n "$TEAM_ID" ] && [[ "$TEAM_ID" =~ ^[0-9]+$ ]]; then
          gh api -X POST "orgs/${ORG_NAME}/invitations" \
            -F invitee_id="$USER_ID" \
            -f role="direct_member" \
            -F "team_ids[]=$TEAM_ID" --silent 2>/dev/null \
            && echo -e "${GREEN}[INVITED/SYNCED]${NC}" \
            || echo -e "${CYAN}[ALREADY IN ORG/PENDING]${NC}"
        else
          gh api -X POST "orgs/${ORG_NAME}/invitations" \
            -F invitee_id="$USER_ID" \
            -f role="direct_member" --silent 2>/dev/null \
            && echo -e "${GREEN}[INVITED/SYNCED]${NC}" \
            || echo -e "${CYAN}[ALREADY IN ORG/PENDING]${NC}"
        fi
      else
        echo -e "${RED}[USER NOT FOUND]${NC}"
      fi
    done
  fi
  echo ""
fi

# Step 2: Provision Repositories
TOTAL_SUCCESS=0
TOTAL_FAILED=0
USER_INDEX=0

for entry in "${TARGET_ASSIGNMENTS[@]}"; do
  ((USER_INDEX++))
  IFS="|" read -r USERNAME USER_PREFIX TEMPLATES_RAW <<< "$entry"
  USERNAME=$(echo "$USERNAME" | xargs)
  USER_PREFIX=$(echo "$USER_PREFIX" | xargs)
  [ -z "$USERNAME" ] && continue

  echo -e "${BOLD}${CYAN}-----------------------------------------------------${NC}"
  if [ -n "$USER_PREFIX" ]; then
    echo -e "${BOLD}[${USER_INDEX}/${TOTAL_USERS}] User: ${USERNAME} (Prefix: ${USER_PREFIX})${NC}"
  else
    echo -e "${BOLD}[${USER_INDEX}/${TOTAL_USERS}] User: ${USERNAME}${NC}"
  fi
  echo -e "${BOLD}${CYAN}-----------------------------------------------------${NC}"

  # Validate prefix convention if prefix is provided
  if [ -n "$USER_PREFIX" ] && ! [[ "$USER_PREFIX" =~ ^[A-Za-z0-9]+-[0-9]{3}-[A-Za-z0-9]+$ ]] && ! [[ "$USER_PREFIX" =~ ^(BATCH|COHORT|CLASS)- ]]; then
    echo -e "  ${YELLOW}Notice: Prefix '${USER_PREFIX}' does not follow 'PREFIX-XXX-TAG' (e.g. BATCH-001-DEV) format.${NC}"
  fi

  IFS="," read -r -a USER_TEMPLATES_ARRAY <<< "$TEMPLATES_RAW"

  for template_query in "${USER_TEMPLATES_ARRAY[@]}"; do
    template_query=$(echo "$template_query" | xargs)
    [ -z "$template_query" ] && continue

    RESOLVED_INFO=$(resolve_template_info "$template_query")
    IFS="|" read -r TEMPLATE_REPO DEADLINE CLEAN_REPO_NAME ORG <<< "$RESOLVED_INFO"

    # Construct target repository name using user-specific prefix
    if [ -n "$USER_PREFIX" ]; then
      TARGET_REPO_NAME="${CLEAN_REPO_NAME}-${USER_PREFIX}-${USERNAME}"
    else
      TARGET_REPO_NAME="${CLEAN_REPO_NAME}-${USERNAME}"
    fi
    NEW_REPO="${ORG}/${TARGET_REPO_NAME}"

    # Format deadline for Milestone (ISO 8601 Asia/Jakarta UTC+07:00)
    if [[ "$DEADLINE" =~ ^[0-9]{4}-[0-9]{2}-[0-9]{2}[[:space:]][0-9]{2}:[0-9]{2}$ ]]; then
      DEADLINE_ISO=$(echo "$DEADLINE" | sed 's/ /T/')":00+07:00"
    else
      DEADLINE_ISO="$DEADLINE"
    fi

    printf "  -> Template: %-32s (Deadline: %s)\n" "$TEMPLATE_REPO" "$DEADLINE"
    printf "     Target:   %-32s\n" "$NEW_REPO"

    if [ "$DRY_RUN" = true ]; then
      echo -e "     Status:   ${YELLOW}[DRY-RUN - WOULD CREATE]${NC}"
      ((TOTAL_SUCCESS++))
    else
      # 1. Create Repository from Template
      echo "     Creating repository from template..."
      CREATE_OUT=$(gh repo create "$NEW_REPO" --template "$TEMPLATE_REPO" --private 2>&1)
      CREATE_EXIT=$?

      if [ $CREATE_EXIT -ne 0 ]; then
        echo -e "     ${RED}[FAILED TO CREATE]${NC} $CREATE_OUT"
        ((TOTAL_FAILED++))
        continue
      fi

      # Wait for GitHub repository initialization
      echo "     Waiting for repository initialization..."
      sleep 5

      # 2. Assign Write Access to User
      echo "     Assigning 'push' access to $USERNAME..."
      gh api -X PUT "repos/$NEW_REPO/collaborators/$USERNAME" -f permission=push --silent 2>&1 || true

      # 2b. Assign Access to Reviewers
      if [ ${#REVIEWERS[@]} -gt 0 ]; then
        for REV in "${REVIEWERS[@]}"; do
          [ -z "$REV" ] && continue
          echo "     Assigning 'maintain' access to reviewer: $REV..."
          gh api -X PUT "repos/$NEW_REPO/collaborators/$REV" -f permission=maintain --silent 2>&1 || true
        done
      fi

      # 3. Update Repository Description with Deadline
      echo "     Updating repository description..."
      gh repo edit "$NEW_REPO" --description "Assignment Repository for $USERNAME. Deadline: $DEADLINE" 2>&1 || true

      # 4. Create Milestone with Deadline
      echo "     Creating milestone 'Assignment Deadline'..."
      MILESTONE_NUMBER=$(gh api -X POST "repos/$NEW_REPO/milestones" \
        -f title="Assignment Deadline" \
        -f due_on="$DEADLINE_ISO" \
        -q '.number' 2>/dev/null || true)

      sleep 2

      # 5. Create Issue linked to Milestone
      if [ -n "$MILESTONE_NUMBER" ] && [ "$MILESTONE_NUMBER" != "null" ]; then
        echo "     Creating issue linked to milestone..."
        gh issue create \
          --repo "$NEW_REPO" \
          --title "Assignment Deadline" \
          --body "Hi @$USERNAME, please be reminded that the deadline for this assignment is **$DEADLINE**." \
          --milestone "Assignment Deadline" 2>&1 || true
      fi

      # 6. Create Feedback Pull Request
      echo "     Creating Feedback Pull Request..."
      DEFAULT_BRANCH=$(gh repo view "$NEW_REPO" --json defaultBranchRef -q .defaultBranchRef.name 2>/dev/null || echo "main")
      SHA=$(gh api "repos/$NEW_REPO/git/ref/heads/$DEFAULT_BRANCH" -q '.object.sha' 2>/dev/null || true)

      if [ -n "$SHA" ] && [ "$SHA" != "null" ]; then
        gh api -X POST "repos/$NEW_REPO/git/refs" -f ref="refs/heads/feedback" -f sha="$SHA" --silent 2>/dev/null || true

        HINT_B64=$(echo "This Pull Request is created for feedback purposes." | base64)
        gh api -X PUT "repos/$NEW_REPO/contents/.github/FEEDBACK_HINT.md" \
          -f message="Setup feedback PR" \
          -f content="$HINT_B64" \
          -f branch="$DEFAULT_BRANCH" --silent 2>/dev/null || true

        PR_ARGS=(
          --repo "$NEW_REPO"
          --title "Feedback"
          --body "Hi @$USERNAME, this Pull Request is created for your feedback and grading. Please do not close this PR."
          --base "feedback"
          --head "$DEFAULT_BRANCH"
        )
        if [ ${#REVIEWERS[@]} -gt 0 ]; then
          REVIEWERS_STR=$(IFS=,; echo "${REVIEWERS[*]}")
          if [ -n "$REVIEWERS_STR" ]; then
            PR_ARGS+=(--reviewer "$REVIEWERS_STR")
          fi
        fi
        gh pr create "${PR_ARGS[@]}" 2>&1 || true
      fi

      echo -e "     Status:   ${GREEN}[SUCCESSFULLY PROVISIONED]${NC}"
      ((TOTAL_SUCCESS++))
    fi
  done
  echo ""
done

# Step 3: Summary Banner
echo -e "${BOLD}${CYAN}=====================================================${NC}"
echo -e "${BOLD}Summary:${NC}"
if [ "$DRY_RUN" = true ]; then
  echo -e "  ${YELLOW}Mode: DRY-RUN (Simulasi)${NC}"
  echo -e "  ${YELLOW}- Users processed:${NC} $TOTAL_USERS"
  echo -e "  ${YELLOW}- Repositories planned:${NC} $TOTAL_SUCCESS"
else
  echo -e "  ${GREEN}✓ Users processed:${NC} $TOTAL_USERS"
  echo -e "  ${GREEN}✓ Repositories successful:${NC} $TOTAL_SUCCESS"
  [ "$TOTAL_FAILED" -gt 0 ] && echo -e "  ${RED}✗ Repositories failed:${NC} $TOTAL_FAILED"
fi
echo -e "${BOLD}${CYAN}=====================================================${NC}"
