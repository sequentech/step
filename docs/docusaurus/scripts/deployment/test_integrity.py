# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
import json
import hashlib
import os
import subprocess
import tempfile
from pathlib import Path
import unittest
from unittest.mock import patch
import integrity

class IntegrityTests(unittest.TestCase):
    def test_each_server_node_verifies_every_layer_and_every_component(self):
        expected={part:'sha256:'+'a'*64 for part in integrity.install.PARTS}
        script=integrity.node_script(expected)
        self.assertIn('.layers[]',script)
        self.assertIn('sha256sum',script)
        for part in expected:self.assertIn(part+' sha256:',script)
        self.assertNotIn('images pull',script)
    def test_an_unexpected_or_missing_component_is_refused(self):
        for values in ({}, {'harvest':'sha256:'+'a'*64}, {part:'bad' for part in integrity.install.PARTS}):
            with self.assertRaises(ValueError):integrity.node_script(values)
    def test_operator_check_requires_all_seven_accepted_layouts_before_remote_reads(self):
        data={'toe':{'format':'tar'}}
        with patch('integrity.install.verify_inputs') as verify:
            with self.assertRaises(ValueError):integrity.check(data)
            verify.assert_not_called()

    def test_real_node_graph_check_detects_a_corrupted_layer(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory);blobs=root/'blobs';blobs.mkdir()
            def blob(value):
                digest=hashlib.sha256(value).hexdigest();(blobs/digest).write_bytes(value)
                return {'digest':'sha256:'+digest,'size':len(value)}
            config=blob(b'{"architecture":"amd64"}')
            layer=blob(b'actual layer contents')
            manifest=blob(json.dumps({'schemaVersion':2,'config':config,'layers':[layer]}).encode())
            fake=root/'k3s'
            fake.write_text('#!/usr/bin/env bash\nset -eu\ndigest="${@: -1}"\ncat "'+str(blobs)+'/${digest#sha256:}"\n')
            fake.chmod(0o700)
            environment={**os.environ,'PATH':str(root)+':'+os.environ['PATH']}
            script=integrity.node_script({part:manifest['digest'] for part in integrity.install.PARTS})
            run=lambda:subprocess.run(['bash','-c',script],env=environment,capture_output=True,text=True,timeout=5)
            self.assertEqual(run().returncode,0)
            (blobs/layer['digest'].split(':')[1]).write_bytes(b'tampered layer')
            self.assertNotEqual(run().returncode,0)
