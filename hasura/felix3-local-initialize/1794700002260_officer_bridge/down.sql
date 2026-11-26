-- SPDX-License-Identifier: AGPL-3.0-only
-- No state to destroy. Never cascade into a different installation.
DROP FUNCTION felix3_initialize_bridge.issuer_pins(felix3_initialize_setup.binding,uuid,uuid,text[],text[],timestamptz);
DROP FUNCTION felix3_initialize_bridge.officer_scope(felix3_initialize_setup.binding,uuid,uuid,text[],text[],timestamptz);
DROP FUNCTION felix3_initialize_bridge.nominee_generation(felix3_initialize_setup.binding,text);
DROP SCHEMA felix3_initialize_bridge;
