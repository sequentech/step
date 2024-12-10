#!/bin/bash

# Check git is installed
if ! command -v git &> /dev/null; then
    log error "Git is not installed"
    exit 1
fi

# Check Docker is installed
if ! command -v docker &> /dev/null; then
    log error "Docker is not installed"
    exit 1
fi

# Check AWS Cli is installed
if ! command -v aws &> /dev/null; then
    log error "AWS cli is not installed"
    exit 1
fi


git clone git@github.com:sequentech/step.git
cd step/packages
# Fetch dependencies
docker build -f packages/windmill/Dockerfile.vstl-dependencies -t windmill-dependencies .

#Compile code
docker build -f packages/windmill/Dockerfile.vstl-compile -t windmill-compile .

#Generate final image
docker build -f packages/windmill/Dockerfile.vstl-build -t windmill-build .
