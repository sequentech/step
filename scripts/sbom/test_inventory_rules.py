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

if __name__ == '__main__': unittest.main()
