# SPDX-FileCopyrightText: 2024 Sequent Legal <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only
#/bin/bash

sudo apt update
sudo apt install -y python3 python3-pip python3-venv sqlite3 default-jdk perl p7zip-full
python3 -m venv ~/.envs/venvwrapper
source ~/.envs/venvwrapper/bin/activate
pip3 install -r requirements.txt