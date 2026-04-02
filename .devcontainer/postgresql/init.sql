-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
--
-- SPDX-License-Identifier: AGPL-3.0-only

CREATE EXTENSION IF NOT EXISTS pgcrypto;
CREATE EXTENSION IF NOT EXISTS unaccent;

CREATE OR REPLACE FUNCTION normalize_text(input_text TEXT)
RETURNS TEXT AS $$
BEGIN
RETURN lower(
        regexp_replace(
            unaccent(btrim(input_text)),
            '[-\s]+',
            '',
            'g'
        )
        );
END;
$$ LANGUAGE plpgsql IMMUTABLE;

DO $$
BEGIN
    IF EXISTS (SELECT FROM information_schema.tables WHERE table_name = 'user_entity') THEN
        CREATE INDEX IF NOT EXISTS idx_user_entity_first_name_normalize ON user_entity((normalize_text(first_name)));
        CREATE INDEX IF NOT EXISTS idx_user_entity_last_name_normalize ON user_entity((normalize_text(last_name)));
        CREATE INDEX IF NOT EXISTS idx_user_entity_realm_id ON user_entity(realm_id);
    END IF;
END $$;

DO $$
BEGIN
    IF EXISTS (SELECT FROM information_schema.tables WHERE table_name = 'user_attribute') THEN
        CREATE INDEX IF NOT EXISTS idx_user_attribute_name_value_normalize_text ON user_attribute(name, (normalize_text(value)));
        CREATE INDEX IF NOT EXISTS idx_user_attribute_user_id ON user_attribute(user_id);
        CREATE INDEX IF NOT EXISTS idx_user_attribute_userid_name_value ON user_attribute(user_id, name, value);
    END IF;
END $$;