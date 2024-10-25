#/bin/bash
sudo apt update
sudo apt install -y python3 python3-pip python3-venv sqlite3
python3 -m venv ~/.envs/venvwrapper
source ~/.envs/venvwrapper/bin/activate
pip3 install -r requirements.txt