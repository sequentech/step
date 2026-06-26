SELECT 'CREATE DATABASE b4'
WHERE NOT EXISTS (SELECT FROM pg_database WHERE datname = 'b4')\gexec

\c b4
\i /docker-entrypoint-initdb.d/schema.sql
