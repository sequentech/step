# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Exercise the production role guard against disposable PostgreSQL tables.

The CSV case additionally checks the database uniqueness boundary used by the
insert-only importer; it is not an HTTP/import worker integration test.
"""
from pathlib import Path
import subprocess

root = Path(__file__).resolve().parents[2]
guard = (root / "packages/harvest/src/services/independent_role_editor.sql").read_text()
sql = """
BEGIN;
CREATE TEMP TABLE realm (id text PRIMARY KEY, name text UNIQUE);
CREATE TEMP TABLE user_entity (id text PRIMARY KEY, realm_id text, username text,
    UNIQUE (realm_id, username));
CREATE TEMP TABLE keycloak_group (id text PRIMARY KEY, realm_id text, parent_group text);
CREATE TEMP TABLE user_group_membership (user_id text, group_id text, PRIMARY KEY(user_id,group_id));
INSERT INTO realm VALUES ('r','tenant-one'),('r2','tenant-two');
INSERT INTO user_entity VALUES ('actor','r','administrator'),('other','r','board'),('foreign','r2','administrator');
INSERT INTO keycloak_group VALUES ('root','r',NULL),('parent','r','root'),('direct','r','parent'),
    ('other','r',NULL),('foreign','r2',NULL);
INSERT INTO user_group_membership VALUES ('actor','direct'),('other','other'),('foreign','foreign');
CREATE FUNCTION pg_temp.guard(text,text,text) RETURNS TABLE(actor_present bool, affects_actor bool)
LANGUAGE SQL AS $guard$
""" + guard + """
$guard$;
DO $$
DECLARE allowed bool;
BEGIN
    IF NOT (SELECT affects_actor FROM pg_temp.guard('actor','tenant-one','direct')) THEN
        RAISE EXCEPTION 'Direct role was editable'; END IF;
    IF NOT (SELECT affects_actor FROM pg_temp.guard('actor','tenant-one','parent')) THEN
        RAISE EXCEPTION 'Parent role was editable'; END IF;
    IF NOT (SELECT affects_actor FROM pg_temp.guard('actor','tenant-one','root')) THEN
        RAISE EXCEPTION 'Ancestor role was editable'; END IF;
    SELECT actor_present AND NOT affects_actor INTO allowed FROM pg_temp.guard('actor','tenant-one','other');
    IF NOT allowed THEN RAISE EXCEPTION 'Independent role edit was refused'; END IF;
    IF (SELECT actor_present FROM pg_temp.guard('missing','tenant-one','other')) THEN
        RAISE EXCEPTION 'Unknown actor admitted'; END IF;
    IF (SELECT actor_present FROM pg_temp.guard('actor','tenant-two','foreign')) THEN
        RAISE EXCEPTION 'Cross-realm actor admitted'; END IF;
    -- UNION terminates even for a malformed cycle; ancestors remain protected.
    UPDATE keycloak_group SET parent_group='direct' WHERE id='root';
    IF NOT (SELECT affects_actor FROM pg_temp.guard('actor','tenant-one','root')) THEN
        RAISE EXCEPTION 'Cyclic ancestor bypass'; END IF;
    BEGIN
        WITH new_user AS (
            INSERT INTO user_entity VALUES ('replacement','r','administrator') RETURNING id
        ) INSERT INTO user_group_membership SELECT id,'other' FROM new_user;
        RAISE EXCEPTION 'Import replaced an existing account';
    EXCEPTION WHEN unique_violation THEN NULL;
    END;
    IF (SELECT count(*) FROM user_group_membership WHERE user_id='actor') <> 1
        OR NOT EXISTS(SELECT 1 FROM user_group_membership WHERE user_id='actor' AND group_id='direct')
        OR EXISTS(SELECT 1 FROM user_entity WHERE id='replacement') THEN
        RAISE EXCEPTION 'Duplicate CSV identity changed account or memberships'; END IF;
    WITH new_user AS (
        INSERT INTO user_entity VALUES ('new','r','new-staff') RETURNING id
    ) INSERT INTO user_group_membership SELECT id,'other' FROM new_user;
    IF NOT EXISTS(SELECT 1 FROM user_group_membership WHERE user_id='new' AND group_id='other') THEN
        RAISE EXCEPTION 'New staff import failed'; END IF;
END $$;
ROLLBACK;
"""
subprocess.run(["sudo", "docker", "exec", "-i", "agd-st-postgres", "psql", "-X", "-q",
                "-U", "postgres", "-v", "ON_ERROR_STOP=1"], input=sql, text=True,
               check=True, timeout=10)
print("PASS: direct/ancestor roles denied; independent roles allowed; missing/cross-realm actors denied; cycle bounded; CSV duplicate identity cannot replace memberships; new staff import allowed")
