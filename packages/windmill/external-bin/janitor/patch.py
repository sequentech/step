# SPDX-FileCopyrightText: 2024 Sequent Legal <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

import re
from typing import Any, Union
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


def patch_dict(data: dict, path: str, value: Any) -> None:
    """
    Update a nested dictionary (and lists) given a dotted path with optional array indices.

    Args:
        data (dict): The dictionary to update.
        path (str): A dotted path representing nested keys and indices, e.g., "a.b[0].c[5]".
        value (any): The value to set at the nested key/index.

    Example:
        data = {}
        patch_dict(data, "a.b[0].c[5]", 42)
        # data is now {"a": {"b": [{"c": [None, None, None, None, None, 42]}]}}
    """
    token_regex = re.compile(r'([^[.\]]+)|\[(\d+)\]')
    tokens = token_regex.findall(path)

    current: Union[dict, list] = data
    for i, (key, index) in enumerate(tokens):
        is_last = (i == len(tokens) - 1)

        if key:  # Dictionary key
            if is_last:
                current[key] = value
            else:
                if key not in current or not isinstance(current[key], (dict, list)):
                    next_key, next_index = tokens[i + 1]
                    if next_index:  # Next token is a list index
                        current[key] = []
                    else:
                        current[key] = {}
                current = current[key]
        elif index:  # List index
            idx = int(index)
            if not isinstance(current, list):
                raise TypeError(f"Expected list at this part of the path, but got {type(current).__name__}")

            while len(current) <= idx:
                current.append(None)
            
            if is_last:
                current[idx] = value
            else:
                if current[idx] is None or not isinstance(current[idx], (dict, list)):
                    next_key, next_index = tokens[i + 1]
                    if next_index:
                        current[idx] = []
                    else:
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
    patch_json_with_excel(excel_data, json_data, args.parameters_type)

    write_path = args.json_path if args.overwrite else args.json_path + ".new"
    write_json(json_data, write_path)


if __name__ == "__main__":
    main()
