# SPDX-FileCopyrightText: 2024 Sequent Legal <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

import re
from typing import Any, Union, List
import openpyxl
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
