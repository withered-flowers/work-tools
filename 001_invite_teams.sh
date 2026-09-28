#!/usr/bin/env bash
#
# global_invite_teams.sh - Invite GitHub users to one or more teams in GitHub organization.
#
# Usage:
#   1. Batch mode using INVITATION_LIST inside this script:
#        ./global_invite_teams.sh
#        ./global_invite_teams.sh --dry-run
#
#   2. CLI Arguments mode (single user with multiple teams):
#        ./global_invite_teams.sh -u "username" -t "Phase 1 - Set 1,Phase 2 - Set 1"
#        ./global_invite_teams.sh -u "username" -t "Phase 1 - Set 1" --dry-run
#
#   3. File / CSV mode (username,team1,team2,...):
#        ./global_invite_teams.sh --file users.csv
#        ./global_invite_teams.sh --file users.csv --dry-run
#

set -uo pipefail

# Disable path conversion in Git Bash / MSYS2 on Windows
export MSYS_NO_PATHCONV=1

ORG_NAME="ORGANIZATION-NAME"
DEFAULT_ROLE="member" # Options: member, maintainer

# ANSI Color codes
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m' # No Color

# ==============================================================================
# Konfigurasi Daftar Undangan Default (Batch Mode)
# Format: "github_username|team_1,team_2,team_3"
# Nama tim dapat berupa nama lengkap (contoh: "Phase 1 - Set 1") atau slug ("phase-1-set-1")
# ==============================================================================
INVITATION_LIST=(
  # Contoh:
  # "john_doe|Phase 1 - Set 1"
  # "jane_smith|Phase 1 - Set 1,Phase 2 - Set 1"
  # "alex_student|Phase 3 - Set 2"
  "user|Phase 1 - Set 1,Phase 2 - Set 1"
)

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
CLI_TEAMS=""
INPUT_FILE=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --dry-run)
      DRY_RUN=true
      shift
      ;;
    -u|--username)
      CLI_USERNAME="$2"
      shift 2
      ;;
    -t|--teams)
      CLI_TEAMS="$2"
      shift 2
      ;;
    -r|--role)
      DEFAULT_ROLE="$2"
      shift 2
      ;;
    -f|--file)
      INPUT_FILE="$2"
      shift 2
      ;;
    -h|--help)
      echo "Usage: ./05-invite-to-teams.sh [OPTIONS]"
      echo ""
      echo "Options:"
      echo "  -u, --username USERNAME    GitHub username to invite"
      echo "  -t, --teams TEAMS          Comma-separated list of teams (e.g. 'Phase 1 - Set 1,Phase 2 - Set 1')"
      echo "  -r, --role ROLE            Team membership role (default: 'member', options: 'member', 'maintainer')"
      echo "  -f, --file FILE_PATH       Path to CSV/TXT file with format: username,team1,team2,..."
      echo "      --dry-run              Simulate invitations without calling GitHub API"
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

# Menyiapkan list target undangan yang akan diproses
TARGET_INVITATIONS=()

if [ -n "$CLI_USERNAME" ]; then
  if [ -z "$CLI_TEAMS" ]; then
    echo -e "${RED}Error: Parameter --teams (-t) wajib disertakan jika menggunakan --username (-u).${NC}"
    exit 1
  fi
  TARGET_INVITATIONS+=("${CLI_USERNAME}|${CLI_TEAMS}")
elif [ -n "$INPUT_FILE" ]; then
  if [ ! -f "$INPUT_FILE" ]; then
    echo -e "${RED}Error: File '${INPUT_FILE}' tidak ditemukan.${NC}"
    exit 1
  fi
  while IFS= read -r line || [ -n "$line" ]; do
    # Abaikan komentar (#) atau baris kosong
    [[ "$line" =~ ^[[:space:]]*# ]] && continue
    [[ -z "${line// }" ]] && continue

    # Parse baris (username,team1,team2 atau username|team1,team2)
    if [[ "$line" == *"|"* ]]; then
      TARGET_INVITATIONS+=("$line")
    elif [[ "$line" == *","* ]]; then
      user=$(echo "$line" | cut -d',' -f1 | xargs)
      teams=$(echo "$line" | cut -d',' -f2- | xargs)
      TARGET_INVITATIONS+=("${user}|${teams}")
    fi
  done < "$INPUT_FILE"
else
  TARGET_INVITATIONS=("${INVITATION_LIST[@]}")
fi

TOTAL_USERS=${#TARGET_INVITATIONS[@]}

if [ "$TOTAL_USERS" -eq 0 ]; then
  echo -e "${YELLOW}Tidak ada data undangan untuk diproses.${NC}"
  echo -e "Silakan gunakan salah satu cara berikut:"
  echo -e "  1. Jalankan dengan argumen: ${BOLD}./05-invite-to-teams.sh -u <username> -t \"Phase 1 - Set 1,Phase 2 - Set 1\"${NC}"
  echo -e "  2. Jalankan dengan file CSV: ${BOLD}./05-invite-to-teams.sh --file users.csv${NC}"
  echo -e "  3. Isi array ${BOLD}INVITATION_LIST${NC} di dalam file skrip ini."
  exit 0
fi

echo -e "${BOLD}${CYAN}=====================================================${NC}"
echo -e "${BOLD}${CYAN} 05. Team Invitations in GitHub Org: ${ORG_NAME}${NC}"
echo -e " Role: ${BOLD}${DEFAULT_ROLE}${NC}"
if [ "$DRY_RUN" = true ]; then
  echo -e "${YELLOW} >>> [DRY-RUN MODE ACTIVE] No changes will be written <<<${NC}"
fi
echo -e "${BOLD}${CYAN}=====================================================${NC}"
echo ""

TOTAL_SUCCESS=0
TOTAL_FAILED=0
USER_INDEX=0

for entry in "${TARGET_INVITATIONS[@]}"; do
  ((USER_INDEX++))
  IFS="|" read -r USERNAME TEAMS_RAW <<< "$entry"
  USERNAME=$(echo "$USERNAME" | xargs)

  echo -e "${BOLD}${CYAN}-----------------------------------------------------${NC}"
  echo -e "${BOLD}[${USER_INDEX}/${TOTAL_USERS}] User: ${USERNAME}${NC}"
  echo -e "${BOLD}${CYAN}-----------------------------------------------------${NC}"

  # Split koma pada daftar tim
  IFS="," read -r -a TEAMS_ARRAY <<< "$TEAMS_RAW"

  for team_item in "${TEAMS_ARRAY[@]}"; do
    TEAM_NAME=$(echo "$team_item" | xargs)
    [ -z "$TEAM_NAME" ] && continue

    # Normalisasi nama tim menjadi team slug
    TEAM_SLUG=$(echo "$TEAM_NAME" | tr '[:upper:]' '[:lower:]' | sed -E 's/[^a-z0-9]+/-/g' | sed -E 's/^-|-$//g')

    printf "  -> Team: %-25s (Slug: %-20s) " "$TEAM_NAME" "$TEAM_SLUG"

    if [ "$DRY_RUN" = true ]; then
      echo -e "${YELLOW}[DRY-RUN - WOULD INVITE]${NC}"
      ((TOTAL_SUCCESS++))
    else
      # PUT orgs/{org}/teams/{team_slug}/memberships/{username}
      RESPONSE=$(gh api --method PUT "orgs/${ORG_NAME}/teams/${TEAM_SLUG}/memberships/${USERNAME}" \
        -f role="${DEFAULT_ROLE}" 2>&1)
      EXIT_CODE=$?

      if [ $EXIT_CODE -eq 0 ]; then
        MEMBERSHIP_STATE=$(echo "$RESPONSE" | jq -r '.state' 2>/dev/null || echo "invited")
        if [ "$MEMBERSHIP_STATE" == "active" ]; then
          echo -e "${GREEN}[ACTIVE MEMBER]${NC}"
        else
          STATE_UPPER=$(echo "$MEMBERSHIP_STATE" | tr '[:lower:]' '[:upper:]')
          echo -e "${GREEN}[INVITATION SENT (${STATE_UPPER})]${NC}"
        fi
        ((TOTAL_SUCCESS++))
      else
        echo -e "${RED}[FAILED]${NC}"
        echo -e "      ${RED}Error: ${RESPONSE}${NC}"
        ((TOTAL_FAILED++))
      fi
    fi
  done
  echo ""
done

echo -e "${BOLD}${CYAN}=====================================================${NC}"
echo -e "${BOLD}Summary:${NC}"
if [ "$DRY_RUN" = true ]; then
  echo -e "  ${YELLOW}Mode: DRY-RUN (Simulasi)${NC}"
  echo -e "  ${YELLOW}- Users processed:${NC} $TOTAL_USERS"
  echo -e "  ${YELLOW}- Invitations planned:${NC} $TOTAL_SUCCESS"
else
  echo -e "  ${GREEN}✓ Users processed:${NC} $TOTAL_USERS"
  echo -e "  ${GREEN}✓ Invitations successful:${NC} $TOTAL_SUCCESS"
  [ "$TOTAL_FAILED" -gt 0 ] && echo -e "  ${RED}✗ Invitations failed:${NC} $TOTAL_FAILED"
fi
echo -e "${BOLD}${CYAN}=====================================================${NC}"
