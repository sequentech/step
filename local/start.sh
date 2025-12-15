#!/bin/bash

# Quick Start Script for Local Development Environment
# This script helps set up and start the local docker-compose environment

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}=== Sequent Local Development Environment Setup ===${NC}\n"

# Check if Docker is running
if ! docker info > /dev/null 2>&1; then
    echo -e "${RED}Error: Docker is not running. Please start Docker first.${NC}"
    exit 1
fi

# Check if docker-compose is available
if ! command -v docker-compose &> /dev/null; then
    echo -e "${RED}Error: docker-compose is not installed.${NC}"
    exit 1
fi

# Check if .env file exists
if [ ! -f .env ]; then
    echo -e "${YELLOW}Warning: .env file not found.${NC}"
    echo -e "Please ensure .env file exists with proper configuration."
    exit 1
fi

# Check AWS CLI for ECR authentication
if command -v aws &> /dev/null; then
    echo -e "${YELLOW}Authenticating with AWS ECR...${NC}"
    if aws ecr get-login-password --region eu-west-1 | docker login --username AWS --password-stdin 133529410358.dkr.ecr.eu-west-1.amazonaws.com; then
        echo -e "${GREEN}✓ AWS ECR authentication successful${NC}\n"
    else
        echo -e "${RED}✗ AWS ECR authentication failed${NC}"
        echo -e "${YELLOW}You may not be able to pull application images.${NC}\n"
    fi
else
    echo -e "${YELLOW}Warning: AWS CLI not found. Skipping ECR authentication.${NC}"
    echo -e "You may need to authenticate manually if pulling images fails.\n"
fi

# Ask user what to start
echo -e "What would you like to start?\n"
echo "1) Infrastructure only (PostgreSQL, RabbitMQ, ImmuDB, MinIO)"
echo "2) All services"
echo "3) Infrastructure + Hasura + Keycloak"
echo "4) Custom selection"
echo ""
read -p "Enter your choice [1-4] (default: 2): " choice
choice=${choice:-2}

case $choice in
    1)
        echo -e "\n${GREEN}Starting infrastructure services...${NC}"
        docker-compose up -d db rabbitmq immudb-primary minio minio-init
        ;;
    2)
        echo -e "\n${GREEN}Starting all services...${NC}"
        docker-compose up -d
        ;;
    3)
        echo -e "\n${GREEN}Starting infrastructure + core services...${NC}"
        docker-compose up -d db rabbitmq immudb-primary minio minio-init hasura keycloakx
        ;;
    4)
        echo -e "\n${YELLOW}Available services:${NC}"
        echo "Infrastructure: db, rabbitmq, immudb-primary, minio"
        echo "Core: hasura, keycloakx"
        echo "Apps: voting-portal, admin-portal, ballot-verifier, b3, harvest"
        echo "Workers: windmill, windmill-beat, windmill-electoral-log"
        echo ""
        read -p "Enter service names (space-separated): " services
        echo -e "\n${GREEN}Starting selected services...${NC}"
        docker-compose up -d $services
        ;;
    *)
        echo -e "${RED}Invalid choice. Exiting.${NC}"
        exit 1
        ;;
esac

# Wait a bit for services to start
echo -e "\n${YELLOW}Waiting for services to initialize...${NC}"
sleep 5

# Check service status
echo -e "\n${GREEN}Service Status:${NC}"
docker-compose ps

# Check health status
echo -e "\n${GREEN}Health Checks:${NC}"
docker-compose ps --format "table {{.Name}}\t{{.Status}}\t{{.Health}}" 2>/dev/null || docker-compose ps

# Display service URLs
echo -e "\n${GREEN}=== Service URLs ===${NC}"
echo -e "Voting Portal:      ${YELLOW}http://localhost:8000${NC}"
echo -e "Ballot Verifier:    ${YELLOW}http://localhost:8001${NC}"
echo -e "Admin Portal:       ${YELLOW}http://localhost:8002${NC}"
echo -e "Hasura Console:     ${YELLOW}http://localhost:8080/console${NC}"
echo -e "Keycloak Admin:     ${YELLOW}http://localhost:8081${NC} (admin/admin)"
echo -e "RabbitMQ Mgmt:      ${YELLOW}http://localhost:15672${NC} (user/rabbitmq_local_password)"
echo -e "MinIO Console:      ${YELLOW}http://localhost:9001${NC} (minioadmin/minioadmin)"

# Helpful commands
echo -e "\n${GREEN}=== Helpful Commands ===${NC}"
echo -e "View logs:          ${YELLOW}docker-compose logs -f${NC}"
echo -e "View logs (service):${YELLOW}docker-compose logs -f <service-name>${NC}"
echo -e "Stop all:           ${YELLOW}docker-compose stop${NC}"
echo -e "Stop all & remove:  ${YELLOW}docker-compose down${NC}"
echo -e "Service status:     ${YELLOW}docker-compose ps${NC}"
echo -e "Or use:             ${YELLOW}make help${NC} (if make is installed)"

echo -e "\n${GREEN}Setup complete! Happy coding! 🚀${NC}\n"
