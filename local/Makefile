.PHONY: help login up down restart logs ps clean clean-volumes init-db shell-db shell-immudb health

help: ## Show this help message
	@echo 'Usage: make [target]'
	@echo ''
	@echo 'Available targets:'
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z_-]+:.*?## / {printf "  %-20s %s\n", $$1, $$2}' $(MAKEFILE_LIST)

login: ## Login to AWS ECR
	@echo "Logging into AWS ECR..."
	@aws ecr get-login-password --region eu-west-1 | docker login --username AWS --password-stdin 133529410358.dkr.ecr.eu-west-1.amazonaws.com

up: ## Start all services
	@echo "Starting all services..."
	@docker-compose up -d
	@echo "Services started. Use 'make logs' to view logs or 'make ps' to check status"

up-infra: ## Start only infrastructure services (db, rabbitmq, immudb, minio)
	@echo "Starting infrastructure services..."
	@docker-compose up -d db rabbitmq immudb-primary minio minio-init
	@echo "Infrastructure services started."

up-apps: ## Start application services (requires infrastructure)
	@echo "Starting application services..."
	@docker-compose up -d hasura keycloakx voting-portal admin-portal ballot-verifier b3 harvest windmill windmill-beat windmill-electoral-log
	@echo "Application services started."

down: ## Stop all services
	@echo "Stopping all services..."
	@docker-compose down
	@echo "All services stopped."

restart: ## Restart all services
	@echo "Restarting all services..."
	@docker-compose restart
	@echo "All services restarted."

restart-%: ## Restart a specific service (e.g., make restart-hasura)
	@echo "Restarting $*..."
	@docker-compose restart $*
	@echo "$* restarted."

logs: ## View logs from all services
	@docker-compose logs -f

logs-%: ## View logs from a specific service (e.g., make logs-hasura)
	@docker-compose logs -f $*

ps: ## Show status of all services
	@docker-compose ps

health: ## Check health status of all services
	@echo "Service Health Status:"
	@docker-compose ps --format "table {{.Name}}\t{{.Status}}\t{{.Health}}"

clean: down ## Stop and remove containers
	@echo "Removing containers..."
	@docker-compose rm -f
	@echo "Containers removed."

clean-volumes: ## Stop services and remove all volumes (WARNING: deletes all data)
	@echo "WARNING: This will delete all data in volumes!"
	@read -p "Are you sure? [y/N] " -n 1 -r; \
	echo; \
	if [[ $$REPLY =~ ^[Yy]$$ ]]; then \
		docker-compose down -v; \
		echo "All volumes removed."; \
	else \
		echo "Cancelled."; \
	fi

pull: ## Pull all images
	@echo "Pulling all images..."
	@docker-compose pull
	@echo "Images pulled."

build: ## Build custom images if any
	@echo "Building images..."
	@docker-compose build
	@echo "Images built."

shell-db: ## Open PostgreSQL shell
	@docker exec -it sequent-postgres psql -U postgres

shell-hasura-db: ## Open Hasura database shell
	@docker exec -it sequent-postgres psql -U test_hasura -d test_hasura

shell-keycloak-db: ## Open Keycloak database shell
	@docker exec -it sequent-postgres psql -U test_keycloak -d test_keycloak

shell-b3-db: ## Open B3 database shell
	@docker exec -it sequent-postgres psql -U test_b3 -d test_b3

shell-immudb: ## Open ImmuDB shell
	@docker exec -it sequent-immudb-primary immuadmin login immudb

shell-%: ## Open shell in a specific container (e.g., make shell-hasura)
	@docker exec -it sequent-$* /bin/sh || docker exec -it sequent-$* /bin/bash

validate: ## Validate docker-compose configuration
	@docker-compose config --quiet && echo "Configuration is valid" || echo "Configuration has errors"

init-hasura: ## Initialize Hasura with migrations and metadata
	@echo "Applying Hasura migrations..."
	@cd ../hasura && hasura migrate apply --endpoint http://localhost:8080 --admin-secret hasura_admin_secret
	@echo "Applying Hasura metadata..."
	@cd ../hasura && hasura metadata apply --endpoint http://localhost:8080 --admin-secret hasura_admin_secret
	@echo "Hasura initialized."

stats: ## Show container resource usage
	@docker stats --no-stream $$(docker-compose ps -q)

top: ## Show running processes in containers
	@docker-compose top

networks: ## Show network information
	@docker network ls | grep sequent
	@echo ""
	@docker network inspect sequent_sequent-network

volumes: ## Show volume information
	@docker volume ls | grep sequent

prune: ## Clean up unused Docker resources
	@echo "Cleaning up unused Docker resources..."
	@docker system prune -f
	@echo "Cleanup complete."

test-connectivity: ## Test connectivity between services
	@echo "Testing database connectivity..."
	@docker exec sequent-hasura curl -f db:5432 && echo "✓ Database reachable" || echo "✗ Database unreachable"
	@echo "Testing RabbitMQ connectivity..."
	@docker exec sequent-windmill curl -f rabbitmq:15672 && echo "✓ RabbitMQ reachable" || echo "✗ RabbitMQ unreachable"
	@echo "Testing ImmuDB connectivity..."
	@docker exec sequent-windmill curl -f immudb-primary:8080 && echo "✓ ImmuDB reachable" || echo "✗ ImmuDB unreachable"

watch: ## Watch service status
	@watch -n 2 'docker-compose ps'
