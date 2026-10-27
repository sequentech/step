#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Validate an entire decrypted tar stream without writing plaintext to disk."""
import sys
import json
import tarfile
from pathlib import PurePosixPath

ROOTS={'system':('root/deploy','var/lib/sequent-data'),'trustee':('trustee1','trustee2')}

def validate(stream,kind):
    if kind not in ROOTS:raise ValueError('Choose system or trustee backup validation')
    roots=ROOTS[kind];seen=set();found=set();metadata=None
    with tarfile.open(fileobj=stream,mode='r|') as archive:
        for item in archive:
            name=item.name.rstrip('/')
            path=PurePosixPath(name)
            if not name or path.is_absolute() or '..' in path.parts or path.as_posix()!=name or name in seen:
                raise ValueError('Backup contains an unsafe or repeated path')
            root=next((r for r in roots if name==r or name.startswith(r+'/')),None)
            if root is None:raise ValueError('Backup contains a path outside its permitted directories')
            if not (item.isfile() or item.isdir()):raise ValueError('Backup contains a link or special file; do not restore it with this procedure')
            seen.add(name);found.add(root)
            if item.isfile():
                data=archive.extractfile(item)
                if data is None:raise ValueError('Backup file content is unavailable')
                remaining=item.size
                if name=='root/deploy/.backup-identity.json':
                    if remaining>65536:raise ValueError('Backup identity is too large')
                    raw=data.read(remaining)
                    if len(raw)!=remaining:raise ValueError('Backup identity is truncated')
                    metadata=json.loads(raw);remaining=0
                while remaining:
                    block=data.read(min(remaining,1024*1024))
                    if not block:raise ValueError('Backup file content is truncated')
                    remaining-=len(block)
    # Consume the whole pipe so GPG authenticates its complete encrypted input.
    while stream.read(1024*1024):pass
    if found!=set(roots):raise ValueError('Backup is missing a required directory')
    return metadata

if __name__=='__main__':
    try:
        if len(sys.argv)!=2:raise ValueError('Use: gpg --decrypt BACKUP | python3 ./archive_check.py system|trustee')
        validate(sys.stdin.buffer,sys.argv[1])
        print('BACKUP CONTENTS VERIFIED: '+sys.argv[1])
    except (ValueError,tarfile.TarError,OSError) as error:
        print('BACKUP VALIDATION FAILED: '+str(error),file=sys.stderr);sys.exit(1)
