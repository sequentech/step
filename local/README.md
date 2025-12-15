# Local Development Environment with Docker Compose

This docker-compose setup creates a local development environment based on the Kubernetes cluster configuration from `test-apps`, `test-networking`, and `global-apps` namespaces.

## Architecture

The setup includes the following services:

### Infrastructure Services (from global-apps)
- **PostgreSQL**: Shared database for all services
- **ImmuDB**: Immutable database (primary instance)
- **RabbitMQ**: Message queue with management UI
- **MinIO**: S3-compatible object storage

### Application Services (from test-apps)
- **Hasura**: GraphQL engine (port 8080)
- **Keycloak**: Identity and access management (port 8081)
- **Voting Portal**: Public voting interface (port 8000)
- **Admin Portal**: Administrative interface (port 8002)
- **Ballot Verifier**: Ballot verification service (port 8001)
- **B3**: Backend service with gRPC (port 50051)
- **Harvest**: Data harvesting service (port 8400)
- **Windmill**: Main worker consuming queues
- **Windmill Beat**: Task scheduler
- **Windmill Electoral Log**: Electoral log processor

## Prerequisites

- Docker and Docker Compose installed
- Access to AWS ECR (for pulling application images)
- AWS credentials configured for ECR access

### AWS ECR Authentication

Before starting the services, authenticate with AWS ECR:

```bash
aws ecr get-login-password --region eu-west-1 | docker login --username AWS --password-stdin 133529410358.dkr.ecr.eu-west-1.amazonaws.com
```

## Quick Start

1. **Navigate to the local directory:**
   ```bash
   cd /home/angel/work/sequent/step/local
   ```

2. **Review and customize the `.env` file:**
   ```bash
   vim .env
   ```

3. **Start all services:**
   ```bash
   docker-compose up -d
   ```

4. **Check service status:**
   ```bash
   docker-compose ps
   ```

5. **View logs:**
   ```bash
   # All services
   docker-compose logs -f

   # Specific service
   docker-compose logs -f hasura
   ```

## Service URLs

Once all services are running, you can access them at:

- **Voting Portal**: http://localhost:8000
- **Ballot Verifier**: http://localhost:8001
- **Admin Portal**: http://localhost:8002
- **Hasura Console**: http://localhost:8080/console (admin secret: `hasura_admin_secret`)
- **Keycloak Admin**: http://localhost:8081 (admin/admin)
- **RabbitMQ Management**: http://localhost:15672 (user/rabbitmq_local_password)
- **MinIO Console**: http://localhost:9001 (minioadmin/minioadmin)

## Database Access

Connect to PostgreSQL:
```bash
# Using docker exec
docker exec -it sequent-postgres psql -U postgres

# Or connect from host
psql -h localhost -U postgres -d postgres
```

Available databases:
- `test_hasura` - Hasura metadata and data
- `test_keycloak` - Keycloak data
- `test_b3` - B3 service data

## ImmuDB Access

Connect to ImmuDB:
```bash
docker exec -it sequent-immudb-primary immuadmin login immudb
```

## MinIO/S3 Storage

MinIO provides S3-compatible storage. Two buckets are created:
- `election-event-documents` - Private bucket
- `public` - Public bucket (read-only access)

Access the MinIO console at http://localhost:9001 to manage buckets and files.

## Troubleshooting

### Services not starting
Check logs for specific services:
```bash
docker-compose logs <service-name>
```

### Database connection issues
Ensure databases are initialized:
```bash
docker-compose logs db
```

The init script should create all required databases and users.

### Image pull errors
Make sure you're authenticated with AWS ECR:
```bash
aws ecr get-login-password --region eu-west-1 | docker login --username AWS --password-stdin 133529410358.dkr.ecr.eu-west-1.amazonaws.com
```

### Health check failures
Services may take time to become healthy. Check health status:
```bash
docker-compose ps
```

## Development Workflow

### Starting specific services
```bash
# Start only infrastructure
docker-compose up -d db rabbitmq immudb-primary minio

# Start with specific services
docker-compose up -d hasura keycloakx windmill
```

### Stopping services
```bash
# Stop all
docker-compose stop

# Stop specific service
docker-compose stop windmill
```

### Rebuilding services
```bash
# Rebuild and restart
docker-compose up -d --build <service-name>
```

### Cleaning up
```bash
# Stop and remove containers
docker-compose down

# Remove volumes (WARNING: deletes all data)
docker-compose down -v
```

## Environment Variables

Key environment variables are defined in `.env`:

- **Database credentials**: POSTGRES_*, HASURA_DB_*, KEYCLOAK_DB_*, B3_PG_*
- **RabbitMQ**: RABBITMQ_DEFAULT_USER, RABBITMQ_DEFAULT_PASS
- **ImmuDB**: IMMUDB_USER, IMMUDB_PASSWORD
- **MinIO/S3**: MINIO_ROOT_USER, AWS_S3_*
- **Application**: ENV_SLUG, LOG_LEVEL, APP_VERSION

## Network

All services run on the `sequent-network` bridge network, allowing them to communicate using service names as hostnames.

## Differences from Kubernetes Deployment

This local setup differs from the Kubernetes deployment in several ways:

1. **Single PostgreSQL instance**: All databases share one PostgreSQL container
2. **No load balancing**: Single instance of each service
3. **MinIO instead of AWS S3**: Local S3-compatible storage
4. **No ingress controller**: Direct port mappings
5. **Simplified secrets**: Environment variables instead of Kubernetes secrets
6. **No replica sets**: ImmuDB runs only primary (no sync/async replicas)
7. **Console-based email/SMS**: No actual AWS SES/SNS integration

## Next Steps

1. Initialize Hasura with migrations:
   ```bash
   cd ../hasura
   hasura migrate apply --endpoint http://localhost:8080 --admin-secret hasura_admin_secret
   hasura metadata apply --endpoint http://localhost:8080 --admin-secret hasura_admin_secret
   ```

2. Configure Keycloak realms and clients as needed

3. Upload public assets to MinIO's public bucket for JWT/JWKS

4. Start developing!

## Support

For issues or questions about this setup, refer to the main project documentation or contact the development team.
