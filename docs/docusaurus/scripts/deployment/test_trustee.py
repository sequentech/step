# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Offline trustee boundaries and failure-safe maintenance orchestration."""
import hashlib
import json
from pathlib import Path
import tempfile
import subprocess
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


if __name__ == '__main__': unittest.main()
