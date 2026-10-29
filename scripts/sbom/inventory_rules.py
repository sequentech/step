# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Extract concrete image references only in syntactically appropriate files."""
from pathlib import PurePosixPath
import re
import subprocess

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


def image_version(reference):
    """A registry port or digest is not an image tag."""
    name = reference.split('@', 1)[0].rsplit('/', 1)[-1]
    return name.rsplit(':', 1)[1] if ':' in name else 'unversioned'


def gitlink(repository, path):
    """Read a committed submodule pin, never an assumed checkout pairing."""
    entry = subprocess.check_output(
        ['git', '-C', str(repository), 'ls-tree', 'HEAD', '--', path], text=True
    ).strip().split()
    if len(entry) == 4 and entry[:2] == ['160000', 'commit'] and entry[3] == path:
        return entry[2]
    return None


def evidence_review_status(status, profile):
    """License evidence cannot approve ownership, scope or other review gates."""
    if profile == 'UNRESOLVED':
        return 'Needs confirmation'
    if status == 'Source evidence collected':
        return 'Source license evidence recorded'
    return status
