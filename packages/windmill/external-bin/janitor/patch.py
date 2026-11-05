# SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

import re
from datetime import datetime, timezone as utc_timezone
from typing import Any, Union, List, Optional
from zoneinfo import ZoneInfo, ZoneInfoNotFoundError

from time_zone_links import TIME_ZONE_LINKS
import json
import argparse

def parse_table_sheet(
    sheet,
    required_keys=[],
    allowed_keys=[],
    map_f=lambda value: value
):
    '''
    Reads a CSV table and returns it as a list of dict items.
    '''
    def check_required_keys(header_values, required_keys):
        '''
        Check that each required_key pattern appears in header_values
        '''
        matched_patterns = set()
        for key in header_values:
            for pattern in required_keys:
                if re.match(pattern, key):
                    matched_patterns.add(pattern)
                    break
        assert(len(matched_patterns) == len(required_keys))

    def check_allowed_keys(header_values, allowed_keys):
        allowed_keys += [
            r"^name$",
            r"^alias$",
            r"^annotations\.[_a-zA-Z0-9]+",
        ]
        matched_patterns = set()
        for key in header_values:
            found = False
            for pattern in allowed_keys:
                if re.match(pattern, key):
                    matched_patterns.add(pattern)
                    found = True
                    break
            if not found:
                raise Exception(f"header {key} not allowed")

    def parse_line(header_values, line_values):
        '''
        Once all keys are validated, let's parse them in the desired structure
        '''
        parsed_object = dict()
        for (key, value) in zip(header_values, line_values):
            split_key = key.split('.')
            subelement = parsed_object
            for split_key_index, split_key_item in enumerate(split_key):
                # if it's not last
                if split_key_index == len(split_key) - 1:
                    if isinstance(value, float):
                        subelement[split_key_item] = int(value)
                    else:
                        subelement[split_key_item] = value
                else:
                    if split_key_item not in subelement:
                        subelement[split_key_item] = dict()
                    subelement = subelement[split_key_item]

        return map_f(parsed_object)

    def sanitize_values(values):
        return [
            sanitize_value(value)
            for value in values
        ]

    def sanitize_value(value):
        return value.strip() if isinstance(value, str) else value

    # Get header and check required and allowed keys
    header_values = None
    ret_data = []
    for row in sheet.values:
        sanitized_row = sanitize_values(row)
        all_none = all(item is None for item in sanitized_row)
        if all_none:
            continue
        if not header_values:
            header_values = [
                value
                for value in sanitized_row
                if value is not None
            ]
            check_required_keys(header_values, required_keys)
            check_allowed_keys(header_values, allowed_keys)
        else:
            ret_data.append(
                parse_line(header_values, sanitized_row)
            )

    return ret_data

def parse_parameters(sheet):
    data = parse_table_sheet(
        sheet,
        required_keys=[
            "^type$",
            "^key$",
            "^value$"
        ],
        allowed_keys=[
            "^type$",
            "^key$",
            "^value$"
        ]
    )
    return data

def parse_excel(excel_path):
    '''
    Parse all input files specified in the config file into their respective
    data structures.
    '''
    import openpyxl

    electoral_data = openpyxl.load_workbook(excel_path)

    return dict(
        parameters = parse_parameters(electoral_data['Parameters']),
    )

def load_json(file_path: str) -> dict:
    with open(file_path, 'r', encoding='utf-8') as file:
        return json.load(file)

def write_json(data: Any, file_path: str) -> None:
    with open(file_path, 'w', encoding='utf-8') as file:
        json.dump(data, file, indent=4, ensure_ascii=False)


def parse_path(path: str) -> List[Union[str, int]]:
    """
    Parses a dotted/bracketed path (with support for escaped dots)
    into a list of tokens. Each token is either:
      - a string (dictionary key)
      - an integer (list index)

    For example:
      "something.other.this\\.has\\.dots[0].more" ->
        ["something", "other", "this.has.dots", 0, "more"]
    """
    tokens: List[Union[str, int]] = []
    current = []   # holds characters of the current key
    i = 0
    length = len(path)

    while i < length:
        c = path[i]

        if c == '\\':
            # Escape character -> take next char as literal, if any
            i += 1
            if i < length:
                current.append(path[i])
        elif c == '.':
            # Dot is a key separator -> finalize current token as a string
            if current:
                tokens.append("".join(current))
                current = []
        elif c == '[':
            # Bracket indicates a list index
            # First push whatever we have (as dictionary key) if anything
            if current:
                tokens.append("".join(current))
                current = []
            # Find the matching ']'
            close_idx = path.find(']', i)
            if close_idx == -1:
                raise ValueError(f"Unmatched '[' at position {i} in '{path}'")
            # Extract the index substring
            idx_str = path[i+1:close_idx]
            try:
                idx = int(idx_str)
            except ValueError:
                raise ValueError(f"Invalid list index '{idx_str}' in path '{path}'")
            tokens.append(idx)
            i = close_idx  # Skip to the closing ']'
        else:
            # Normal character -> accumulate in current token
            current.append(c)

        i += 1

    # If there's something left in current, push it as a final token
    if current:
        tokens.append("".join(current))

    return tokens

def patch_dict(data: dict, path: str, value: Any) -> None:
    """
    Update a nested dictionary (and lists) given a dotted path (which can contain
    escaped dots) with optional array indices.

    Args:
        data (dict): The dictionary to update.
        path (str): A dotted path representing nested keys/indices, e.g., 
                    "a.b[0].c[5]" or "a.this\\.contains\\.dots[3].something".
        value (any): The value to set at the nested key/index.

    Example:
        data = {}
        patch_dict(data, "a.b[0].c[5]", 42)
        # data becomes {"a": {"b": [{"c": [None, None, None, None, None, 42]}]}}

        data = {}
        patch_dict(data, "some.key.this\\.has\\.dots[2]", "hello")
        # data becomes {"some": {"key": {"this.has.dots": [None, None, "hello"]}}}
    """
    tokens = parse_path(path)
    current: Union[dict, list] = data

    for i, token in enumerate(tokens):
        is_last = (i == len(tokens) - 1)

        if isinstance(token, str):
            # Dictionary key
            if is_last:
                current[token] = value
            else:
                # Look ahead to see if next token is an int (list) or a string (dict)
                next_token = tokens[i + 1]
                if isinstance(next_token, int):
                    # Next is a list index
                    if token not in current or not isinstance(current[token], list):
                        current[token] = []
                    current = current[token]
                else:
                    # Next is a dict key
                    if token not in current or not isinstance(current[token], dict):
                        current[token] = {}
                    current = current[token]

        else:
            # token is an int -> list index
            idx = token
            if not isinstance(current, list):
                raise TypeError(
                    f"Expected a list at this part of the path, but got {type(current).__name__}"
                )
            # Ensure list is large enough
            while len(current) <= idx:
                current.append(None)

            if is_last:
                current[idx] = value
            else:
                next_token = tokens[i + 1]
                if isinstance(next_token, int):
                    # Next is list as well
                    if current[idx] is None or not isinstance(current[idx], list):
                        current[idx] = []
                else:
                    # Next is dict key
                    if current[idx] is None or not isinstance(current[idx], dict):
                        current[idx] = {}
                current = current[idx]

def parse_cell_value(cell_value: Any) -> Any:
    """
    Interprets the raw cell value from Excel, trying to:
      - Keep numeric cells as numeric types
      - Convert the literal string "null" to None
      - Parse valid JSON strings into dict/list/string/number/etc.
      - Otherwise keep it as a literal string
    """
    if cell_value is None:
        return None

    # If the cell is already a numeric type, just return it
    if isinstance(cell_value, (int, float)):
        return cell_value

    # If it's a string, let's handle some special cases
    if isinstance(cell_value, str):
        trimmed = cell_value.strip()

        # The literal string "null" => JSON null
        if trimmed.lower() == "null":
            return None

        # Try to interpret it as JSON
        # If it parses successfully, we'll use the parsed object.
        # For example, a cell containing {"a":1} will become a dict,
        # a cell containing [1,2,3] becomes a list,
        # a cell containing "2" becomes a string "2",
        # etc.
        try:
            parsed = json.loads(trimmed)
            return parsed
        except json.JSONDecodeError:
            # If it's not valid JSON, treat it as a plain string
            return cell_value

    # Fallback: return as-is
    return cell_value

def parse_excel(excel_path: str) -> dict:
    '''
    Parse all input files specified in the config file into their respective
    data structures.
    '''
    import openpyxl

    electoral_data = openpyxl.load_workbook(excel_path)

    return dict(
        parameters = parse_parameters(electoral_data['Parameters']),
    )

def patch_json_with_excel(excel_data, json_data, parameters_type):
    parameters_data = [t for t in excel_data["parameters"] if t["type"] == parameters_type]
    for row in parameters_data:
        key = row["key"]
        value = parse_cell_value(row["value"])
        print(f"Patching key {key} with value {value}")
        patch_dict(json_data, key, value)

# --- Scheduled events in local time (VOTE-LIFECYCLE) ------------------------
#
# The same rules as the platform (`sequent_core::time_zones` and windmill's
# `services/time_zones.rs`): a row's zone is its own, else its election's
# when that zone is configured on the event, else the event's primary, else
# UTC. A wall time is stored with its zone and the instant it runs.

DEFAULT_TIME_ZONE = "UTC"

# Old names and their tzdata canonical names, as windmill's `ALIASES`.
TIME_ZONE_ALIASES = {
    "Asia/Calcutta": "Asia/Kolkata",
    "Asia/Katmandu": "Asia/Kathmandu",
    "Asia/Saigon": "Asia/Ho_Chi_Minh",
    "Asia/Rangoon": "Asia/Yangon",
    "Asia/Dacca": "Asia/Dhaka",
    "Asia/Ulan_Bator": "Asia/Ulaanbaatar",
    "Europe/Kiev": "Europe/Kyiv",
    "America/Buenos_Aires": "America/Argentina/Buenos_Aires",
    "America/Godthab": "America/Nuuk",
    "Atlantic/Faeroe": "Atlantic/Faroe",
    "Pacific/Enderbury": "Pacific/Kanton",
    "Pacific/Truk": "Pacific/Chuuk",
    "Pacific/Ponape": "Pacific/Pohnpei",
}


def canonical_zone(name: str) -> str:
    """
    The tzdata canonical name of a zone, as windmill's `canonical_time_zone`:
    links (`US/Eastern`) become the zone they name; `UTC` stays; any other
    name without a `/` (`EST`, `Japan`) is refused.
    """
    name = name.strip()
    name = TIME_ZONE_ALIASES.get(name, name)
    if name == "UTC":
        return name
    if "/" not in name:
        raise ValueError(f"Not a timezone (abbreviations aren't): {name!r}")
    return TIME_ZONE_LINKS.get(name, name)


def parse_zone(name: str) -> ZoneInfo:
    try:
        return ZoneInfo(canonical_zone(name))
    except (ZoneInfoNotFoundError, ValueError):
        raise ValueError(f"Unknown timezone: {name!r}")


def _event_time_zones(election_event: dict) -> dict:
    return ((election_event or {}).get("presentation") or {}).get("timezones") or {}


def primary_time_zone(election_event: dict) -> str:
    primary = (_event_time_zones(election_event).get("primary") or "").strip()
    return primary or DEFAULT_TIME_ZONE


def configured_time_zones(election_event: dict) -> list:
    """The event's configured zones, canonical."""
    return [canonical_zone(zone) for zone in _event_time_zones(election_event).get("configured") or []]


def effective_time_zone(election_event: dict, election: Optional[dict]) -> str:
    zone = ((election or {}).get("presentation") or {}).get("timezone")
    if isinstance(zone, str) and canonical_zone(zone) in configured_time_zones(election_event):
        return canonical_zone(zone)
    return canonical_zone(primary_time_zone(election_event))


def parse_schedule_date(value: Any):
    """
    A date of the ScheduledEvents sheet: a datetime with an offset is an
    instant (as before); one without is a wall time. Returns
    (naive wall time, None) or (None, aware instant).
    """
    if isinstance(value, datetime):
        moment = value
    else:
        text = str(value).strip().replace(" ", "T", 1)
        # Python before 3.11 doesn't read a trailing `Z`.
        if text.endswith(("Z", "z")):
            text = text[:-1] + "+00:00"
        try:
            moment = datetime.fromisoformat(text)
        except ValueError:
            raise ValueError(f"Invalid scheduled event date: {value!r}")
    if moment.tzinfo is None:
        return moment.replace(microsecond=0), None
    return None, moment


def resolve_local(local: datetime, zone: ZoneInfo):
    """
    The instant of a wall time in a zone, and "exact", "gap" (it doesn't
    exist: clocks go forward) or "overlap" (it happens twice: the first is
    used).
    """
    first = local.replace(tzinfo=zone, fold=0)
    second = local.replace(tzinfo=zone, fold=1)
    instant = first.astimezone(utc_timezone.utc)
    if first.utcoffset() == second.utcoffset():
        return instant, "exact"
    if instant.astimezone(zone).replace(tzinfo=None) != local:
        return instant, "gap"
    return instant, "overlap"


def format_local(local: datetime) -> str:
    return local.strftime("%Y-%m-%dT%H:%M" if local.second == 0 else "%Y-%m-%dT%H:%M:%S")


def format_instant(instant: datetime, zone: ZoneInfo) -> str:
    """RFC 3339 with the zone's offset, `Z` for UTC, as windmill writes it."""
    text = instant.astimezone(zone).isoformat(timespec="seconds")
    return text[:-6] + "Z" if text.endswith("+00:00") else text


def schedule_cron_config(date: Any, row_zone: Optional[str], zone_name: str) -> dict:
    """
    The cron_config of a scheduled event: the instant (`scheduled_date`), the
    wall time (`local`) and its zone (`timezone`). `row_zone` is the sheet's
    `timezone` cell; without one, `zone_name` (the election's zone) applies.
    A wall time that doesn't exist in the zone is refused.
    """
    name = row_zone.strip() if isinstance(row_zone, str) and row_zone.strip() else zone_name
    zone = parse_zone(name)
    local, instant = parse_schedule_date(date)
    if instant is None:
        instant, kind = resolve_local(local, zone)
        if kind == "gap":
            raise ValueError(
                f"{format_local(local)} doesn't exist in {zone.key}: clocks go forward then"
            )
        if kind == "overlap":
            print(f"Note: {format_local(local)} happens twice in {zone.key}; the first is used")
    else:
        local = instant.astimezone(zone).replace(tzinfo=None)
    return {
        "cron": None,
        "scheduled_date": format_instant(instant, zone),
        "local": format_local(local),
        "timezone": zone.key,
    }


def apply_schedule_time_zones(bundle: dict) -> None:
    """
    Fills the cron_config of every scheduled event of an election event
    bundle from its date and zone. Run after the Parameters are patched in,
    so the event's timezones are final.
    """
    election_event = bundle.get("election_event") or {}
    elections = {election.get("id"): election for election in bundle.get("elections") or []}
    configured = configured_time_zones(election_event)
    for election in elections.values():
        zone = (election.get("presentation") or {}).get("timezone")
        if zone and canonical_zone(zone) not in configured:
            # The platform would use the primary instead: say so now.
            raise ValueError(
                f"Post {election.get('alias')!r} has timezone {zone!r}, which isn't one of "
                f"the event's configured timezones {configured}"
            )
    for scheduled_event in bundle.get("scheduled_events") or []:
        cron_config = scheduled_event.get("cron_config") or {}
        election_id = (scheduled_event.get("event_payload") or {}).get("election_id")
        election = elections.get(election_id) if election_id else None
        scheduled_event["cron_config"] = schedule_cron_config(
            cron_config.get("scheduled_date"),
            cron_config.get("timezone"),
            effective_time_zone(election_event, election),
        )


def main():
    parser = argparse.ArgumentParser(description="patch a json with data from an excel")
    parser.add_argument('json_path', type=str, help='json path')
    parser.add_argument('excel_path', type=str, help='excel')
    parser.add_argument('parameters_type', type=str, help='parameters type')
    parser.add_argument('--overwrite', action='store_true',
                        help='If set, overwrite the original JSON file instead of creating a new file')

    args = parser.parse_args()

    excel_data = parse_excel(args.excel_path)
    json_data = load_json(args.json_path)
    final_json = {
        "tenant_configurations": {},
        "keycloak_admin_realm": json_data
    }
    patch_json_with_excel(excel_data, final_json, args.parameters_type)

    write_path = args.json_path if args.overwrite else args.json_path + ".new"
    write_json(final_json["keycloak_admin_realm"], write_path)


if __name__ == "__main__":
    main()
