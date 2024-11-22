#!/bin/bash

# Ensure the script is run with sudo
if [[ $EUID -ne 0 ]]; then
   echo "This script must be run as root. Use sudo." 
   exit 1
fi

# Install software-properties-common
echo "Installing software-properties-common..."
apt-get install -y software-properties-common

# Add the universe repository
echo "Adding the universe repository..."
add-apt-repository -y universe

# Update the package list
echo "Updating package list..."
apt-get update -y

# Install any additional packages (this will need clarification if specific packages are desired)
echo "Installing additional packages..."
apt-get install -y

echo "All tasks completed successfully!"