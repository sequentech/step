# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
import unittest
from inventory_rules import image_reference

class InventoryRulesTest(unittest.TestCase):
    def test_non_docker_from_is_not_image(self):
        for filename, line in [('query.sh', 'FROM (VALUES'), ('installer.nix', "from prepare.sh's output"), ('load.sh','FROM ballot_box_voter')]:
            self.assertIsNone(image_reference(filename, line))
    def test_only_concrete_image_references(self):
        self.assertEqual(image_reference('compose.yaml','  image: postgres:16'), 'postgres:16')
        self.assertEqual(image_reference('Dockerfile.prod','FROM --platform=linux/amd64 debian:bookworm AS runtime'), 'debian:bookworm')
        for line in ['image: {{ .Values.image }}', 'image: ${IMAGE}', 'image: foo:${VERSION}', 'image: ""']:
            self.assertIsNone(image_reference('deployment.yaml',line))


class EvidenceRulesTest(unittest.TestCase):
    def test_image_version_ignores_registry_port_and_digest(self):
        from inventory_rules import image_version
        for ref, expected in [('registry:5000/team/image', 'unversioned'), ('image@sha256:abcdef', 'unversioned'), ('registry:5000/team/image:1.2@sha256:abcdef', '1.2'), ('image:latest', 'latest')]:
            self.assertEqual(image_version(ref), expected)

    def test_license_evidence_does_not_close_independent_review(self):
        from inventory_rules import evidence_review_status
        self.assertEqual(evidence_review_status('Needs confirmation', 'OPEN'), 'Needs confirmation')
        self.assertEqual(evidence_review_status('Needs confirmation', 'SEQUENT'), 'Needs confirmation')
        self.assertEqual(evidence_review_status('Source evidence collected', 'UNRESOLVED'), 'Needs confirmation')
        self.assertEqual(evidence_review_status('Source evidence collected', 'OPEN'), 'Source license evidence recorded')
        self.assertEqual(evidence_review_status('Approved', 'OPEN'), 'Approved')

    def test_gitlink_reads_current_commit_and_rejects_regular_file(self):
        from inventory_rules import gitlink
        from unittest.mock import patch
        with patch('inventory_rules.subprocess.check_output', return_value='160000 commit ' + 'a'*40 + '\tbeyond\n') as run:
            self.assertEqual(gitlink('/repo', 'beyond'), 'a'*40)
            run.assert_called_once_with(['git', '-C', '/repo', 'ls-tree', 'HEAD', '--', 'beyond'], text=True)
        for output in ['', '100644 blob ' + 'b'*40 + '\tbeyond\n']:
            with patch('inventory_rules.subprocess.check_output', return_value=output):
                self.assertIsNone(gitlink('/repo', 'beyond'))

class PlatformInventoryTest(unittest.TestCase):
    def test_repository_csv_exists_even_when_empty(self):
        from inventory_rules import write_platform_dependencies
        from tempfile import TemporaryDirectory
        from pathlib import Path
        import csv
        with TemporaryDirectory() as directory:
            path = Path(directory) / 'platform.csv'
            records = [dict(repository='step', component='api', purl='pkg:maven/api', reason='provided', evidence='pom.xml')]
            write_platform_dependencies(path, records, 'beyond')
            with path.open() as stream:
                reader = csv.DictReader(stream)
                self.assertIn('component', reader.fieldnames)
                self.assertEqual(list(reader), [])
            write_platform_dependencies(path, records, 'step')
            with path.open() as stream:
                self.assertEqual(list(csv.DictReader(stream)), records)


class PinnedSourceTest(unittest.TestCase):
    def test_reads_pinned_content_not_current_checkout(self):
        from inventory_rules import pinned_source
        from tempfile import TemporaryDirectory
        from pathlib import Path
        import subprocess
        with TemporaryDirectory() as directory:
            root = Path(directory)
            def git(*args):
                return subprocess.check_output(['git', '-C', str(root), *args], text=True).strip()
            git('init', '-q')
            git('config', 'user.name', 'Test')
            git('config', 'user.email', 'test@example.invalid')
            (root/'notice').write_text('original owner')
            git('add', 'notice'); git('commit', '-qm', 'first')
            pin = git('rev-parse', 'HEAD')
            (root/'notice').write_text('different owner')
            git('commit', '-qam', 'second')
            with pinned_source(root, pin) as source:
                self.assertEqual((source/'notice').read_text(), 'original owner')
            self.assertFalse(source.exists())
            self.assertEqual((root/'notice').read_text(), 'different owner')
            with self.assertRaises(subprocess.CalledProcessError):
                with pinned_source(root, '0'*40): pass


class SourceIdentityTest(unittest.TestCase):
    def test_git_source_requires_repository_and_exact_revision(self):
        from inventory_rules import step_source_matches
        pin = 'a' * 40
        row = {'Type': 'rust-crate', '_source': 'git+https://github.com/other/project#'+pin}
        self.assertFalse(step_source_matches(row, pin, True))
        row['_source'] = 'git+https://github.com/sequentech/step?branch=ovcs#'+'b'*40
        self.assertFalse(step_source_matches(row, pin, True))
        row['_source'] = 'git+https://github.com/sequentech/step?branch=ovcs#'+pin
        self.assertTrue(step_source_matches(row, pin, False))
        row['_source'] = ''
        self.assertFalse(step_source_matches(row, pin, False))
        self.assertTrue(step_source_matches(row, pin, True))

    def test_unknown_license_stays_unresolved(self):
        from inventory_rules import module_rights_profile
        self.assertEqual(module_rights_profile('NOASSERTION', 'Sequent-authored module'), 'UNRESOLVED')
        self.assertEqual(module_rights_profile('NOASSERTION', 'Imported third-party source'), 'UNRESOLVED')
        self.assertEqual(module_rights_profile('AGPL-3.0-only', 'Sequent-authored module'), 'SEQUENT')

class LocalIdentityTest(unittest.TestCase):
    def test_only_reachable_local_manifests_supply_identity(self):
        from inventory_rules import local_step_identities
        from tempfile import TemporaryDirectory
        from pathlib import Path
        with TemporaryDirectory() as directory:
            step = Path(directory)/'step'; beyond = Path(directory)/'beyond'
            def write(root, name, text):
                path = root/name; path.parent.mkdir(parents=True, exist_ok=True); path.write_text(text)
            write(beyond, 'packages/Cargo.toml', '[workspace]\n[workspace.dependencies]\ncore={path="../../packages/core"}\n')
            write(beyond, 'packages/app/Cargo.toml', '[package]\nname="app"\n[dependencies]\ncore={workspace=true}\nforeign={git="https://example.invalid/repo"}\n')
            write(step, 'packages/core/Cargo.toml', '[package]\nname="core"\n[dependencies]\nhelper={path="../helper"}\n')
            write(step, 'packages/helper/Cargo.toml', '[package]\nname="helper"\n')
            write(step, 'packages/foreign/Cargo.toml', '[package]\nname="foreign"\n')
            prefix = '<project xmlns="http://maven.apache.org/POM/4.0.0">'
            write(beyond, 'packages/java/pom.xml', prefix+'<parent><relativePath>../../../packages/java/pom.xml</relativePath></parent></project>')
            write(step, 'packages/java/pom.xml', prefix+'<groupId>sequent</groupId><artifactId>root</artifactId><version>1</version><modules><module>api</module></modules></project>')
            write(step, 'packages/java/api/pom.xml', prefix+'<parent><groupId>sequent</groupId><version>1</version></parent><artifactId>api</artifactId></project>')
            identities = local_step_identities(step, beyond)
            self.assertIn('packages/core/Cargo.toml', identities)
            self.assertIn('packages/helper/Cargo.toml', identities)
            self.assertNotIn('packages/foreign/Cargo.toml', identities)
            self.assertEqual(identities['packages/java/api/pom.xml'], 'pkg:maven/sequent/api@1')

if __name__ == '__main__': unittest.main()
