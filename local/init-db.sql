-- Initialize databases for local development

-- Create Hasura database and user
CREATE DATABASE test_hasura;
CREATE USER test_hasura WITH PASSWORD 'hasura_db_password';
GRANT ALL PRIVILEGES ON DATABASE test_hasura TO test_hasura;

-- Create Keycloak database and user
CREATE DATABASE test_keycloak;
CREATE USER test_keycloak WITH PASSWORD 'keycloak_db_password';
GRANT ALL PRIVILEGES ON DATABASE test_keycloak TO test_keycloak;

-- Create B3 database and user
CREATE DATABASE test_b3;
CREATE USER test_b3 WITH PASSWORD 'b3_db_password';
GRANT ALL PRIVILEGES ON DATABASE test_b3 TO test_b3;

-- Grant permissions to schemas
\c test_hasura;
GRANT ALL ON SCHEMA public TO test_hasura;

\c test_keycloak;
GRANT ALL ON SCHEMA public TO test_keycloak;

\c test_b3;
GRANT ALL ON SCHEMA public TO test_b3;
