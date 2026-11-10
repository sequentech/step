#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Offline AGD-04 command launcher. Public inputs only; never captures command output."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parent
OPERATIONS = ('check', 'install', 'verify', 'commission', 'inspect', 'backup', 'restore', 'services', 'renew-tls',
              'backup-trustees', 'resume-trustees', 'remove-trustees')

def need(condition, message):
    if not condition: raise ValueError(message)

def checked_file(root, name, digest):
    p = root / name
    need(not Path(name).is_absolute() and '..' not in Path(name).parts and p.is_file() and not p.is_symlink(), 'Invalid bundled file')
    need(p.resolve().is_relative_to(root.resolve()), 'Bundled path leaves the package')
    need(hashlib.sha256(p.read_bytes()).hexdigest() == digest, 'Bundled file changed; stop and obtain a verified package')
    return p

def manifest():
    m = json.loads((ROOT / 'manifest.json').read_text())
    need(m['schema'] == 1 and re.fullmatch('[0-9a-f]{40}', m['source_commit']), 'Invalid source identity')
    for name, digest in m['files'].items(): checked_file(ROOT, name, digest)
    return m

def main():
    os.umask(0o077)
    if len(sys.argv) == 3 and sys.argv[1] in ('backup-trustees','resume-trustees','remove-trustees'):
        m=manifest();need(not m.get('test_only'), 'Test-only bundle cannot maintain an installation')
        import trustee
        trustee.server(sys.argv[2],sys.argv[1]);return
    if (len(sys.argv)==3 and sys.argv[1] in ('backup','restore','renew-tls')) or (len(sys.argv) in (4,5) and sys.argv[1]=='services'):
        m=manifest();need(not m.get('test_only'), 'Test-only bundle cannot maintain an installation')
        import maintenance
        maintenance.main(sys.argv[1],sys.argv[2],sys.argv[3] if len(sys.argv)>3 else None,sys.argv[4] if len(sys.argv)>4 else None);return
    if len(sys.argv) in (4, 5) and sys.argv[1] == 'inspect':
        m = manifest()
        need(not m.get('test_only'), 'Test-only bundle cannot inspect an installation')
        import inspection
        inspection.main(sys.argv[2], sys.argv[3], sys.argv[4] if len(sys.argv) == 5 else None)
        return
    if len(sys.argv)==3 and sys.argv[1] in ('check','install','verify','commission'):
        m=manifest()
        need(not m.get('test_only'), 'Test-only bundle cannot install a platform')
        import install
        install.main(sys.argv[1],sys.argv[2]); return
    p = argparse.ArgumentParser(prog='./deploy',description='Install from the administration workstation: ./deploy check site.json; ./deploy install site.json; ./deploy commission site.json. Repeat automated checks with ./deploy verify site.json. AGD-04 describes each operation.')
    p.add_argument('command', choices=OPERATIONS)
    p.add_argument('value', nargs='*'); a = p.parse_args()
    if a.command == 'inspect':p.error('Use ./deploy inspect site.json kit|network|tls|platform|integrity, ./deploy inspect client-network.json client-network, or ./deploy inspect site.json election REALM')
    if a.command == 'services':p.error('Use ./deploy services site.json status|stop|resume-unchanged, or ./deploy services site.json restart|logs COMPONENT')
    need(a.command == 'check' and not a.value, 'Provide the configuration path: ./deploy '+a.command+' site.json')
    manifest(); print('Package hashes match. Independently verify its delivery signature and source identity.')

if __name__ == '__main__':
    try: main()
    except (ValueError, OSError, KeyError, subprocess.CalledProcessError) as error:
        print('Stopped: ' + str(error) if isinstance(error, ValueError) else 'Stopped: invalid configuration or unavailable prerequisite', file=sys.stderr); sys.exit(1)
