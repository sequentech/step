-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only
DROP FUNCTION sequent_backend.execution_security_evidence(uuid,uuid);
ALTER TABLE sequent_backend.attack_detection DROP CONSTRAINT attack_detection_event;
ALTER TABLE sequent_backend.attack_auth_activity DROP CONSTRAINT attack_auth_event;
DO $$ DECLARE tab text; BEGIN
    FOREACH tab IN ARRAY ARRAY['lifecycle_notice','attack_detection_settings','attack_auth_activity','attack_detection'] LOOP
        EXECUTE format('DROP TRIGGER track_security_execution_revision ON sequent_backend.%I',tab);
    END LOOP;
END $$;
DROP FUNCTION sequent_backend.track_security_execution_revision();
