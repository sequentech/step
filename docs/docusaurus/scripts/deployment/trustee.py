#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Small operator commands for the accepted offline trustee and its server state."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import stat
import subprocess
import uuid
import install
import oci

need = install.require
FIELDS = {'schema', 'delivery_directory', 'manifest_sha256', 'working_directory', 'keys_file', 'request_file', 'proof_file', 'tally_session_id'}


def validate(data):
    need(isinstance(data, dict) and set(data) == FIELDS, 'Trustee configuration has missing or unknown fields')
    need(type(data['schema']) is int and data['schema'] == 1, 'Unsupported trustee configuration')
    need(isinstance(data['manifest_sha256'], str) and re.fullmatch('[0-9a-f]{64}', data['manifest_sha256']), 'Use the independently accepted image-digests.txt SHA-256')
    for name in ('delivery_directory', 'working_directory'):
        value = data[name]
        need(isinstance(value, str) and Path(value).is_absolute() and not any(character in value for character in '\n\r\x00,:'), 'Use an absolute directory without control or mount-option characters')
    for name in ('keys_file', 'request_file', 'proof_file'):
        need(isinstance(data[name], str) and re.fullmatch('[A-Za-z0-9][A-Za-z0-9_.-]{0,159}', data[name]) and data[name] not in ('.', '..'), 'Use a simple file name inside this trustee working directory')
    need(len({data[name] for name in ('keys_file', 'request_file', 'proof_file')}) == 3, 'Keys, request and proof must use different names')
    tally = data['tally_session_id']
    if tally is not None:
        need(isinstance(tally, str) and str(uuid.UUID(tally)) == tally, 'Use the exact tally UUID from the portal, or null for the key ceremony')
    return data


def verify_offline_state(swaps, interfaces):
    need(len([line for line in swaps.splitlines() if line.strip()]) == 1, 'Disable all swap before handling trustee keys')
    need(all(name == 'lo' or state.strip() == 'down' for name, state in interfaces.items()), 'Disable every non-loopback network interface before handling trustee keys')


def offline():
    verify_offline_state(Path('/proc/swaps').read_text(), {path.name: (path / 'operstate').read_text() for path in Path('/sys/class/net').iterdir()})


def checked_directory(value, private=False):
    path = Path(value)
    need(path.is_dir() and not path.is_symlink() and path.resolve() == path, 'Directory is missing or contains a symbolic link')
    if private:
        need(path.stat().st_uid == os.geteuid() and not stat.S_IMODE(path.stat().st_mode) & 0o077, 'Trustee working directory must be owned by you with mode 700')
    return path


def encrypted_working_directory(data):
    path = checked_directory(data['working_directory'], private=True)
    source = install.run(['findmnt', '--noheadings', '--output', 'SOURCE', '--target', str(path)], 'Encrypted trustee medium check', timeout=5).decode().strip()
    need(re.fullmatch('/dev/mapper/[A-Za-z0-9_.+-]+', source) is not None, 'Trustee directory must be on its own unlocked encrypted medium')
    status = install.run(['cryptsetup', 'status', source.rsplit('/', 1)[1]], 'Encrypted trustee medium check', timeout=5).decode()
    need(re.search(r'^\s*type:\s*LUKS[12]\s*$', status, re.M), 'Trustee medium must use active LUKS encryption')
    return path


def accepted_digest(data, manifest):
    need(manifest.is_file() and not manifest.is_symlink(), 'Accepted image manifest is missing')
    raw = manifest.read_bytes()
    need(hashlib.sha256(raw).hexdigest() == data['manifest_sha256'], 'Image manifest differs from the independently accepted SHA-256')
    matches = re.findall(r'^braid (sha256:[0-9a-f]{64})$', raw.decode(), re.M)
    need(len(matches) == 1, 'Accepted image manifest must contain exactly one braid image')
    return matches[0]


def verify_local_image(digest):
    actual = install.run(['podman', 'image', 'inspect', '--format', '{{.Digest}}', 'localhost/braid@' + digest], 'Loaded trustee image verification', timeout=10).decode().strip()
    need(actual == digest, 'Loaded trustee image differs from the accepted delivery')


def prepare(data, config):
    delivery = checked_directory(data['delivery_directory'])
    source = delivery / 'image-digests.txt'
    digest = accepted_digest(data, source)
    oci.validate(delivery / 'images' / 'braid', digest)
    print('TRUSTEE IMAGE VERIFIED: all accepted manifests and layers match', flush=True)
    install.run(['skopeo', 'copy', 'oci:' + str(delivery / 'images' / 'braid') + ':v10.0.0', 'containers-storage:localhost/braid:v10.0.0'], 'Offline trustee image import', timeout=60)
    verify_local_image(digest)
    retained = config.parent / 'trustee-image-digests.txt'
    need(not retained.is_symlink(), 'Retained public manifest must not be a symbolic link')
    if retained.exists():
        need(retained.read_bytes() == source.read_bytes(), 'A different delivery was already prepared in this folder')
    else:
        with retained.open('xb') as stream: stream.write(source.read_bytes())
    print('TRUSTEE TOOL READY: accepted image loaded; safely eject the delivery medium', flush=True)


def check_files(data, operation):
    work = checked_directory(data['working_directory'], private=True)
    keys = work / data['keys_file']
    need(not keys.is_symlink(), 'Keys path must not be a symbolic link')
    if operation == 'generate':
        need(not keys.exists(), 'Keys already exist; use public-keys, never replace existing custody keys')
    else:
        need(keys.is_file() and keys.stat().st_uid == os.geteuid() and not stat.S_IMODE(keys.stat().st_mode) & 0o077, 'Existing keys must be owned by you with mode 600')
    if operation == 'attest':
        request = work / data['request_file']; proof = work / data['proof_file']
        need(request.is_file() and not request.is_symlink() and request.stat().st_size <= 1024 * 1024, 'Copy the public custody request into this working directory')
        need(not proof.exists() and not proof.is_symlink(), 'Proof already exists; choose a new proof_file in trustee.json')


def native_args(data, digest, operation):
    need(operation in ('generate', 'public-keys', 'attest'), 'Only offline key and custody operations are supported')
    args = ['podman', 'run', '--rm', '--pull=never', '--network=none', '--read-only', '--cap-drop=ALL', '--security-opt=no-new-privileges', '--pids-limit=64', '--memory=256m', '--memory-swap=256m', '--cpus=1', '--entrypoint', '/usr/bin/trustee', '-v', data['working_directory'] + ':/work:Z', '-w', '/work', 'localhost/braid@' + digest, operation, '--trustee-config', data['keys_file']]
    if operation == 'attest':
        args += ['--request', data['request_file'], '--output', data['proof_file']]
        if data['tally_session_id'] is not None: args += ['--tally-session-id', data['tally_session_id']]
    return args


def native(data, digest, operation):
    check_files(data, operation)
    output = install.run(native_args(data, digest, operation), 'Offline trustee ' + operation, timeout=30)
    if operation == 'generate':
        check_files(data, 'public-keys')
        output = install.run(native_args(data, digest, 'public-keys'), 'Public trustee key verification', timeout=30)
        print('TRUSTEE KEYS CREATED: ' + data['keys_file'])
    if operation in ('generate', 'public-keys'):
        public = json.loads(output)
        need(isinstance(public, dict) and set(public) == {'signing_public_key', 'share_encryption_public_key'} and all(isinstance(key, str) and key for key in public.values()), 'Native tool did not return the two public keys')
        print(json.dumps(public, indent=2))
    else:
        proof = Path(data['working_directory']) / data['proof_file']
        need(proof.is_file() and not proof.is_symlink() and proof.stat().st_size > 0, 'Native tool did not create the custody proof')
        print('CUSTODY PROOF CREATED: ' + data['proof_file'])


OTHER_MAINTENANCE = '''test ! -e /root/deploy/.backup-in-progress
test ! -e /var/lib/sequent-maintenance/state.json
'''
CLAIM = '''test ! -L /var/lib/sequent-maintenance
install -d -m 0700 /var/lib/sequent-maintenance
mkdir /var/lib/sequent-maintenance/claim
printf trustee > /var/lib/sequent-maintenance/claim/owner
'''
OWNED = '''test ! -L /var/lib/sequent-maintenance/claim
test ! -L /var/lib/sequent-maintenance/claim/owner
test "$(cat /var/lib/sequent-maintenance/claim/owner)" = trustee
'''
RELEASE = '''rm -f /var/lib/sequent-maintenance/claim/owner /var/lib/sequent-maintenance/claim/phase /var/lib/sequent-maintenance/claim/operation.lock /var/lib/sequent-maintenance/claim/id
rmdir /var/lib/sequent-maintenance/claim
'''
STOP = OTHER_MAINTENANCE + '''test ! -e /root/deploy/.trustee-backup-in-progress
test ! -e /root/deploy/.trustee-removal-in-progress
''' + CLAIM + '''(set -o noclobber; : > /root/deploy/.trustee-backup-in-progress)
printf backing-up > /var/lib/sequent-maintenance/claim/phase
exec 9>/var/lib/sequent-maintenance/claim/operation.lock
flock -n 9
test ! -L /root/deploy/trustee-backup-replicas.json
k3s kubectl -n step-infra get deploy trustee1 trustee2 -o json |
 jq '[.items[] | {name:.metadata.name,replicas:(.spec.replicas // 1)}]' > /root/deploy/trustee-backup-replicas.json
k3s kubectl -n step-infra scale deploy trustee1 trustee2 --replicas=0
k3s kubectl -n step-infra wait --for=delete pod -l 'app in (trustee1,trustee2)' --timeout=30s
count=$(k3s kubectl -n step-infra get pods -l 'app in (trustee1,trustee2)' -o json | jq '.items | length')
test "$count" = 0
printf 'TRUSTEES STOPPED: state backup may begin\n'
'''
ARCHIVE = OWNED + '''test -f /root/deploy/.trustee-backup-in-progress
test "$(cat /var/lib/sequent-maintenance/claim/phase)" = backing-up
exec 9>/var/lib/sequent-maintenance/claim/operation.lock
flock -n 9
count=$(k3s kubectl -n step-infra get pods -l 'app in (trustee1,trustee2)' -o json | jq '.items | length')
test "$count" = 0
for path in /var/lib/sequent-data/trustee1 /var/lib/sequent-data/trustee2; do test -d "$path"; test ! -L "$path"; done
tar -C /var/lib/sequent-data -cf - trustee1 trustee2
'''
RESUME = OTHER_MAINTENANCE + OWNED + '''test ! -e /root/deploy/.trustee-removal-in-progress
case "$(cat /var/lib/sequent-maintenance/claim/phase)" in backup-failed|backup-complete) ;; *) exit 1;; esac
exec 9>/var/lib/sequent-maintenance/claim/operation.lock
flock -n 9
test -f /root/deploy/.trustee-backup-in-progress
test ! -L /root/deploy/.trustee-backup-in-progress
test -f /root/deploy/trustee-backup-replicas.json
test ! -L /root/deploy/trustee-backup-replicas.json
jq -e 'length == 2 and ([.[].name] | sort) == ["trustee1","trustee2"] and all(.[]; (.replicas|type)=="number" and .replicas>=0 and .replicas<=100 and (.replicas|floor)==.replicas)' /root/deploy/trustee-backup-replicas.json >/dev/null
jq -r '.[] | [.name,(.replicas|tostring)] | @tsv' /root/deploy/trustee-backup-replicas.json |
 while read -r name replicas; do k3s kubectl -n step-infra scale "deployment/$name" --replicas="$replicas"; done
for name in trustee1 trustee2; do k3s kubectl -n step-infra rollout status "deployment/$name" --timeout=20s; done
rm /root/deploy/.trustee-backup-in-progress
printf 'TRUSTEE REPLICA COUNTS RESTORED\n'
''' + RELEASE
REMOVE = OTHER_MAINTENANCE + '''test ! -e /root/deploy/.trustee-backup-in-progress
test ! -e /root/deploy/.trustee-removal-in-progress
''' + CLAIM + '''(set -o noclobber; : > /root/deploy/.trustee-removal-in-progress)
for path in /var/lib/sequent-data/trustee1 /var/lib/sequent-data/trustee2; do test -d "$path"; test ! -L "$path"; done
k3s kubectl -n step-infra scale deployment/trustee1 deployment/trustee2 --replicas=0
k3s kubectl -n step-infra wait --for=delete pod -l 'app in (trustee1,trustee2)' --timeout=30s
count=$(k3s kubectl -n step-infra get pods -l 'app in (trustee1,trustee2)' -o json | jq '.items | length')
test "$count" = 0
k3s kubectl -n step-infra delete secret trustee1-keys trustee2-keys trustee1-auth trustee2-auth
find /var/lib/sequent-data/trustee1 /var/lib/sequent-data/trustee2 -mindepth 1 -delete
remaining=$(k3s kubectl -n step-infra get secret trustee1-keys trustee2-keys trustee1-auth trustee2-auth --ignore-not-found -o name)
test -z "$remaining"
count=$(find /var/lib/sequent-data/trustee1 /var/lib/sequent-data/trustee2 -mindepth 1 | wc -l)
test "$count" = 0
printf 'TRUSTEE REMOVAL COMPLETE: four Secrets absent; state directories empty\n'
rm /root/deploy/.trustee-removal-in-progress
''' + RELEASE


def backup_output():
    import maintenance
    return maintenance.backup_destination(prefix='trustee-state')


def backup_server(data):
    import maintenance
    output = backup_output()
    secret = maintenance.password(new=True)
    attempt = uuid.uuid4().hex
    owner_line = 'printf trustee > /var/lib/sequent-maintenance/claim/owner\n'
    stop = STOP.replace(owner_line, owner_line + 'printf ' + attempt + ' > /var/lib/sequent-maintenance/claim/id\n')
    owned_attempt = OWNED + 'test "$(cat /var/lib/sequent-maintenance/claim/id)" = ' + attempt + '\n'
    try:
        print(install.ssh(data, 'c1', stop, 'Stopping trustee instances', timeout=45).decode().strip(), flush=True)
        checksum = maintenance.encrypt_remote(data, owned_attempt + ARCHIVE, output, kind='trustee', passphrase=secret)
        install.ssh(data, 'c1', owned_attempt + 'printf backup-complete > /var/lib/sequent-maintenance/claim/phase\n', 'Record completed trustee backup', timeout=10)
    except BaseException:
        # An explicit resume must never race an active archive reader or a still
        # running shutdown after a disconnected SSH session.
        try:
            install.ssh(data, 'c1', owned_attempt + 'test -f /root/deploy/.trustee-backup-in-progress\nexec 9>/var/lib/sequent-maintenance/claim/operation.lock\nflock -n 9\nprintf backup-failed > /var/lib/sequent-maintenance/claim/phase\n', 'Record interrupted trustee backup', timeout=10)
        except BaseException:
            pass  # The durable backing-up state continues to refuse resumption.
        raise
    print(install.ssh(data, 'c1', RESUME, 'Restoring original trustee replica counts', timeout=50).decode().strip(), flush=True)
    print('TRUSTEE BACKUP COMPLETE: ' + str(output))
    print('Encrypted backup SHA-256: ' + checksum)


def confirm(phrase, explanation):
    with open('/dev/tty', 'r+') as tty:
        tty.write(explanation + '\nType ' + phrase + ': '); tty.flush()
        need(tty.readline().strip() == phrase, 'Operation cancelled; no automatic confirmation')


def server(config_path, operation):
    import maintenance
    need(os.geteuid() == 0, 'Run in a root administration-workstation terminal')
    os.umask(0o077)
    data = maintenance.load(config_path)
    if operation == 'backup-trustees': backup_server(data)
    elif operation == 'resume-trustees':
        confirm('RESUME UNCHANGED TRUSTEES', 'Use only after a failed backup, with the original state intact and board authorization.')
        print(install.ssh(data, 'c1', RESUME, 'Restoring original trustee replica counts', timeout=50).decode().strip())
    elif operation == 'remove-trustees':
        confirm('REMOVE TRUSTEE KEYS', 'Board-authorized retirement after retention and execution-data deletion. Server keys and state will be destroyed.')
        print(install.ssh(data, 'c1', REMOVE, 'Removing retired trustee keys and state', timeout=50).decode().strip())
    else: raise ValueError('Unknown trustee maintenance operation')


def main():
    parser = argparse.ArgumentParser(description='Use the accepted trustee tool offline; no server or browser private-key handling.')
    parser.add_argument('operation', choices=('prepare', 'generate', 'public-keys', 'attest'))
    parser.add_argument('configuration', type=Path)
    args = parser.parse_args()
    import runner
    need(not runner.manifest().get('test_only'), 'Use the accepted delivery tool folder, not a test-only bundle')
    need(os.geteuid() == 0, 'Run in the offline trustee workstation root terminal')
    os.umask(0o077)
    config = install.checked_path(args.configuration)
    data = validate(json.loads(config.read_text()))
    offline()
    if args.operation == 'prepare': prepare(data, config)
    else:
        encrypted_working_directory(data)
        digest = accepted_digest(data, config.parent / 'trustee-image-digests.txt')
        verify_local_image(digest)
        native(data, digest, args.operation)


if __name__ == '__main__':
    try: main()
    except (ValueError, OSError, subprocess.SubprocessError) as error:
        print('TRUSTEE OPERATION STOPPED: ' + str(error), file=__import__('sys').stderr)
        raise SystemExit(1)
