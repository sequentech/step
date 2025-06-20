#!/bin/bash

# SPDX-FileCopyrightText: 2024-2025 Sequent Legal <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

set -e  # Exit on any error

INSTALL_DIR="/opt/sequent-step"
VENV_DIR="$INSTALL_DIR/venv"
GIST_LOAD_TOOL="https://gist.githubusercontent.com/edulix/875a9a5d26407e1530f7769419dd8961/raw/341d7bab0482913d6b56efad512f383e5869eb85/load_tool.py"

# Color definitions
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BOLD='\033[1m'
RESET='\033[0m'

echo -e "${GREEN}${BOLD}=== Load Testing Tool Setup Script ===${RESET}"
echo -e "${YELLOW}This script will install dependencies, download the tool, and verify it works.${RESET}"
echo ""

# Ensure running as root for /opt install
if [[ $EUID -ne 0 ]]; then
    echo "Please run as root (sudo $0)"
    exit 1
fi

# Update package repositories
echo -e "${GREEN}Step 1/7:${RESET} Updating apt package repositories..."
apt update -y

# Install system dependencies
echo -e "${GREEN}Step 2/7:${RESET} Installing Python 3, pip, venv, and system dependencies..."
apt install -y python3 python3-pip python3-venv python3-dev libpq-dev curl

# Check Python version
echo -e "${GREEN}Step 3/7:${RESET} Checking Python installation..."
python3 --version
pip3 --version

# Create install directory
echo -e "${GREEN}Step 4/7:${RESET} Creating install directory at ${BOLD}$INSTALL_DIR${RESET}..."
mkdir -p "$INSTALL_DIR"
cd "$INSTALL_DIR"

# Create virtual environment
echo -e "${GREEN}Step 5/7:${RESET} Creating Python virtual environment at ${BOLD}$VENV_DIR${RESET}..."
python3 -m venv "$VENV_DIR"

# Activate virtual environment
source "$VENV_DIR/bin/activate"

# Inline Python dependencies (replace with your actual requirements)
echo -e "${GREEN}Step 6/7:${RESET} Installing Python dependencies in virtualenv..."
cat > requirements.txt <<EOF
Faker==13.3.4
psycopg2==2.9.10
openpyxl==3.1.5
pyzipper==0.3.6
python-dotenv==1.0.1
pybars3==0.9.7
EOF
pip install -r requirements.txt

# Download load_tool.py from gist
echo -e "${GREEN}Step 7/7:${RESET} Downloading load_tool.py from Gist..."
wget "$GIST_LOAD_TOOL"

# Verify installation by running the tool with --help
echo -e "${GREEN}Verifying installation by running load_tool.py --help...${RESET}"
python load_tool.py --help

echo ""
echo -e "${BOLD}${GREEN}=== Setup Complete! ===${RESET}"
echo -e "${YELLOW}The load testing tool is now installed and ready to use.${RESET}"
echo -e "You can run it from this directory: ${BOLD}$INSTALL_DIR${RESET}"
echo ""
echo -e "${GREEN}To use the tool, activate the virtual environment first:${RESET}"
echo -e "  ${BOLD}cd ${INSTALL_DIR}${RESET}"
echo -e "  ${BOLD}source $VENV_DIR/bin/activate${RESET}"
echo ""
echo -e "${GREEN}Example usage:${RESET}"
echo -e "  ${BOLD}python load_tool.py duplicate-votes --num-votes 1000 --election-event-id <ID>${RESET}"
echo ""