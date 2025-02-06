<!--
SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# Release NEXT

## ✨ Remove tagalo from admin settings in janitor

In order to ensure that tagalo is not active as a language in the admin portal, ensure
that in the excel file you're using for janitor, you have this configuration: in the
`Parameters` tab, add a row with:

- type: admin
- key: tenant_configurations.settings.language_conf.enabled_language_codes
- value: ["en"]

## ✨ Windmill > Enrollment: improved fuzzy search with indexes

A new function needs to be created to normalize search values:
```
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
```

And a few index that make use of the new normalizing function
```
-- Normalized User entity
CREATE INDEX idx_user_entity_first_name_normalize ON user_entity((normalize_text(first_name)));
CREATE INDEX idx_user_entity_last_name_normalize ON user_entity((normalize_text(last_name)));
-- Normalized attribute
CREATE INDEX idx_user_attribute_name_value_normalize_text ON user_attribute(name, (normalize_text(value)));