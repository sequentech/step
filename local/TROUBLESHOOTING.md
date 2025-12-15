# Troubleshooting Guide

This guide helps resolve common issues when running the local development environment.

## Table of Contents
- [Service Health Issues](#service-health-issues)
- [Database Connection Problems](#database-connection-problems)
- [Image Pull Errors](#image-pull-errors)
- [Network Issues](#network-issues)
- [Performance Problems](#performance-problems)
- [Data Persistence Issues](#data-persistence-issues)

---

## Service Health Issues

### Service shows as "unhealthy"

**Symptoms:** `docker-compose ps` shows service health as "unhealthy"

**Solutions:**

1. **Check service logs:**
   ```bash
   docker-compose logs <service-name>
   ```

2. **Verify health check configuration:**
   ```bash
   docker inspect sequent-<service-name> | grep -A 10 Healthcheck
   ```

3. **Manually test health endpoint:**
   ```bash
   # For Hasura
   docker exec sequent-hasura curl -f http://localhost:8080/healthz
   
   # For Windmill
   docker exec sequent-windmill curl -f http://localhost:3030/live
   ```

4. **Increase health check timeout:** Edit `docker-compose.yaml` and increase `timeout` and `interval` values.

### Service keeps restarting

**Symptoms:** Container restarts repeatedly

**Solutions:**

1. **Check logs for errors:**
   ```bash
   docker-compose logs --tail=100 <service-name>
   ```

2. **Common causes:**
   - Database not ready: Ensure DB is healthy before starting dependent services
   - Missing environment variables: Check `.env` file
   - Port conflicts: Ensure ports are not in use by other applications
   - Insufficient resources: Check Docker resource limits

3. **Start services in order:**
   ```bash
   # Start infrastructure first
   docker-compose up -d db rabbitmq immudb-primary minio
   
   # Wait for health checks
   sleep 30
   
   # Then start applications
   docker-compose up -d hasura keycloakx
   ```

---

## Database Connection Problems

### "Could not connect to database"

**Symptoms:** Services can't connect to PostgreSQL

**Solutions:**

1. **Check database is running:**
   ```bash
   docker-compose ps db
   ```

2. **Verify database is healthy:**
   ```bash
   docker exec sequent-postgres pg_isready -U postgres
   ```

3. **Check database logs:**
   ```bash
   docker-compose logs db
   ```

4. **Verify credentials in `.env` match:**
   ```bash
   cat .env | grep DB
   ```

5. **Test connection manually:**
   ```bash
   docker exec sequent-postgres psql -U test_hasura -d test_hasura -c "SELECT 1;"
   ```

### Database not initialized

**Symptoms:** "database does not exist" errors

**Solutions:**

1. **Check if init script ran:**
   ```bash
   docker-compose logs db | grep "init-db.sql"
   ```

2. **Manually create databases:**
   ```bash
   docker exec -it sequent-postgres psql -U postgres
   
   # In psql:
   CREATE DATABASE test_hasura;
   CREATE USER test_hasura WITH PASSWORD 'hasura_db_password';
   GRANT ALL PRIVILEGES ON DATABASE test_hasura TO test_hasura;
   ```

3. **Recreate database container:**
   ```bash
   docker-compose stop db
   docker-compose rm -f db
   docker volume rm sequent_db_data
   docker-compose up -d db
   ```

---

## Image Pull Errors

### "unauthorized: authentication required"

**Symptoms:** Cannot pull images from AWS ECR

**Solutions:**

1. **Authenticate with ECR:**
   ```bash
   aws ecr get-login-password --region eu-west-1 | \
     docker login --username AWS --password-stdin \
     133529410358.dkr.ecr.eu-west-1.amazonaws.com
   ```

2. **Check AWS credentials:**
   ```bash
   aws sts get-caller-identity
   ```

3. **Verify ECR permissions:** Ensure your AWS user has ECR pull permissions.

### "manifest not found" or "image not found"

**Symptoms:** Image version doesn't exist in ECR

**Solutions:**

1. **Check available tags:**
   ```bash
   aws ecr describe-images --repository-name windmill --region eu-west-1
   ```

2. **Update `.env` with correct version:**
   ```bash
   APP_VERSION=v9.3.0-rc.19  # or latest available version
   ```

3. **Use local images if available:**
   Edit `docker-compose.yaml` to use locally built images instead.

---

## Network Issues

### Services can't communicate

**Symptoms:** Service A can't reach Service B

**Solutions:**

1. **Verify all services are on same network:**
   ```bash
   docker network inspect sequent_sequent-network
   ```

2. **Test connectivity between containers:**
   ```bash
   docker exec sequent-windmill ping -c 3 db
   docker exec sequent-windmill ping -c 3 rabbitmq
   ```

3. **Check DNS resolution:**
   ```bash
   docker exec sequent-windmill nslookup db
   ```

4. **Restart network:**
   ```bash
   docker-compose down
   docker network rm sequent_sequent-network
   docker-compose up -d
   ```

### Port already in use

**Symptoms:** "bind: address already in use"

**Solutions:**

1. **Find process using port:**
   ```bash
   sudo lsof -i :8080  # Check port 8080
   sudo netstat -tulpn | grep 8080
   ```

2. **Stop conflicting service or change port in `docker-compose.yaml`:**
   ```yaml
   ports:
     - "8081:8080"  # Map to different host port
   ```

---

## Performance Problems

### Services are slow

**Solutions:**

1. **Check Docker resource allocation:**
   - Docker Desktop: Preferences → Resources
   - Increase CPU cores and memory allocation

2. **Monitor resource usage:**
   ```bash
   docker stats
   ```

3. **Reduce running services:**
   Start only needed services instead of all services.

4. **Check disk space:**
   ```bash
   df -h
   docker system df
   ```

5. **Clean up Docker:**
   ```bash
   docker system prune -a
   docker volume prune
   ```

### Database queries are slow

**Solutions:**

1. **Check database size:**
   ```bash
   docker exec sequent-postgres psql -U postgres -c "\l+"
   ```

2. **Vacuum database:**
   ```bash
   docker exec sequent-postgres vacuumdb -U postgres -d test_hasura -v
   ```

3. **Add indexes:** Review slow queries and add appropriate indexes.

---

## Data Persistence Issues

### Data lost after restart

**Symptoms:** Data disappears when containers restart

**Solutions:**

1. **Verify volumes are defined:**
   ```bash
   docker volume ls | grep sequent
   ```

2. **Check volume mounts in docker-compose.yaml:** Ensure services have proper volume mounts.

3. **Use named volumes instead of bind mounts** for production-like persistence.

### Cannot remove volumes

**Symptoms:** "volume is in use" when trying to remove

**Solutions:**

1. **Stop all containers first:**
   ```bash
   docker-compose down
   ```

2. **Force remove:**
   ```bash
   docker volume rm -f sequent_db_data
   ```

3. **Find containers using volume:**
   ```bash
   docker ps -a --filter volume=sequent_db_data
   ```

---

## RabbitMQ Issues

### "connection refused" errors

**Solutions:**

1. **Check RabbitMQ is running:**
   ```bash
   docker-compose ps rabbitmq
   ```

2. **Verify RabbitMQ is ready:**
   ```bash
   docker exec sequent-rabbitmq rabbitmq-diagnostics ping
   ```

3. **Check RabbitMQ logs:**
   ```bash
   docker-compose logs rabbitmq
   ```

4. **Access management UI:** http://localhost:15672 and check queue status

---

## ImmuDB Issues

### "immudb connection failed"

**Solutions:**

1. **Check ImmuDB is running:**
   ```bash
   docker-compose ps immudb-primary
   ```

2. **Verify gRPC port:**
   ```bash
   docker exec sequent-immudb-primary netstat -tulpn | grep 3322
   ```

3. **Test connection:**
   ```bash
   docker exec sequent-immudb-primary immuadmin status
   ```

---

## MinIO/S3 Issues

### "bucket does not exist"

**Solutions:**

1. **Check MinIO is running:**
   ```bash
   docker-compose ps minio
   ```

2. **Verify buckets were created:**
   ```bash
   docker-compose logs minio-init
   ```

3. **Manually create buckets:**
   Access MinIO console at http://localhost:9001 or use mc:
   ```bash
   docker run --rm --network sequent_sequent-network \
     minio/mc alias set myminio http://minio:9000 minioadmin minioadmin
   
   docker run --rm --network sequent_sequent-network \
     minio/mc mb myminio/election-event-documents
   ```

---

## Getting More Help

If problems persist:

1. **Collect diagnostic information:**
   ```bash
   docker-compose ps > status.txt
   docker-compose logs > logs.txt
   docker stats --no-stream > stats.txt
   docker system df > disk.txt
   ```

2. **Check Docker daemon logs:**
   - Linux: `journalctl -u docker`
   - Mac: Docker Desktop → Troubleshoot → Support

3. **Review Docker Compose configuration:**
   ```bash
   docker-compose config
   ```

4. **Contact team:** Share diagnostic files with development team
