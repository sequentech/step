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

if __name__ == '__main__': unittest.main()
