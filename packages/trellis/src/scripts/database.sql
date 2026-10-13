-- psql -U postgres -f .\src\scripts\database.sql
DROP DATABASE trellis_db;
CREATE DATABASE trellis_db;
CREATE USER "user" WITH PASSWORD 'user';
GRANT ALL PRIVILEGES ON DATABASE trellis_db TO "user";

-- Connect to the trellis_db database to set schema permissions
\c trellis_db

-- Grant schema permissions
GRANT ALL ON SCHEMA public TO "user";
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT ALL ON TABLES TO "user";
ALTER DEFAULT PRIVILEGES IN SCHEMA public GRANT ALL ON SEQUENCES TO "user";
