#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

"""Scan explicitly collected ballot artifacts for synthetic private values.

This offline check never discovers files, contacts services, or resolves voters.
PDFs must first be extracted to UTF-8 text. Mixed authentication logs are outside
its scope: their actor identities are legitimate authentication audit evidence.
"""
import argparse
import base64
import json
import re
import sys
from pathlib import Path

CATEGORIES = frozenset({"voter_id", "username", "ip", "country", "device", "credential"})
BASE64_FIELDS = frozenset({"received_entry", "cast_entry", "message"})
MAX_JSON_DEPTH = 32


def read_forbidden(path):
    values = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(values, dict) or not values or set(values) - CATEGORIES:
        raise ValueError("Invalid forbidden-value categories")
    if any(not isinstance(items, list) or not items or
           any(not isinstance(item, str) or not item for item in items)
           for items in values.values()):
        raise ValueError("Forbidden values must be nonempty string lists")
    return values


def decoded_texts(value, depth=0):
    """Inspect decoded JSON strings and canonical signed-message base64 too."""
    if depth > MAX_JSON_DEPTH:
        raise ValueError("Nested artifact exceeds supported depth")
    if isinstance(value, dict):
        for key, item in value.items():
            yield str(key)
            if key in BASE64_FIELDS and isinstance(item, str):
                try:
                    yield base64.b64decode(item, validate=True).decode("utf-8", errors="replace")
                except (ValueError, UnicodeError):
                    pass
            yield from decoded_texts(item, depth + 1)
    elif isinstance(value, list):
        for item in value:
            yield from decoded_texts(item, depth + 1)
    elif isinstance(value, str):
        yield value
        if value.lstrip().startswith(("{", "[")):
            try:
                nested = json.loads(value)
            except ValueError:
                return
            yield from decoded_texts(nested, depth + 1)


def scan(path, forbidden):
    if path.suffix.lower() == ".pdf":
        raise ValueError("Extract PDF text before scanning")
    raw = path.read_text(encoding="utf-8")
    texts = [raw]
    if path.suffix.lower() == ".json":
        texts.extend(decoded_texts(json.loads(raw)))
    found = set()
    for category, values in forbidden.items():
        for value in values:
            # Country codes are short; match complete codes, not substrings of
            # field names such as "user_id". Other values are exact substrings.
            pattern = re.compile(r"(?<![A-Za-z0-9])" + re.escape(value) + r"(?![A-Za-z0-9])")
            if any((pattern.search(text) if category == "country" else value in text) for text in texts):
                found.add(category)
                break
    return sorted(found)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--forbidden", required=True, type=Path,
                        help="Local JSON mapping categories to synthetic private values")
    parser.add_argument("artifacts", nargs="+", type=Path,
                        help="Explicit ballot JSON/CSV/plain text or extracted PDF text files")
    args = parser.parse_args(argv)
    try:
        forbidden = read_forbidden(args.forbidden)
    except (OSError, ValueError, UnicodeError):
        print("ERROR forbidden configuration", file=sys.stderr)
        return 2
    failures = False
    errors = False
    for artifact in args.artifacts:
        try:
            categories = scan(artifact, forbidden)
        except (OSError, ValueError, UnicodeError):
            print(f"ERROR {artifact.name}: unreadable or unsupported artifact", file=sys.stderr)
            errors = True
            continue
        if categories:
            failures = True
            print(f"FAIL {artifact.name}: {', '.join(categories)}")
        else:
            print(f"PASS {artifact.name}")
    return 2 if errors else 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
