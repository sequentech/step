# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Extract concrete image references only in syntactically appropriate files."""
from pathlib import PurePosixPath
import re

def image_reference(filename, line):
    path = PurePosixPath(filename)
    if path.name.startswith('Dockerfile') or path.name.endswith('.Dockerfile'):
        match = re.match(r'^\s*FROM\s+(?:--platform=\S+\s+)?(\S+)', line, re.I)
    elif path.suffix in ('.yaml', '.yml'):
        match = re.match(r'^\s*image:\s*[\"\x27]?([^\s\"\x27]+)', line)
    else:
        return None
    if not match:
        return None
    ref = match.group(1)
    if ref == 'scratch' or not re.fullmatch(r'[a-zA-Z0-9][a-zA-Z0-9._/:@+-]*', ref):
        return None
    return ref
