#!/bin/bash

# This script prepares a fresh Ubuntu server for running the Sequent Step development environment.

# Update and upgrade the system
sudo apt-get update && sudo apt-get upgrade -y

# Install git
sudo apt-get install -y git

# Install Docker
curl -fsSL https://get.docker.com -o get-docker.sh
sudo sh get-docker.sh

# Add the current user to the docker group
sudo usermod -aG docker ${USER}

# Clone the repository
echo "Cloning the Sequent Step repository..."
git clone https://github.com/sequentech/step.git /home/${USER}/step

# Display a message to the user to log out and log back in for the group changes to take effect
echo "The Sequent Step repository has been cloned to /home/${USER}/step."
echo "Please log out and log back in for the Docker group changes to take effect."
