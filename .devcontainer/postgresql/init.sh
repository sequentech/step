#!/bin/bash
# SPDX-FileCopyrightText: 2023-2024 Sequent Tech <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

set -ex

psql \
    -v ON_ERROR_STOP=1 \
    --username "$POSTGRES_USER" \
    --dbname "$POSTGRES_DB" \
    <<-EOSQL
    CREATE EXTENSION IF NOT EXISTS pgaudit;
    CREATE EXTENSION IF NOT EXISTS pgcrypto;
    CREATE EXTENSION IF NOT EXISTS unaccent;

    -- Create the normalization function
    CREATE OR REPLACE FUNCTION normalize_text(input_text TEXT)
    RETURNS TEXT AS $$
    BEGIN
    RETURN lower(
            regexp_replace(
                unaccent(btrim(input_text)),
                '[-\s]+', -- Match hyphens and whitespace
                '',
                'g'      -- Globally replace
            )
            );
    END;
    $$ LANGUAGE plpgsql IMMUTABLE;

    -- Check if user_entity table exists and create indexes if it does
    DO $$
    BEGIN
        IF EXISTS (SELECT FROM information_schema.tables WHERE table_name = 'user_entity') THEN
            -- Normalized User entity
            CREATE INDEX IF NOT EXISTS idx_user_entity_first_name_normalize ON user_entity((normalize_text(first_name)));
            CREATE INDEX IF NOT EXISTS idx_user_entity_last_name_normalize ON user_entity((normalize_text(last_name)));
        END IF;
    END $$;

    -- Check if user_attribute table exists and create index if it does
    DO $$
    BEGIN
        IF EXISTS (SELECT FROM information_schema.tables WHERE table_name = 'user_attribute') THEN
            -- Normalized attribute
            CREATE INDEX IF NOT EXISTS idx_user_attribute_name_value_normalize_text ON user_attribute(name, (normalize_text(value)));
        END IF;
    END $$;
EOSQL

{ echo "host replication $POSTGRES_USER 0.0.0.0/0 trust"; } >> "$PGDATA/pg_hba.conf"
