-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only
SELECT current_user=session_user AND current_user='os14990_queue'
            AND current_database()='operational_demo_queues_14990' AND inet_client_addr() IS NULL
            AND EXISTS (SELECT FROM pg_roles r WHERE r.rolname=current_user
                AND NOT (rolsuper OR rolcreaterole OR rolcreatedb OR rolreplication OR rolbypassrls))
            AND (SELECT environment FROM step_queue.installation)='operational_demo_14990'
            AND (SELECT count(*) FROM pgmq.meta WHERE queue_name IN ('short_queue','electoral_log_beat_queue'))=2
