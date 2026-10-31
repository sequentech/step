-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only

-- Enforce the same invariant for Hasura and server import writes. Existing
-- optional/legacy values remain readable; unrelated updates do not rewrite
-- previously stored settings. Alias choices match windmill time_zone_links.rs
-- and canonical_zone (tzdata 2025b); PostgreSQL verifies location existence.
CREATE FUNCTION sequent_backend.canonical_event_timezone(input_name text)
RETURNS text LANGUAGE plpgsql STABLE AS $$
DECLARE
    zone text;
BEGIN
    zone := CASE btrim(input_name)
        WHEN 'Africa/Asmera' THEN 'Africa/Asmara'
        WHEN 'Africa/Timbuktu' THEN 'Africa/Abidjan'
        WHEN 'America/Argentina/ComodRivadavia' THEN 'America/Argentina/Catamarca'
        WHEN 'America/Atka' THEN 'America/Adak'
        WHEN 'America/Buenos_Aires' THEN 'America/Argentina/Buenos_Aires'
        WHEN 'America/Catamarca' THEN 'America/Argentina/Catamarca'
        WHEN 'America/Coral_Harbour' THEN 'America/Atikokan'
        WHEN 'America/Cordoba' THEN 'America/Argentina/Cordoba'
        WHEN 'America/Ensenada' THEN 'America/Tijuana'
        WHEN 'America/Fort_Wayne' THEN 'America/Indiana/Indianapolis'
        WHEN 'America/Godthab' THEN 'America/Nuuk'
        WHEN 'America/Indianapolis' THEN 'America/Indiana/Indianapolis'
        WHEN 'America/Jujuy' THEN 'America/Argentina/Jujuy'
        WHEN 'America/Knox_IN' THEN 'America/Indiana/Knox'
        WHEN 'America/Louisville' THEN 'America/Kentucky/Louisville'
        WHEN 'America/Mendoza' THEN 'America/Argentina/Mendoza'
        WHEN 'America/Montreal' THEN 'America/Toronto'
        WHEN 'America/Nipigon' THEN 'America/Toronto'
        WHEN 'America/Pangnirtung' THEN 'America/Iqaluit'
        WHEN 'America/Porto_Acre' THEN 'America/Rio_Branco'
        WHEN 'America/Rainy_River' THEN 'America/Winnipeg'
        WHEN 'America/Rosario' THEN 'America/Argentina/Cordoba'
        WHEN 'America/Santa_Isabel' THEN 'America/Tijuana'
        WHEN 'America/Shiprock' THEN 'America/Denver'
        WHEN 'America/Thunder_Bay' THEN 'America/Toronto'
        WHEN 'America/Virgin' THEN 'America/Puerto_Rico'
        WHEN 'America/Yellowknife' THEN 'America/Edmonton'
        WHEN 'Antarctica/South_Pole' THEN 'Pacific/Auckland'
        WHEN 'Asia/Ashkhabad' THEN 'Asia/Ashgabat'
        WHEN 'Asia/Calcutta' THEN 'Asia/Kolkata'
        WHEN 'Asia/Choibalsan' THEN 'Asia/Ulaanbaatar'
        WHEN 'Asia/Chongqing' THEN 'Asia/Shanghai'
        WHEN 'Asia/Chungking' THEN 'Asia/Shanghai'
        WHEN 'Asia/Dacca' THEN 'Asia/Dhaka'
        WHEN 'Asia/Harbin' THEN 'Asia/Shanghai'
        WHEN 'Asia/Istanbul' THEN 'Europe/Istanbul'
        WHEN 'Asia/Kashgar' THEN 'Asia/Urumqi'
        WHEN 'Asia/Katmandu' THEN 'Asia/Kathmandu'
        WHEN 'Asia/Macao' THEN 'Asia/Macau'
        WHEN 'Asia/Rangoon' THEN 'Asia/Yangon'
        WHEN 'Asia/Saigon' THEN 'Asia/Ho_Chi_Minh'
        WHEN 'Asia/Tel_Aviv' THEN 'Asia/Jerusalem'
        WHEN 'Asia/Thimbu' THEN 'Asia/Thimphu'
        WHEN 'Asia/Ujung_Pandang' THEN 'Asia/Makassar'
        WHEN 'Asia/Ulan_Bator' THEN 'Asia/Ulaanbaatar'
        WHEN 'Atlantic/Faeroe' THEN 'Atlantic/Faroe'
        WHEN 'Atlantic/Jan_Mayen' THEN 'Europe/Berlin'
        WHEN 'Australia/ACT' THEN 'Australia/Sydney'
        WHEN 'Australia/Canberra' THEN 'Australia/Sydney'
        WHEN 'Australia/Currie' THEN 'Australia/Hobart'
        WHEN 'Australia/LHI' THEN 'Australia/Lord_Howe'
        WHEN 'Australia/NSW' THEN 'Australia/Sydney'
        WHEN 'Australia/North' THEN 'Australia/Darwin'
        WHEN 'Australia/Queensland' THEN 'Australia/Brisbane'
        WHEN 'Australia/South' THEN 'Australia/Adelaide'
        WHEN 'Australia/Tasmania' THEN 'Australia/Hobart'
        WHEN 'Australia/Victoria' THEN 'Australia/Melbourne'
        WHEN 'Australia/West' THEN 'Australia/Perth'
        WHEN 'Australia/Yancowinna' THEN 'Australia/Broken_Hill'
        WHEN 'Brazil/Acre' THEN 'America/Rio_Branco'
        WHEN 'Brazil/DeNoronha' THEN 'America/Noronha'
        WHEN 'Brazil/East' THEN 'America/Sao_Paulo'
        WHEN 'Brazil/West' THEN 'America/Manaus'
        WHEN 'Canada/Atlantic' THEN 'America/Halifax'
        WHEN 'Canada/Central' THEN 'America/Winnipeg'
        WHEN 'Canada/Eastern' THEN 'America/Toronto'
        WHEN 'Canada/Mountain' THEN 'America/Edmonton'
        WHEN 'Canada/Newfoundland' THEN 'America/St_Johns'
        WHEN 'Canada/Pacific' THEN 'America/Vancouver'
        WHEN 'Canada/Saskatchewan' THEN 'America/Regina'
        WHEN 'Canada/Yukon' THEN 'America/Whitehorse'
        WHEN 'Chile/Continental' THEN 'America/Santiago'
        WHEN 'Chile/EasterIsland' THEN 'Pacific/Easter'
        WHEN 'Etc/GMT+0' THEN 'Etc/GMT'
        WHEN 'Etc/GMT-0' THEN 'Etc/GMT'
        WHEN 'Etc/GMT0' THEN 'Etc/GMT'
        WHEN 'Etc/Greenwich' THEN 'Etc/GMT'
        WHEN 'Etc/UCT' THEN 'Etc/UTC'
        WHEN 'Etc/Universal' THEN 'Etc/UTC'
        WHEN 'Etc/Zulu' THEN 'Etc/UTC'
        WHEN 'Europe/Belfast' THEN 'Europe/London'
        WHEN 'Europe/Kiev' THEN 'Europe/Kyiv'
        WHEN 'Europe/Nicosia' THEN 'Asia/Nicosia'
        WHEN 'Europe/Tiraspol' THEN 'Europe/Chisinau'
        WHEN 'Europe/Uzhgorod' THEN 'Europe/Kyiv'
        WHEN 'Europe/Zaporozhye' THEN 'Europe/Kyiv'
        WHEN 'Mexico/BajaNorte' THEN 'America/Tijuana'
        WHEN 'Mexico/BajaSur' THEN 'America/Mazatlan'
        WHEN 'Mexico/General' THEN 'America/Mexico_City'
        WHEN 'Pacific/Enderbury' THEN 'Pacific/Kanton'
        WHEN 'Pacific/Johnston' THEN 'Pacific/Honolulu'
        WHEN 'Pacific/Ponape' THEN 'Pacific/Pohnpei'
        WHEN 'Pacific/Samoa' THEN 'Pacific/Pago_Pago'
        WHEN 'Pacific/Truk' THEN 'Pacific/Chuuk'
        WHEN 'Pacific/Yap' THEN 'Pacific/Port_Moresby'
        WHEN 'US/Alaska' THEN 'America/Anchorage'
        WHEN 'US/Aleutian' THEN 'America/Adak'
        WHEN 'US/Arizona' THEN 'America/Phoenix'
        WHEN 'US/Central' THEN 'America/Chicago'
        WHEN 'US/East-Indiana' THEN 'America/Indiana/Indianapolis'
        WHEN 'US/Eastern' THEN 'America/New_York'
        WHEN 'US/Hawaii' THEN 'Pacific/Honolulu'
        WHEN 'US/Indiana-Starke' THEN 'America/Indiana/Knox'
        WHEN 'US/Michigan' THEN 'America/Detroit'
        WHEN 'US/Mountain' THEN 'America/Denver'
        WHEN 'US/Pacific' THEN 'America/Los_Angeles'
        WHEN 'US/Samoa' THEN 'Pacific/Pago_Pago'
        ELSE btrim(input_name)
    END;
    IF zone IS NULL OR (zone <> 'UTC' AND position('/' IN zone) = 0)
       OR NOT EXISTS (SELECT 1 FROM pg_catalog.pg_timezone_names WHERE name = zone)
       OR zone LIKE 'posix/%' OR zone LIKE 'right/%' THEN
        RAISE EXCEPTION 'Invalid IANA timezone: %', input_name
            USING ERRCODE = '23514', CONSTRAINT = 'event_timezone_valid';
    END IF;
    RETURN zone;
END;
$$;

CREATE FUNCTION sequent_backend.validate_event_timezones()
RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE
    settings jsonb;
    configured jsonb := '[]'::jsonb;
    entry jsonb;
    zone text;
    primary_zone text;
    post_zone text;
BEGIN
    settings := NEW.presentation->'timezones';
    IF NEW.presentation IS NOT NULL AND NEW.presentation <> 'null'::jsonb
       AND jsonb_typeof(NEW.presentation) <> 'object' THEN
        RAISE EXCEPTION 'Election event presentation must be an object'
            USING ERRCODE = '23514', CONSTRAINT = 'event_timezones_valid';
    END IF;
    IF TG_OP = 'UPDATE' AND settings IS NOT DISTINCT FROM OLD.presentation->'timezones' THEN
        RETURN NEW;
    END IF;
    IF settings IS NULL OR settings = 'null'::jsonb THEN
        -- The backwards-compatible absent form means one implicit UTC zone.
        configured := '["UTC"]'::jsonb;
    ELSE
        IF jsonb_typeof(settings) <> 'object'
           OR jsonb_typeof(settings->'configured') IS DISTINCT FROM 'array'
           OR jsonb_typeof(settings->'primary') IS DISTINCT FROM 'string' THEN
            RAISE EXCEPTION 'Timezones require a configured array and a primary IANA zone'
                USING ERRCODE = '23514', CONSTRAINT = 'event_timezones_valid';
        END IF;
        FOR entry IN SELECT value FROM jsonb_array_elements(settings->'configured') LOOP
            IF jsonb_typeof(entry) <> 'string' THEN
                RAISE EXCEPTION 'Configured timezones must be IANA zone names'
                    USING ERRCODE = '23514', CONSTRAINT = 'event_timezones_valid';
            END IF;
            zone := sequent_backend.canonical_event_timezone(entry #>> '{}');
            IF NOT configured ? zone THEN
                configured := configured || jsonb_build_array(zone);
            END IF;
        END LOOP;
        primary_zone := sequent_backend.canonical_event_timezone(settings->>'primary');
        IF jsonb_array_length(configured) = 0 OR NOT configured ? primary_zone THEN
            RAISE EXCEPTION 'At least one timezone is required; primary must be configured'
                USING ERRCODE = '23514', CONSTRAINT = 'event_timezones_valid';
        END IF;
        IF settings ? 'logs' AND (jsonb_typeof(settings->'logs') IS DISTINCT FROM 'string'
           OR settings->>'logs' NOT IN ('primary', 'election')) THEN
            RAISE EXCEPTION 'Log timezone policy must be primary or election'
                USING ERRCODE = '23514', CONSTRAINT = 'event_timezones_valid';
        END IF;
        NEW.presentation := jsonb_set(NEW.presentation, '{timezones}',
            settings || jsonb_build_object('configured', configured, 'primary', primary_zone));
    END IF;
    -- This update holds the parent row lock. Post timezone writes take the
    -- same lock (NOWAIT), so a concurrent assignment cannot race removal.
    FOR post_zone IN
        SELECT presentation->>'timezone' FROM sequent_backend.election
        WHERE tenant_id = NEW.tenant_id AND election_event_id = NEW.id
          AND presentation->'timezone' IS NOT NULL
          AND presentation->'timezone' <> 'null'::jsonb
    LOOP
        IF NOT configured ? sequent_backend.canonical_event_timezone(post_zone) THEN
            RAISE EXCEPTION 'Timezone % is used by a Post and cannot be removed', post_zone
                USING ERRCODE = '23514', CONSTRAINT = 'event_timezone_in_use';
        END IF;
    END LOOP;
    RETURN NEW;
END;
$$;

CREATE FUNCTION sequent_backend.validate_election_timezone()
RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE
    settings jsonb;
    configured jsonb;
    zone text;
BEGIN
    IF NEW.presentation IS NOT NULL AND NEW.presentation <> 'null'::jsonb
       AND jsonb_typeof(NEW.presentation) <> 'object' THEN
        RAISE EXCEPTION 'Post presentation must be an object'
            USING ERRCODE = '23514', CONSTRAINT = 'election_timezone_valid';
    END IF;
    IF TG_OP = 'UPDATE'
       AND NEW.tenant_id = OLD.tenant_id
       AND NEW.election_event_id = OLD.election_event_id
       AND NEW.presentation->'timezone' IS NOT DISTINCT FROM OLD.presentation->'timezone' THEN
        RETURN NEW;
    END IF;
    IF NEW.presentation->'timezone' IS NULL OR NEW.presentation->'timezone' = 'null'::jsonb THEN
        RETURN NEW;
    END IF;
    IF jsonb_typeof(NEW.presentation->'timezone') <> 'string' THEN
        RAISE EXCEPTION 'Post timezone must be an IANA zone name'
            USING ERRCODE = '23514', CONSTRAINT = 'election_timezone_valid';
    END IF;
    zone := sequent_backend.canonical_event_timezone(NEW.presentation->>'timezone');
    -- A child UPDATE already owns its row before this trigger. NOWAIT avoids
    -- a lock inversion with scheduler writes (parent before children). Retry
    -- after the concurrent save/transition finishes; no invalid write commits.
    BEGIN
        SELECT presentation->'timezones' INTO settings
        FROM sequent_backend.election_event
        WHERE tenant_id = NEW.tenant_id AND id = NEW.election_event_id
        FOR UPDATE NOWAIT;
    EXCEPTION WHEN lock_not_available THEN
        RAISE EXCEPTION 'Election event settings are busy; retry the timezone save'
            USING ERRCODE = '55P03';
    END;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'Election event for Post timezone does not exist'
            USING ERRCODE = '23514', CONSTRAINT = 'election_timezone_valid';
    END IF;
    configured := CASE WHEN settings IS NULL OR settings = 'null'::jsonb
        THEN '["UTC"]'::jsonb ELSE settings->'configured' END;
    IF jsonb_typeof(configured) IS DISTINCT FROM 'array' OR NOT configured ? zone THEN
        RAISE EXCEPTION 'Post timezone % is not configured on its election event', zone
            USING ERRCODE = '23514', CONSTRAINT = 'election_timezone_valid';
    END IF;
    NEW.presentation := jsonb_set(NEW.presentation, '{timezone}', to_jsonb(zone));
    RETURN NEW;
END;
$$;

CREATE TRIGGER validate_event_timezones
BEFORE INSERT OR UPDATE OF presentation ON sequent_backend.election_event
FOR EACH ROW EXECUTE FUNCTION sequent_backend.validate_event_timezones();
CREATE TRIGGER validate_election_timezone
BEFORE INSERT OR UPDATE OF presentation, tenant_id, election_event_id ON sequent_backend.election
FOR EACH ROW EXECUTE FUNCTION sequent_backend.validate_election_timezone();
