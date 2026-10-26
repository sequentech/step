#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Offline AGD-04 command launcher. Public inputs only; never captures command output."""
import argparse
import hashlib
import ipaddress
import json
import os
from pathlib import Path
import re
import shlex
import stat
import shutil
import subprocess
import sys

ROOT = Path(__file__).resolve().parent
WORK = Path('/root/deploy')
CONFIG = WORK / 'operator.json'
ROLES = ('admin', 'c1', 'c2', 'client', 'trustee')
SITE = {'voting-portal-host': 'PORTAL_HOST', 'administration-portal-host': 'ADMIN_HOST',
        'ballot-verifier-host': 'VERIFIER_HOST', 'authentication-server-host': 'AUTH_HOST',
        'alias-of-the-authentication-server': 'IDP_ALIAS', 'address-of-c1': 'C1', 'address-of-c2': 'C2'}
TOKENS = re.compile(r'<([a-z][a-z0-9 -]*)>')

def need(condition, message):
    if not condition: raise ValueError(message)

def validate_value(name, value):
    need(isinstance(value, str) and 0 < len(value) <= 2048, 'A public value is required')
    need(bool(re.fullmatch(r'[A-Za-z0-9_@./:+,= -]+', value)), 'Use a literal public value without shell punctuation')
    if name.startswith('address-of-'): ipaddress.ip_address(value)
    elif name.endswith('-host'): need(bool(re.fullmatch(r'[A-Za-z0-9][A-Za-z0-9.-]*', value)), 'A DNS hostname is required')
    elif name in ('issuer', 'authorization-endpoint', 'token-endpoint', 'jwks-url', 'end-session-endpoint'):
        need(value.startswith('https://') and ' ' not in value, 'An HTTPS URL is required')
    elif 'path' in name or name == 'backup-medium':
        need(value.startswith('/') and ' ' not in value, 'An absolute path without spaces is required')
    elif name == 'native-trustee-number': need(value in ('1', '2'), 'Choose native trustee 1 or 2')
    elif name == 'ports-printed-on-c1':
        need(all(p.isdigit() and 0 < int(p) < 65536 for p in value.split()), 'Use the observed port numbers')
    else: need(' ' not in value or name == 'names of the election event realms, separated by spaces', 'This identifier cannot contain spaces')
    return value

def numeric_id(entry):
    return entry.get('legacy_id', entry['id'])

def step_entries(m):
    entries = {}
    for entry in m['steps']:
        for name in {entry['id'], numeric_id(entry)}:
            need(name not in entries, 'Duplicate step name or alias')
            entries[name] = entry
    return entries

def confirmation_for(entry):
    return {'08-014': 'REMOVE TRUSTEE KEYS',
            '11-003': 'RESTORE AUTHORIZED BACKUP'}.get(numeric_id(entry))

def require_role(entry, role):
    need(role in entry['roles'], 'Wrong session: use ' + '/'.join(entry['roles']))

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

def configuration():
    need(CONFIG.is_file() and not CONFIG.is_symlink(), 'Run ./deploy configure first')
    need(CONFIG.stat().st_uid == 0 and stat.S_IMODE(CONFIG.stat().st_mode) == 0o600, 'operator.json must be root-owned mode 600')
    d = json.loads(CONFIG.read_text())
    for name in SITE: validate_value(name, d['site'][name])
    return d

def write_config(d):
    WORK.mkdir(mode=0o700, exist_ok=True)
    need(not WORK.is_symlink() and WORK.stat().st_uid == 0 and not (WORK.stat().st_mode & 0o077), '/root/deploy must be root-only')
    tmp = WORK / '.operator.new'
    with tmp.open('x') as f: json.dump(d, f, indent=2); f.write('\n')
    tmp.chmod(0o600); tmp.replace(CONFIG)

def prompt(label, default=''):
    with open('/dev/tty', 'r+') as tty:
        tty.write(label + (f' [{default}]' if default else '') + ': '); tty.flush()
        value = tty.readline().strip()
    return value or default

def configure(role):
    d = configuration() if CONFIG.exists() else {'site': {}, 'parameters': {}}
    for name in SITE:
        d['site'][name] = validate_value(name, prompt(name, d['site'].get(name, '')))
    if role == 'admin':
        d['kit_directory'] = validate_value('path-to-kit', prompt('Verified installation kit directory', d.get('kit_directory', '')))
    d['role'] = role; d['machine_id'] = Path('/etc/machine-id').read_text().strip()
    env = ''.join(f'export {key}={shlex.quote(d["site"][name])}\n' for name, key in SITE.items())
    env += 'export R=tenant-90505c8a-23a9-4cdf-a26b-4e19f6a097d5\n'
    hosts = WORK / 'hosts.env'
    if hosts.exists():
        need(not hosts.is_symlink() and hosts.stat().st_uid == 0 and not hosts.stat().st_mode & 0o077, 'hosts.env must be root-only')
        existing = hosts.read_text()
        for name, key in SITE.items():
            need(re.search(r'^(?:export )?' + key + '=' + re.escape(shlex.quote(d['site'][name])) + r'$', existing, re.M), 'Existing hosts.env differs: reconcile the public configuration before proceeding')
    write_config(d)
    if not hosts.exists(): hosts.write_text(env); hosts.chmod(0o600)
    print('Public configuration saved. No passwords, keys or client secrets belong in operator.json.')

def verify_host(d, role):
    need(d['role'] == role and d['machine_id'] == Path('/etc/machine-id').read_text().strip(), 'Configure this machine for the requested session')
    addresses = {a['local'] for row in json.loads(subprocess.check_output(['ip', '-j', 'address'])) for a in row.get('addr_info', [])}
    if role in ('c1', 'c2'):
        need(d['site']['address-of-' + role] in addresses, 'This machine does not have the configured server address')
    else:
        need(not addresses.intersection({d['site']['address-of-c1'], d['site']['address-of-c2']}), 'A workstation session cannot run on a cluster server')

def substitute(body, values):
    for name in set(TOKENS.findall(body)):
        need(name in values, 'Unresolved public input: ' + name)
        body = body.replace('<' + name + '>', validate_value(name, values[name]))
    return body

def prepare(entry, d, role):
    require_role(entry, role); verify_host(d, role)
    original = checked_file(ROOT, entry['file'], entry['sha256']).read_text()
    if entry['language'] != 'bash':
        need(entry.get('destination'), 'This block is expected output; read it with show')
        p = Path(entry['destination']); need(p.parent == WORK, 'Unexpected data destination')
        if p.exists(): need(not p.is_symlink() and p.read_text() == original, 'Existing filter differs; review it before replacement')
        else:
            with p.open('x') as f: f.write(original)
            p.chmod(0o600)
        body = 'printf "%s\\n" "Reviewed filter installed: ' + str(p) + '"\n'
    else:
        values = {**d['site'], **d['parameters'].get(numeric_id(entry), {}), **d['parameters'].get(entry['id'], {})}
        for name in sorted(set(TOKENS.findall(original))):
            if name not in d['site']:
                values[name] = validate_value(name, prompt(name + ' (public)', values.get(name, '')))
        d['parameters'][entry['id']] = {n: values[n] for n in TOKENS.findall(original) if n not in d['site']}; write_config(d)
        body = substitute(original, values)
    confirmation = confirmation_for(entry)
    if confirmation:
        reason = ('Only after retention and board authorization' if numeric_id(entry) == '08-014'
                  else 'Only with board restore authorization and stopped services')
        need(prompt(reason + '; type ' + confirmation) == confirmation, 'Destructive operation cancelled')
    target = WORK / '.commands'; target.mkdir(mode=0o700, exist_ok=True)
    need(not target.is_symlink() and target.stat().st_uid == 0 and not (target.stat().st_mode & 0o077), 'Private command directory required')
    p = target / (entry['id'] + '.sh'); need(not p.is_symlink(), 'Command path is a symlink')
    p.write_text(body); p.chmod(0o600)
    print(p)

def main():
    os.umask(0o077)
    if len(sys.argv)==3 and sys.argv[1] in ('check','install','verify','commission'):
        m=manifest()
        need(not m.get('test_only'), 'Test-only bundle cannot install a platform')
        import install
        install.main(sys.argv[1],sys.argv[2]); return
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('command', choices=['check', 'list', 'show', 'configure', 'session', 'prepare'])
    p.add_argument('value', nargs='?'); a = p.parse_args(); m = manifest()
    if a.command == 'check': print('Package hashes match. Independently verify its delivery signature and source identity.'); return
    if a.command == 'list':
        for e in m['steps']: print(e['id'], '/'.join(e['roles']), e['heading'])
        return
    entries = step_entries(m)
    if a.command == 'show':
        need(a.value in entries, 'Unknown step'); e = entries[a.value]
        print(e['id'], '/'.join(e['roles']), e['heading'], '\nSource:', e['source'], 'line', e['line'])
        print(checked_file(ROOT, e['file'], e['sha256']).read_text()); return
    need(not m.get('test_only'), 'Test-only bundle cannot configure or execute deployment')
    need(os.geteuid() == 0, 'Use a root shell on the machine named by the guide')
    if a.command == 'configure':
        need(a.value in ROLES, 'Choose admin, c1, c2, client or trustee'); configure(a.value); return
    d = configuration()
    if a.command == 'session':
        need(a.value in ROLES, 'Choose a session role'); verify_host(d, a.value)
        env = {'PATH': os.defpath + ':/run/current-system/sw/bin', 'HOME': '/root', 'TERM': os.environ.get('TERM', 'xterm'), 'AGD_BUNDLE': str(ROOT), 'AGD_ROLE': a.value}
        os.chdir(d.get('kit_directory', str(WORK)) if a.value == 'admin' else WORK)
        bash = shutil.which('bash', path=env['PATH']); need(bash, 'Bash is required from the trusted system PATH')
        os.execve(bash, ['bash', '--noprofile', '--rcfile', str(ROOT / 'session.sh'), '-i'], env)
    need(a.value in entries, 'Unknown step'); prepare(entries[a.value], d, os.environ.get('AGD_ROLE'))

if __name__ == '__main__':
    try: main()
    except (ValueError, OSError, KeyError, subprocess.CalledProcessError) as error:
        print('Stopped: ' + str(error) if isinstance(error, ValueError) else 'Stopped: invalid configuration or unavailable prerequisite', file=sys.stderr); sys.exit(1)
