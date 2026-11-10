# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Offline trustee boundaries and failure-safe maintenance orchestration."""
import hashlib
import json
import os
from pathlib import Path
import tempfile
import subprocess
import sys
import unittest
from unittest.mock import patch
import trustee


class TrusteeTests(unittest.TestCase):
    def config(self, folder):
        return {'schema': 1, 'delivery_directory': str(folder / 'delivery'), 'manifest_sha256': 'a' * 64, 'working_directory': str(folder / 'work'), 'keys_file': 'trustee.toml', 'request_file': 'custody-request.json', 'proof_file': 'custody-proof.json', 'tally_session_id': None}

    def test_paths_and_names_cannot_escape_current_trustee_medium(self):
        with tempfile.TemporaryDirectory() as name:
            config = self.config(Path(name)); trustee.validate(config)
            for field, value in [('keys_file', '../other/trustee.toml'), ('request_file', '/root/request.json'), ('proof_file', 'x:y'), ('tally_session_id', '$(id)'), ('working_directory', '/tmp/work,ro')]:
                with self.subTest(field=field), self.assertRaises(ValueError):
                    trustee.validate(dict(config, **{field: value}))

    def test_native_arguments_always_select_offline_binary_and_key_file(self):
        with tempfile.TemporaryDirectory() as name:
            config = self.config(Path(name))
            for operation in ('generate', 'public-keys', 'attest'):
                args = trustee.native_args(config, 'sha256:' + 'a' * 64, operation)
                self.assertIn('--network=none', args)
                self.assertIn('--pull=never', args)
                self.assertIn('--read-only', args)
                self.assertIn('--cap-drop=ALL', args)
                self.assertEqual(args[args.index('--entrypoint') + 1], '/usr/bin/trustee')
                self.assertEqual(args[args.index('--trustee-config') + 1], 'trustee.toml')
                self.assertNotIn('start', args)
            with self.assertRaises(ValueError):
                trustee.native_args(config, 'sha256:' + 'a' * 64, 'start')

    def test_retained_public_manifest_must_match_independently_accepted_hash(self):
        with tempfile.TemporaryDirectory() as name:
            folder = Path(name); config = self.config(folder)
            manifest = b'braid sha256:' + b'b' * 64 + b'\n'
            config['manifest_sha256'] = hashlib.sha256(manifest).hexdigest()
            record = folder / 'trustee-image-digests.txt'; record.write_bytes(manifest)
            self.assertEqual(trustee.accepted_digest(config, record), 'sha256:' + 'b' * 64)
            record.write_bytes(manifest.replace(b'b', b'c'))
            with self.assertRaises(ValueError): trustee.accepted_digest(config, record)

    def test_keys_symlinks_and_public_permissions_are_refused(self):
        with tempfile.TemporaryDirectory() as name:
            folder = Path(name); config = self.config(folder)
            work = folder / 'work'; work.mkdir(mode=0o700)
            key = work / 'trustee.toml'; key.write_text('never printed'); key.chmod(0o644)
            with self.assertRaises(ValueError): trustee.check_files(config, 'public-keys')
            key.chmod(0o600); trustee.check_files(config, 'public-keys')
            with self.assertRaises(ValueError): trustee.check_files(config, 'generate')
            key.unlink(); key.symlink_to(folder / 'missing')
            with self.assertRaises(ValueError): trustee.check_files(config, 'generate')

    def test_active_network_or_swap_refuses_keys(self):
        trustee.verify_offline_state('Filename Type Size Used Priority\n', {'lo': 'unknown', 'eth0': 'down'})
        for swaps, interfaces in [('Filename Type Size Used Priority\n/dev/dm-0 partition 4 0 1\n', {'lo': 'unknown'}), ('Filename Type Size Used Priority\n', {'eth0': 'up'}), ('Filename Type Size Used Priority\n', {'eth0': 'unknown'})]:
            with self.assertRaises(ValueError): trustee.verify_offline_state(swaps, interfaces)

    def test_failed_encryption_never_resumes_trustees(self):
        with patch('trustee.install.ssh', return_value=b'fixture') as ssh, patch('trustee.backup_output', return_value=Path('/private/backup.gpg')), patch('maintenance.password', return_value='private test only'), patch('maintenance.encrypt_remote', side_effect=ValueError('failed')):
            with self.assertRaises(ValueError): trustee.backup_server({})
            self.assertEqual(ssh.call_count, 2)
            self.assertIn('.trustee-backup-in-progress', ssh.call_args.args[2])
            self.assertIn('backup-failed', ssh.call_args.args[2])
            self.assertIn('/claim/id)', ssh.call_args.args[2])
            self.assertNotIn('scale', ssh.call_args.args[2])

    def test_removal_checks_stopped_pods_and_exact_paths_before_deleting(self):
        script = trustee.REMOVE
        self.assertLess(script.index('test "$count" = 0'), script.index('delete secret'))
        self.assertLess(script.index('test ! -L "$path"'), script.index('find /var/lib/sequent-data/trustee1'))
        self.assertIn('trustee1-keys trustee2-keys trustee1-auth trustee2-auth', script)
        self.assertNotIn('rm -rf', script)

    def test_shared_claim_prevents_system_and_trustee_overlap(self):
        for script in (trustee.STOP, trustee.REMOVE):
            self.assertLess(script.index('mkdir /var/lib/sequent-maintenance/claim'), script.index('scale deploy'))
            self.assertIn('test ! -e /var/lib/sequent-maintenance/state.json', script)
        self.assertIn('test "$(cat /var/lib/sequent-maintenance/claim/owner)" = trustee', trustee.RESUME)
        self.assertIn('test ! -e /root/deploy/.trustee-removal-in-progress', trustee.RESUME)
        self.assertIn('backup-failed|backup-complete', trustee.RESUME)
        self.assertIn('flock -n 9', trustee.RESUME)

    def test_delivered_executable_has_its_dependencies_and_refuses_test_bundle_execution(self):
        import build
        with tempfile.TemporaryDirectory() as folder:
            output = Path(folder) / 'verified folder' / 'deployment'
            manifest = build.build(Path(__file__).resolve().parents[4], output, '0' * 40, True)
            for name in ('trustee', 'trustee.py', 'trustee.example.json', 'runner.py', 'install.py', 'oci.py', 'maintenance.py', 'archive_check.py', 'step_names.py'):
                self.assertIn(name, manifest['files'])
                self.assertEqual(hashlib.sha256((output / name).read_bytes()).hexdigest(), manifest['files'][name])
            executable = output / 'trustee'
            help_result = subprocess.run([str(executable), '--help'], capture_output=True, text=True)
            self.assertEqual(help_result.returncode, 0, help_result.stderr)
            self.assertIn('public-keys', help_result.stdout)
            result = subprocess.run([str(executable), 'generate', 'trustee.json'], cwd=output, capture_output=True, text=True)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn('test-only bundle', result.stderr)
            with (output / 'trustee.py').open('a') as source: source.write('\n# changed after delivery\n')
            checked = subprocess.run([str(output / 'deploy'), 'check'], capture_output=True, text=True)
            self.assertNotEqual(checked.returncode, 0)

    def test_actual_resume_script_refuses_active_copy_and_preserves_saved_counts(self):
        import fcntl
        for phase, locked in (('backing-up', False), ('backup-failed', True), ('backup-failed', False)):
            with self.subTest(phase=phase, locked=locked), tempfile.TemporaryDirectory() as folder:
                root = Path(folder); deploy = root / 'deploy'; deploy.mkdir()
                state = root / 'maintenance'; claim = state / 'claim'; claim.mkdir(parents=True)
                (claim / 'owner').write_text('trustee'); (claim / 'phase').write_text(phase)
                (deploy / '.trustee-backup-in-progress').touch()
                (deploy / 'trustee-backup-replicas.json').write_text(json.dumps([{'name': 'trustee1', 'replicas': 0}, {'name': 'trustee2', 'replicas': 2}]))
                with (claim / 'operation.lock').open('w') as held:
                    if locked: fcntl.flock(held, fcntl.LOCK_EX | fcntl.LOCK_NB)
                    script = trustee.RESUME.replace('/root/deploy', str(deploy)).replace('/var/lib/sequent-maintenance', str(state))
                    result = subprocess.run(['bash', '-c', 'set -Eeuo pipefail\nk3s(){ printf "MOCK CALL: %s\\n" "$*"; }\n' + script], capture_output=True, text=True)
                if phase == 'backing-up' or locked:
                    self.assertNotEqual(result.returncode, 0)
                    self.assertNotIn('MOCK CALL', result.stdout)
                    self.assertTrue(claim.exists())
                else:
                    self.assertEqual(result.returncode, 0, result.stderr)
                    self.assertIn('deployment/trustee1 --replicas=0', result.stdout)
                    self.assertIn('deployment/trustee2 --replicas=2', result.stdout)
                    self.assertFalse(claim.exists())


# A stand-in for `k3s` on C1: it records each call and keeps the Secrets, the
# destroy pods and the datastore answers in a state directory.
MOCK_K3S = r"""#!/usr/bin/env python3
import json, os, sys
from pathlib import Path
state = Path(os.environ['MOCK_STATE'])
config = json.loads((state / 'config.json').read_text())
args = sys.argv[1:]
with (state / 'calls').open('a') as calls: calls.write(' '.join(args) + '\n')
pods = state / 'pods'; secrets = state / 'secrets'
def fail(message):
    sys.stderr.write(message + '\n'); sys.exit(1)
if args[0] == 'ctr':
    if not config['destroy_available']: fail("error: unrecognized subcommand 'destroy'")
    sys.exit(0)
rest = args[1:]
if rest[:2] == ['-n', 'step-infra']: rest = rest[2:]
verb, what = rest[0], rest[1]
if verb == 'exec':
    print(config['open'])
elif verb == 'get' and what == 'secret':
    names = [a for a in rest[2:] if not a.startswith('-') and a != 'name']
    present = [n for n in names if (secrets / n).exists()]
    if '--ignore-not-found' not in rest and present != names: fail('Error from server (NotFound): secrets not found')
    for n in present: print('secret/' + n)
elif verb == 'get' and what == 'deployment':
    print(json.dumps({'items': [{'spec': {'template': {'spec': {'containers': [{'image': image}]}}}} for image in config['images']]}))
elif verb == 'get' and what == 'pods':
    print(json.dumps({'items': []}))
elif verb == 'get' and what == 'pod':
    phase = pods / (rest[2] + '.phase')
    if not phase.exists(): fail('Error from server (NotFound): pods not found')
    sys.stdout.write(phase.read_text())
elif verb == 'get' and what == '--raw':
    if 'resourceVersion=' not in rest[2]:
        print(json.dumps({'kind': 'EventList', 'metadata': {'resourceVersion': '4242'}, 'items': []}))
        sys.exit(0)
    seen_file = state / 'exact-queries'
    seen = int(seen_file.read_text()) if seen_file.exists() else 0
    seen_file.write_text(str(seen + 1))
    if config['raw_error']: fail(config['raw_error'])
    if config['compacted_after'] is not None and seen >= config['compacted_after']:
        fail('Error from server (Expired): The resourceVersion for the provided list is too old.')
    print(json.dumps({'kind': 'EventList', 'metadata': {'resourceVersion': '4242'}, 'items': []}))
elif verb == 'create':
    pod = json.loads(sys.stdin.read())
    name = pod['metadata']['name']; trustee = name[:-len('-destroy')]
    if (pods / (name + '.json')).exists(): fail('Error from server (AlreadyExists): pods "' + name + '" already exists')
    (pods / (name + '.json')).write_text(json.dumps(pod)); (state / ('created-' + name + '.json')).write_text(json.dumps(pod))
    outcome = config['destroy_outcomes'].get(trustee, 'Succeeded')
    if outcome == 'Succeeded':
        store = Path(config['data']) / trustee / 'message_store'
        lines = ['destroyed the keys file /run/destroy/trustee.toml (131 bytes) of', 'public_key: synthetic-' + trustee, 'share_encryption_public_key: synthetic-' + trustee]
        if store.exists():
            for item in sorted(store.iterdir()):
                lines.append('destroyed the session store /opt/trustee/message_store/' + item.name + ' (' + str(item.stat().st_size) + ' bytes)')
                item.unlink()
            store.rmdir(); lines.append('removed the session store directory /opt/trustee/message_store')
        (pods / (name + '.log')).write_text('\n'.join(lines) + '\n')
    elif outcome == 'Failed':
        (pods / (name + '.log')).write_text('Error: nothing was removed\n\nCaused by:\n    synthetic refusal\n')
    (pods / (name + '.phase')).write_text(outcome)
elif verb == 'logs':
    log = pods / (what.split('/', 1)[1] + '.log')
    if not log.exists(): fail('container "destroy" in pod is waiting to start')
    sys.stdout.write(log.read_text())
elif verb == 'delete' and what == 'pod':
    if not (pods / (rest[2] + '.json')).exists():
        if '--ignore-not-found' not in rest: fail('Error from server (NotFound): pods "' + rest[2] + '" not found')
    else: print('pod "' + rest[2] + '" deleted')
    for suffix in ('.json', '.phase', '.log'): (pods / (rest[2] + suffix)).unlink(missing_ok=True)
elif verb == 'delete' and what == 'secret':
    names = [a for a in rest[2:] if not a.startswith('-')]
    if '--ignore-not-found' not in rest and not all((secrets / n).exists() for n in names): fail('Error from server (NotFound): secrets not found')
    for n in names:
        if (secrets / n).exists(): (secrets / n).unlink(); print('secret "' + n + '" deleted')
elif verb == 'scale':
    for target in rest[1:]:
        if target.startswith('deployment/'): print('deployment.apps/' + target.split('/', 1)[1] + ' scaled')
elif verb == 'wait':
    print('pod/trustee1-5d8f7c9b4-x2k8p condition met')
else:
    fail('unexpected call')
"""
IMAGE = 'sequentech.local/braid@sha256:' + 'b' * 64
SECRETS = ('trustee1-keys', 'trustee2-keys', 'trustee1-auth', 'trustee2-auth')


class TrusteeKeyDestructionTests(unittest.TestCase):
    def fixture(self, folder, destroyed=False, **settings):
        root = Path(folder)
        deploy = root / 'deploy'; deploy.mkdir()
        (deploy / 'image-digests.txt').write_text('b4 sha256:' + 'a' * 64 + '\nbraid ' + IMAGE.split('@')[1] + '\n')
        state = root / 'state'; (state / 'secrets').mkdir(parents=True); (state / 'pods').mkdir()
        for name in ('trustee1', 'trustee2'):
            (root / 'data' / name).mkdir(parents=True)
            if not destroyed:
                (state / 'secrets' / (name + '-keys')).touch(); (state / 'secrets' / (name + '-auth')).touch()
                store = root / 'data' / name / 'message_store'; store.mkdir()
                (store / 'board.sqlite').write_bytes(b'synthetic store'); (store / 'board.sqlite-wal').write_bytes(b'synthetic')
        config = {'open': '0', 'images': [IMAGE, IMAGE], 'destroy_available': True, 'destroy_outcomes': {}, 'data': str(root / 'data'), 'compacted_after': 0, 'raw_error': None}
        config.update(settings)
        (state / 'config.json').write_text(json.dumps(config))
        (root / 'bin').mkdir(); (root / 'bin' / 'k3s').write_text(MOCK_K3S); (root / 'bin' / 'k3s').chmod(0o755)
        return root

    def command(self, root, script):
        script = script.replace('/root/deploy', str(root / 'deploy')).replace('/var/lib/sequent-maintenance', str(root / 'maintenance')).replace('/var/lib/sequent-data', str(root / 'data'))
        return ['env', 'PATH=' + str(root / 'bin') + os.pathsep + os.environ['PATH'], 'MOCK_STATE=' + str(root / 'state'), 'bash', '-c', 'set -Eeuo pipefail\nset +x\numask 077\n' + script]

    def run_remote(self, root, script):
        return subprocess.run(self.command(root, script), capture_output=True, text=True, timeout=60)

    def calls(self, root):
        path = root / 'state' / 'calls'
        return path.read_text().splitlines() if path.exists() else []

    def assert_unchanged(self, root):
        calls = '\n'.join(self.calls(root))
        for word in ('scale', 'create', 'delete'): self.assertNotIn(word, calls)
        self.assertFalse((root / 'maintenance' / 'claim').exists())
        self.assertFalse((root / 'deploy' / '.trustee-removal-in-progress').exists())
        self.assertEqual(sorted(path.name for path in (root / 'state' / 'secrets').iterdir()), sorted(SECRETS))
        for name in ('trustee1', 'trustee2'): self.assertTrue((root / 'data' / name / 'message_store' / 'board.sqlite').exists())

    def refusal(self, root, script=None):
        with self.assertRaises(ValueError) as caught:
            trustee.install.run(self.command(root, script or trustee.REMOVE), 'Trustee key destruction')
        return str(caught.exception)

    def test_destruction_refuses_while_an_election_event_can_still_use_the_keys(self):
        with tempfile.TemporaryDirectory() as folder:
            root = self.fixture(folder, open='1')
            self.assertEqual(self.refusal(root), 'Trustee key destruction failed: An election event can still use the trustee keys: every election event must be in post-processing or archived; nothing was changed; stop and inspect this stage privately; see If something fails in the Deployment Guide')
            self.assert_unchanged(root)
            self.assertNotIn('ctr', '\n'.join(self.calls(root)))
        # Finished means what the platform allows to be exported and deleted.
        self.assertIn("WHERE l.phase IS NULL OR NOT (l.phase = 'post_processing' OR (l.phase = 'preparation' AND e.is_archived IS TRUE))", trustee.OPEN_EVENTS_SQL)
        self.assertIn('-c "' + trustee.OPEN_EVENTS_SQL + '"', trustee.REMOVE)

    def test_an_image_without_destroy_or_other_than_the_accepted_one_is_refused_before_any_change(self):
        with tempfile.TemporaryDirectory() as folder:
            root = self.fixture(folder, destroy_available=False)
            self.assertEqual(self.refusal(root), 'Trustee key destruction failed: The accepted trustee image does not provide trustee destroy; nothing was changed; stop and inspect this stage privately; see If something fails in the Deployment Guide')
            self.assert_unchanged(root)
        other = IMAGE[:-64] + 'c' * 64
        for images in ([IMAGE, other], ['sequentech.local/trustee:latest'] * 2, [other] * 2, [IMAGE.split('@')[1]] * 2):
            with self.subTest(images=images), tempfile.TemporaryDirectory() as folder:
                root = self.fixture(folder, images=images)
                result = self.run_remote(root, trustee.REMOVE)
                self.assertNotEqual(result.returncode, 0)
                self.assert_unchanged(root)
                self.assertNotIn('ctr', '\n'.join(self.calls(root)))

    def test_destruction_runs_trustee_destroy_for_each_trustee_before_deleting_the_secrets(self):
        with tempfile.TemporaryDirectory() as folder:
            root = self.fixture(folder)
            result = self.run_remote(root, trustee.REMOVE)
            self.assertEqual(result.returncode, 0, result.stderr)
            lines = result.stdout.splitlines()
            self.assertEqual(lines[0], 'trustee1: destroyed the keys file /run/destroy/trustee.toml (131 bytes) of')
            self.assertIn('trustee1: destroyed the session store /opt/trustee/message_store/board.sqlite (15 bytes)', lines)
            self.assertIn('trustee2: removed the session store directory /opt/trustee/message_store', lines)
            self.assertEqual(lines[-1], 'TRUSTEE REMOVAL COMPLETE: four Secrets absent; state directories empty')
            self.assertEqual((root / 'deploy' / 'trustee-destroy.log').read_text().splitlines(), lines[:-1])
            for word in ('scaled', 'condition met', 'deleted'): self.assertNotIn(word, result.stdout)
            calls = self.calls(root)
            order = [next(i for i, call in enumerate(calls) if call.startswith(prefix)) for prefix in (
                'ctr -n k8s.io run --rm ' + IMAGE + ' agd-trustee-destroy-check-', 'kubectl -n step-infra scale deployment/trustee1 deployment/trustee2 --replicas=0',
                'kubectl -n step-infra wait --for=delete pod', 'kubectl -n step-infra delete pod trustee1-destroy --ignore-not-found', 'kubectl create -f -',
                'kubectl -n step-infra delete pod trustee1-destroy --timeout=60s', 'kubectl -n step-infra delete pod trustee2-destroy --ignore-not-found',
                'kubectl -n step-infra delete pod trustee2-destroy --timeout=60s', 'kubectl -n step-infra delete secret ' + ' '.join(SECRETS))]
            self.assertEqual(order, sorted(order))
            self.assertIn('--timeout=' + str(trustee.TRUSTEE_STOP_SECONDS) + 's', calls[order[2]])
            self.assertEqual(sum(call == 'kubectl create -f -' for call in calls), 2)
            self.assertEqual(list((root / 'state' / 'secrets').iterdir()), [])
            for name in ('trustee1', 'trustee2'): self.assertEqual(list((root / 'data' / name).iterdir()), [])
            self.assertFalse((root / 'maintenance' / 'claim').exists())
            self.assertFalse((root / 'deploy' / '.trustee-removal-in-progress').exists())

    def test_the_destroy_pod_uses_the_trustee_volume_and_an_in_memory_copy_of_its_keys(self):
        with tempfile.TemporaryDirectory() as folder:
            root = self.fixture(folder)
            self.assertEqual(self.run_remote(root, trustee.REMOVE).returncode, 0)
            created = {name: json.loads((root / 'state' / ('created-' + name + '-destroy.json')).read_text()) for name in ('trustee1', 'trustee2')}
        for name, pod in created.items():
            spec = pod['spec']; container = spec['containers'][0]
            self.assertEqual((pod['metadata']['namespace'], spec['restartPolicy'], spec['activeDeadlineSeconds'], spec['automountServiceAccountToken']), ('step-infra', 'Never', 120, False))
            self.assertEqual(spec['nodeSelector'], {'node-role.kubernetes.io/control-plane': 'true'})
            self.assertEqual((container['image'], container['imagePullPolicy'], container['workingDir']), (IMAGE, 'Never', '/opt/trustee'))
            self.assertEqual(container['command'][-1], 'umask 077; cat /var/run/trustee-keys/trustee.toml > /run/destroy/trustee.toml && exec trustee destroy --trustee-config /run/destroy/trustee.toml')
            volumes = {volume['name']: volume for volume in spec['volumes']}
            mounts = {mount['name']: mount for mount in container['volumeMounts']}
            self.assertEqual(volumes['data']['persistentVolumeClaim'], {'claimName': name + '-pvc'})
            self.assertEqual(mounts['data']['mountPath'], '/opt/trustee')
            self.assertEqual(volumes['keys']['secret'], {'secretName': name + '-keys', 'defaultMode': 0o400})
            self.assertEqual((mounts['keys']['mountPath'], mounts['keys']['readOnly']), ('/var/run/trustee-keys', True))
            self.assertEqual(volumes['copy']['emptyDir'], {'medium': 'Memory', 'sizeLimit': '1Mi'})
            self.assertEqual(mounts['copy']['mountPath'], '/run/destroy')

    def test_a_destroy_run_that_does_not_succeed_keeps_the_secrets_and_the_trustees_stopped(self):
        self.assertIn('deadline=$((SECONDS+150))', trustee.REMOVE)
        for outcome in ('Failed', 'Pending'):
            with self.subTest(outcome=outcome), tempfile.TemporaryDirectory() as folder:
                root = self.fixture(folder, destroy_outcomes={'trustee2': outcome})
                script = trustee.REMOVE.replace('deadline=$((SECONDS+150))', 'deadline=$((SECONDS+0))')
                self.assertEqual(self.refusal(root, script), 'Trustee key destruction failed: A trustee destroy run did not complete; the trustee Secrets are kept and both trustees stay stopped; its output is in /root/deploy/trustee-destroy.log on C1; stop and inspect this stage privately; see If something fails in the Deployment Guide')
                calls = '\n'.join(self.calls(root))
                self.assertIn('kubectl -n step-infra delete pod trustee2-destroy', calls)
                self.assertNotIn('delete secret', calls)
                self.assertEqual(list((root / 'state' / 'pods').iterdir()), [])
                self.assertEqual(sorted(path.name for path in (root / 'state' / 'secrets').iterdir()), sorted(SECRETS))
                self.assertTrue((root / 'deploy' / '.trustee-removal-in-progress').exists())
                self.assertTrue((root / 'maintenance' / 'claim').exists())
                log = (root / 'deploy' / 'trustee-destroy.log').read_text()
                self.assertIn('trustee1: removed the session store directory', log)
                self.assertIn('trustee2: Error: nothing was removed' if outcome == 'Failed' else 'trustee2: container "destroy" in pod is waiting to start', log)
                # Run again, it continues: trustee1 has only its keys file left.
                config = json.loads((root / 'state' / 'config.json').read_text()); config['destroy_outcomes'] = {}
                (root / 'state' / 'config.json').write_text(json.dumps(config))
                result = self.run_remote(root, trustee.REMOVE)
                self.assertEqual(result.returncode, 0, result.stderr)
                lines = result.stdout.splitlines()
                self.assertEqual([line for line in lines if line.startswith('trustee1: ')], ['trustee1: destroyed the keys file /run/destroy/trustee.toml (131 bytes) of', 'trustee1: public_key: synthetic-trustee1', 'trustee1: share_encryption_public_key: synthetic-trustee1'])
                self.assertIn('trustee2: destroyed the session store /opt/trustee/message_store/board.sqlite (15 bytes)', lines)
                self.assertEqual(lines[-1], 'TRUSTEE REMOVAL COMPLETE: four Secrets absent; state directories empty')
                self.assertEqual(list((root / 'state' / 'secrets').iterdir()), [])
                for name in ('trustee1', 'trustee2'): self.assertEqual(list((root / 'data' / name).iterdir()), [])
                self.assertFalse((root / 'maintenance' / 'claim').exists())
                self.assertFalse((root / 'deploy' / '.trustee-removal-in-progress').exists())
                self.assertTrue((root / 'deploy' / 'trustee-destroy.log').read_text().startswith(log))

    def interrupted(self, folder, **settings):
        root = self.fixture(folder, **settings)
        claim = root / 'maintenance' / 'claim'; claim.mkdir(parents=True); (claim / 'owner').write_text('trustee')
        (root / 'deploy' / '.trustee-removal-in-progress').touch()
        return root

    def test_a_run_that_stopped_after_the_secrets_were_deleted_is_completed_by_the_next(self):
        with tempfile.TemporaryDirectory() as folder:
            root = self.interrupted(folder, destroyed=True)
            (root / 'data' / 'trustee2' / 'left').write_text('synthetic')
            (root / 'state' / 'pods' / 'trustee1-destroy.json').write_text('{}')
            result = self.run_remote(root, trustee.REMOVE)
            self.assertEqual((result.returncode, result.stdout), (0, 'TRUSTEE REMOVAL COMPLETE: four Secrets absent; state directories empty\n'), result.stderr)
            calls = '\n'.join(self.calls(root))
            self.assertNotIn('create', calls)
            self.assertEqual(list((root / 'data' / 'trustee2').iterdir()), [])
            self.assertFalse((root / 'maintenance' / 'claim').exists())
            self.assertFalse((root / 'deploy' / '.trustee-removal-in-progress').exists())

    def test_a_leftover_destroy_pod_is_removed_before_the_next_run_creates_its_own(self):
        with tempfile.TemporaryDirectory() as folder:
            root = self.interrupted(folder)
            (root / 'state' / 'pods' / 'trustee1-destroy.json').write_text('{}')
            result = self.run_remote(root, trustee.REMOVE)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(result.stdout.splitlines()[-1], 'TRUSTEE REMOVAL COMPLETE: four Secrets absent; state directories empty')
            self.assertEqual(list((root / 'state' / 'pods').iterdir()), [])

    def test_continuing_needs_the_trustee_claim_and_no_other_run(self):
        import fcntl
        with tempfile.TemporaryDirectory() as folder:
            root = self.interrupted(folder)
            with open(root / 'maintenance' / 'claim' / 'operation.lock', 'w') as lock:
                fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
                result = self.run_remote(root, trustee.REMOVE)
            self.assertNotEqual(result.returncode, 0)
            calls = '\n'.join(self.calls(root))
            for word in ('scale', 'create', 'delete'): self.assertNotIn(word, calls)
            self.assertTrue((root / 'deploy' / '.trustee-removal-in-progress').exists())
        for owner in ('maintenance', None):
            with self.subTest(owner=owner), tempfile.TemporaryDirectory() as folder:
                root = self.interrupted(folder)
                if owner: (root / 'maintenance' / 'claim' / 'owner').write_text(owner)
                else: (root / 'maintenance' / 'claim' / 'owner').unlink(); (root / 'maintenance' / 'claim').rmdir()
                result = self.run_remote(root, trustee.REMOVE)
                self.assertNotEqual(result.returncode, 0)
                calls = '\n'.join(self.calls(root))
                for word in ('exec', 'scale', 'create', 'delete'): self.assertNotIn(word, calls)

    def test_a_repeated_run_after_destruction_only_reports_it(self):
        with tempfile.TemporaryDirectory() as folder:
            root = self.fixture(folder, destroyed=True, open='1')
            result = self.run_remote(root, trustee.REMOVE)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(result.stdout, 'TRUSTEE KEYS ALREADY DESTROYED: four Secrets absent; state directories empty\n')
            calls = '\n'.join(self.calls(root))
            for word in ('exec', 'ctr', 'scale', 'create', 'delete'): self.assertNotIn(word, calls)
            self.assertFalse((root / 'maintenance').exists())
        with tempfile.TemporaryDirectory() as folder:
            root = self.fixture(folder, destroyed=True)
            (root / 'data' / 'trustee2' / 'left').write_text('synthetic')
            result = self.run_remote(root, trustee.REMOVE)
            self.assertNotEqual(result.returncode, 0)
            self.assertNotIn('ALREADY DESTROYED', result.stdout)
            self.assertNotIn('scale', '\n'.join(self.calls(root)))
            self.assertFalse((root / 'maintenance').exists())

    def test_a_removal_after_a_stopped_import_of_new_keys_destroys_what_it_installed(self):
        with tempfile.TemporaryDirectory() as folder:
            root = self.fixture(folder)
            for name in ('trustee2-keys', 'trustee2-auth'): (root / 'state' / 'secrets' / name).unlink()
            for item in (root / 'data' / 'trustee2' / 'message_store').iterdir(): item.unlink()
            (root / 'data' / 'trustee2' / 'message_store').rmdir()
            # Without the stopped import, a missing keys Secret is refused.
            result = self.run_remote(root, trustee.REMOVE)
            self.assertNotEqual(result.returncode, 0)
            self.assertNotIn('scale', '\n'.join(self.calls(root)))
            (root / 'deploy' / '.trustee-keys-import-started').touch(); (root / 'deploy' / '.trustee-keys-destroyed').touch()
            result = self.run_remote(root, trustee.REMOVE)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(result.stdout.splitlines()[-1], 'TRUSTEE REMOVAL COMPLETE: four Secrets absent; state directories empty')
            self.assertNotIn('trustee2: ', result.stdout)
            self.assertIn('trustee1: destroyed the session store /opt/trustee/message_store/board.sqlite (15 bytes)', result.stdout.splitlines())
            self.assertEqual(sum(call == 'kubectl create -f -' for call in self.calls(root)), 1)
            self.assertEqual(list((root / 'state' / 'secrets').iterdir()), [])
            for marker in ('.trustee-keys-import-started', '.trustee-keys-destroyed', '.trustee-removal-in-progress'):
                self.assertFalse((root / 'deploy' / marker).exists(), marker)
            self.assertFalse((root / 'maintenance' / 'claim').exists())

    def test_a_removal_with_nothing_left_ends_a_stopped_import(self):
        with tempfile.TemporaryDirectory() as folder:
            root = self.fixture(folder, destroyed=True)
            (root / 'deploy' / '.trustee-keys-import-started').touch()
            result = self.run_remote(root, trustee.REMOVE)
            self.assertEqual((result.returncode, result.stdout), (0, 'TRUSTEE KEYS ALREADY DESTROYED: four Secrets absent; state directories empty\n'), result.stderr)
            self.assertFalse((root / 'deploy' / '.trustee-keys-import-started').exists())

    def test_a_new_removal_withdraws_the_record_of_the_earlier_one_until_its_own_check(self):
        with tempfile.TemporaryDirectory() as folder:
            root = self.fixture(folder)
            (root / 'deploy' / '.trustee-keys-destroyed').touch()
            result = self.run_remote(root, trustee.REMOVE.replace('k3s kubectl -n step-infra delete secret', 'test ! -e /root/deploy/.trustee-keys-destroyed\nk3s kubectl -n step-infra delete secret'))
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertFalse((root / 'deploy' / '.trustee-keys-destroyed').exists())

    def test_the_datastore_check_waits_until_the_revision_after_the_deletion_is_compacted(self):
        quick = trustee.COMPACTION.replace('sleep 20', 'sleep 0')
        expired = 'deadline=$((SECONDS+' + str(trustee.COMPACTION_SECONDS) + '))'
        self.assertIn(expired, quick)
        self.assertIn("'/api/v1/namespaces/step-infra/events?fieldSelector=metadata.name%3Dagd-no-such-event'", quick)
        cases = [({'compacted_after': 2}, quick, 'DATASTORE COMPACTED: no earlier revision of the trustee Secrets remains in the cluster datastore\n', 3),
                 ({'compacted_after': None}, quick.replace(expired, 'deadline=$((SECONDS+0))'), 'DATASTORE COMPACTION PENDING\n', 1),
                 ({'raw_error': 'Unable to connect to the server: x509: certificate has expired or is not yet valid'}, quick.replace(expired, 'deadline=$((SECONDS+0))'), 'DATASTORE COMPACTION PENDING\n', 1)]
        for settings, script, output, queries in cases:
            with self.subTest(settings=settings), tempfile.TemporaryDirectory() as folder:
                root = self.fixture(folder, destroyed=True, **settings)
                result = self.run_remote(root, script)
                self.assertEqual((result.returncode, result.stdout), (0, output), result.stderr)
                exact = [call for call in self.calls(root) if 'resourceVersion=' in call]
                self.assertEqual(len(exact), queries)
                self.assertTrue(all(call.endswith('&resourceVersion=4242&resourceVersionMatch=Exact') for call in exact))
                self.assertEqual((root / 'deploy' / '.trustee-keys-destroyed').exists(), output.startswith('DATASTORE COMPACTED'))

    def test_removal_confirms_destroys_and_then_checks_the_datastore(self):
        complete = b'trustee1: destroyed the keys file\nTRUSTEE REMOVAL COMPLETE: four Secrets absent; state directories empty\n'
        compacted = b'DATASTORE COMPACTED: no earlier revision of the trustee Secrets remains in the cluster datastore\n'
        with patch('trustee.confirm') as confirm, patch('trustee.install.ssh', side_effect=[complete, compacted]) as ssh, patch('builtins.print') as printed:
            trustee.remove_server({})
        self.assertEqual(confirm.call_args.args[0], 'REMOVE TRUSTEE KEYS')
        self.assertIn('post-processing or archived', confirm.call_args.args[1])
        self.assertEqual([call.args[1:4] for call in ssh.call_args_list], [('c1', trustee.REMOVE, 'Trustee key destruction'), ('c1', trustee.COMPACTION, 'Cluster datastore compaction check')])
        self.assertGreater(ssh.call_args_list[1].kwargs['timeout'], trustee.COMPACTION_SECONDS)
        self.assertIn(compacted.decode().strip(), str(printed.call_args_list[-2]))
        self.assertEqual(printed.call_args_list[-1].args[0], 'Before the next election event: each trustee generates a new key pair offline and an administrator registers both new public keys in the customer administration portal; then run ./deploy commission site.json with both trustees and their key media present.')
        with patch('trustee.confirm'), patch('trustee.install.ssh', side_effect=[complete, b'DATASTORE COMPACTION PENDING\n']), patch('builtins.print'):
            with self.assertRaises(ValueError) as caught: trustee.remove_server({})
        self.assertEqual(str(caught.exception), 'The cluster datastore still holds earlier revisions of the deleted trustee Secrets; run ./deploy remove-trustees site.json again later')
        with patch('trustee.confirm'), patch('trustee.install.ssh', side_effect=ValueError('refused')) as ssh, patch('builtins.print'):
            with self.assertRaises(ValueError): trustee.remove_server({})
        self.assertEqual(ssh.call_count, 1)
        with patch('trustee.confirm', side_effect=ValueError('Operation cancelled; no automatic confirmation')), patch('trustee.install.ssh') as ssh:
            with self.assertRaises(ValueError): trustee.remove_server({})
        ssh.assert_not_called()

    def test_the_trustee_custody_operations_are_commands_of_the_deploy_helper(self):
        import runner
        for operation in ('backup-trustees', 'resume-trustees', 'remove-trustees'):
            self.assertIn(operation, runner.OPERATIONS)
            with patch('sys.argv', ['deploy', operation, 'site.json']), patch('runner.manifest', return_value={}), patch('trustee.server') as server:
                runner.main()
            server.assert_called_once_with('site.json', operation)

    def test_the_backup_waits_for_a_stopping_trustee(self):
        self.assertIn("--timeout=" + str(trustee.TRUSTEE_STOP_SECONDS) + "s\n", trustee.STOP)
        self.assertGreaterEqual(trustee.TRUSTEE_STOP_SECONDS, 150)
        with patch('trustee.install.ssh', side_effect=ValueError('stopped')) as ssh, patch('trustee.backup_output', return_value=Path('/private/backup.gpg')), patch('maintenance.password', return_value='private test only'):
            with self.assertRaises(ValueError): trustee.backup_server({})
        self.assertGreater(ssh.call_args_list[0].kwargs['timeout'], trustee.TRUSTEE_STOP_SECONDS)



# A stand-in for podman on the offline workstation: it records each call and,
# like `trustee destroy`, removes only a file that reads as a keys file.
MOCK_PODMAN = r"""#!/usr/bin/env python3
import os, sys
from pathlib import Path
args = sys.argv[1:]
with open(os.environ['MOCK_CALLS'], 'a') as calls: calls.write(' '.join(args) + '\n')
if args[-2:] == ['destroy', '--help']: sys.exit(1 if os.environ.get('MOCK_NO_DESTROY') else 0)
volume = args[args.index('-v') + 1].split(':')
config = args[args.index('--trustee-config') + 1]
assert volume[1:] == ['/keys', 'Z'] and config.startswith('/keys/'), args
path = Path(volume[0]) / config[len('/keys/'):]
if path.is_symlink() or not path.is_file() or path.read_bytes() != b'synthetic trustee keys\n':
    print('Error: nothing was removed', file=sys.stderr); sys.exit(1)
size = path.stat().st_size
if not os.environ.get('MOCK_KEEP'): path.write_bytes(bytes(size)); path.unlink()
print('destroyed the keys file ' + config + ' (' + str(size) + ' bytes) of')
print('public_key: synthetic-signing-key')
print('share_encryption_public_key: synthetic-share-key')
"""
DIGEST = 'sha256:' + 'b' * 64


class OfflineKeyDestructionTests(unittest.TestCase):
    def setUp(self):
        folder = tempfile.TemporaryDirectory(); self.addCleanup(folder.cleanup)
        self.root = Path(folder.name)
        self.work = self.root / 'work'; self.work.mkdir(mode=0o700)
        self.keys = self.work / 'trustee.toml'; self.keys.write_bytes(b'synthetic trustee keys\n'); self.keys.chmod(0o600)
        self.config = TrusteeTests.config(None, self.root)
        (self.root / 'bin').mkdir(); (self.root / 'bin' / 'podman').write_text(MOCK_PODMAN); (self.root / 'bin' / 'podman').chmod(0o755)
        self.calls = self.root / 'calls'
        environment = patch.dict(os.environ, {'PATH': str(self.root / 'bin') + os.pathsep + os.environ['PATH'], 'MOCK_CALLS': str(self.calls)})
        environment.start(); self.addCleanup(environment.stop)
        medium = patch('trustee.encrypted_working_directory', return_value=self.work)
        self.medium = medium.start(); self.addCleanup(medium.stop)

    def podman_calls(self):
        return self.calls.read_text().splitlines() if self.calls.exists() else []

    def destroy(self, copy=None, confirm=None):
        with patch('trustee.confirm', side_effect=confirm) as confirmed, patch('builtins.print') as printed:
            trustee.destroy(self.config, DIGEST, copy)
        return confirmed, [call.args[0] for call in printed.call_args_list]

    def refused(self, copy=None, confirm=None):
        with patch('trustee.confirm', side_effect=confirm) as confirmed, self.assertRaises(ValueError) as caught:
            trustee.destroy(self.config, DIGEST, copy)
        return confirmed, str(caught.exception)

    def test_the_keys_file_is_destroyed_offline_after_the_typed_confirmation(self):
        confirmed, printed = self.destroy()
        self.assertFalse(os.path.lexists(self.keys))
        self.medium.assert_called_once_with(self.config)
        self.assertEqual(confirmed.call_args.args[0], 'DESTROY KEYS FILE')
        self.assertIn(str(self.keys) + ' is overwritten with zeros', confirmed.call_args.args[1])
        self.assertIn(trustee.DESTROY_LIMITS, confirmed.call_args.args[1])
        self.assertEqual(printed, ['destroyed the keys file ' + str(self.keys) + ' (23 bytes) of\npublic_key: synthetic-signing-key\nshare_encryption_public_key: synthetic-share-key', 'KEYS FILE DESTROYED: ' + str(self.keys), trustee.DESTROY_LIMITS])
        available, destroyed = self.podman_calls()
        self.assertTrue(available.endswith('localhost/braid@' + DIGEST + ' destroy --help'))
        self.assertNotIn(' -v ', available)
        for option in ('--network=none', '--read-only', '--cap-drop=ALL', '--pull=never', '--entrypoint /usr/bin/trustee'):
            self.assertIn(option, available); self.assertIn(option, destroyed)
        self.assertTrue(destroyed.endswith('-v ' + str(self.work) + ':/keys:Z -w / localhost/braid@' + DIGEST + ' destroy --trustee-config /keys/trustee.toml'))

    def test_a_copy_on_another_medium_is_destroyed_by_its_path(self):
        backup = self.root / 'backup'; backup.mkdir()
        copy = backup / 'trustee-copy.toml'; copy.write_bytes(b'synthetic trustee keys\n')
        confirmed, printed = self.destroy(copy)
        self.assertFalse(os.path.lexists(copy))
        self.assertTrue(self.keys.exists())
        self.medium.assert_not_called()
        self.assertEqual(printed[1], 'KEYS FILE DESTROYED: ' + str(copy))
        self.assertIn('-v ' + str(backup) + ':/keys:Z', self.podman_calls()[1])

    def test_a_path_that_is_not_a_regular_file_is_refused_before_any_change(self):
        target = self.root / 'target.toml'; target.write_bytes(b'synthetic trustee keys\n')
        cases = {'missing': lambda path: None, 'directory': lambda path: path.mkdir(), 'symbolic link': lambda path: path.symlink_to(target), 'pipe': lambda path: os.mkfifo(path)}
        for case, make in cases.items():
            with self.subTest(case=case):
                self.keys.unlink(missing_ok=True)
                make(self.keys)
                confirmed, message = self.refused()
                self.assertTrue(message.endswith('; nothing was changed'), message)
                self.assertIn(str(self.keys), message)
                confirmed.assert_not_called()
                self.assertEqual(self.podman_calls(), [])
                if self.keys.is_dir() and not self.keys.is_symlink(): self.keys.rmdir()
        self.assertEqual(target.read_bytes(), b'synthetic trustee keys\n')

    def test_the_path_of_a_copy_must_be_absolute_and_plain(self):
        (self.root / 'linked').symlink_to(self.work)
        plain = 'Use the absolute path of the copy, without control or mount-option characters'
        cases = [(Path('work/trustee.toml'), plain), (self.root / 'linked' / 'trustee.toml', 'Directory is missing or contains a symbolic link'), (self.root / 'work' / '..' / 'work' / 'trustee.toml', 'Directory is missing or contains a symbolic link')]
        for name in ('a:b', 'a,b', 'a\nb'):
            (self.root / name).mkdir(); (self.root / name / 'trustee.toml').write_bytes(b'synthetic trustee keys\n')
            cases.append((self.root / name / 'trustee.toml', plain))
        previous = os.getcwd(); os.chdir(self.root); self.addCleanup(os.chdir, previous)
        for copy, expected in cases:
            with self.subTest(copy=str(copy)):
                confirmed, message = self.refused(copy)
                self.assertEqual(message, expected)
                confirmed.assert_not_called()
        self.assertEqual(self.podman_calls(), [])
        self.assertTrue(self.keys.exists())

    def test_a_copy_may_keep_the_name_a_file_manager_gave_it(self):
        backup = self.root / 'backup'; backup.mkdir()
        copy = backup / 'trustee (copy 1).toml'; copy.write_bytes(b'synthetic trustee keys\n')
        self.destroy(copy)
        self.assertFalse(os.path.lexists(copy))
        self.assertTrue(self.podman_calls()[1].endswith("--trustee-config /keys/trustee (copy 1).toml"))

    def test_every_other_operation_still_needs_the_encrypted_medium(self):
        (self.root / 'trustee.json').write_text(json.dumps(self.config))
        for argv, medium in ((['generate'], True), (['public-keys'], True), (['attest'], True), (['destroy'], False), (['destroy', '/media/backup/trustee.toml'], False)):
            with self.subTest(argv=argv), patch('sys.argv', ['trustee', argv[0], str(self.root / 'trustee.json')] + argv[1:]), patch('runner.manifest', return_value={}), patch('trustee.os.geteuid', return_value=0), patch('trustee.install.checked_path', side_effect=Path), patch('trustee.offline'), patch('trustee.os.umask'), patch('trustee.accepted_digest', return_value=DIGEST), patch('trustee.verify_local_image') as image, patch('trustee.native') as native, patch('trustee.destroy') as destroyed:
                self.medium.reset_mock()
                trustee.main()
                self.assertEqual(self.medium.called, medium)
                image.assert_called_once_with(DIGEST)
                if argv[0] == 'destroy': destroyed.assert_called_once_with(self.config, DIGEST, Path(argv[1]) if argv[1:] else None); native.assert_not_called()
                else: native.assert_called_once_with(self.config, DIGEST, argv[0]); destroyed.assert_not_called()

    def test_an_image_without_destroy_is_refused_before_the_confirmation(self):
        with patch.dict(os.environ, {'MOCK_NO_DESTROY': '1'}):
            confirmed, message = self.refused()
        self.assertEqual(message, 'The accepted trustee image does not provide trustee destroy; nothing was changed')
        confirmed.assert_not_called()
        self.assertEqual(len(self.podman_calls()), 1)
        self.assertTrue(self.keys.exists())

    def test_a_cancelled_confirmation_changes_nothing(self):
        confirmed, message = self.refused(confirm=ValueError('Operation cancelled; no automatic confirmation'))
        self.assertEqual(message, 'Operation cancelled; no automatic confirmation')
        self.assertEqual(len(self.podman_calls()), 1)
        self.assertEqual(self.keys.read_bytes(), b'synthetic trustee keys\n')

    def test_a_file_the_native_command_refuses_is_kept_and_reported(self):
        self.keys.write_bytes(b'not a keys file\n')
        confirmed, message = self.refused()
        self.assertEqual(message, 'trustee destroy did not complete; ' + str(self.keys) + ' still exists. It must be a trustee keys file you can write')
        self.assertEqual(self.keys.read_bytes(), b'not a keys file\n')

    def test_a_run_that_leaves_the_file_is_not_reported_as_a_destruction(self):
        with patch.dict(os.environ, {'MOCK_KEEP': '1'}), patch('builtins.print') as printed:
            confirmed, message = self.refused()
        self.assertEqual(message, 'trustee destroy did not remove ' + str(self.keys))
        printed.assert_not_called()

    def test_only_destroy_takes_the_path_of_a_copy(self):
        with patch('sys.argv', ['trustee', 'generate', 'trustee.json', '/media/backup/trustee.toml']), self.assertRaises(ValueError) as caught:
            trustee.main()
        self.assertEqual(str(caught.exception), 'Only destroy takes the path of a copy')
        result = subprocess.run([sys.executable, str(Path(trustee.__file__)), '--help'], capture_output=True, text=True, timeout=30)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn('destroy', result.stdout)
        self.assertIn('destroy only: the path of a copy of the keys file on another medium', ' '.join(result.stdout.split()))


if __name__ == '__main__': unittest.main()
