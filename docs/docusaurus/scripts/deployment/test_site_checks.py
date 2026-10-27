# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import site_checks

class SiteChecksTest(unittest.TestCase):
    def public(self):
        return {'schema':1,'nodes':{'c1':'192.0.2.11','c2':'192.0.2.12'},'ingress_ports':[80,443,10080,10443],'voting_host':'vote.example.org'}

    def test_public_report_cannot_accept_private_configuration(self):
        value=self.public();value['ssh_identity']='/root/private-key'
        with self.assertRaisesRegex(ValueError,'fields'):site_checks.validate_client(value)

    def test_client_report_rejects_invalid_or_injected_ports(self):
        for ports in [[],[0],[65536],[True],['22;id'],[22,22]]:
            value=self.public();value['ingress_ports']=ports
            with self.assertRaises(ValueError):site_checks.validate_client(value)

    def test_client_report_rejects_same_node_or_shell_host(self):
        for name in ['192.0.2.11','192.0.2.12;id']:
            value=self.public();value['nodes']['c2']=name
            with self.assertRaises(ValueError):site_checks.validate_client(value)

    def test_closed_everywhere_is_not_firewall_success(self):
        with patch('site_checks.port_open',return_value=False):
            with self.assertRaisesRegex(ValueError,'positive'):site_checks.client_network(self.public())

    def test_forbidden_ssh_open_is_refused(self):
        with patch('site_checks.port_open',side_effect=lambda host,port: (host,port) in [('192.0.2.11',443),('192.0.2.11',22)]):
            with self.assertRaisesRegex(ValueError,'forbidden'):site_checks.client_network(self.public())

    def test_public_report_is_never_silently_overwritten(self):
        with tempfile.TemporaryDirectory() as temp:
            path=Path(temp)/'client-network.json';path.write_text('unrelated contents')
            with self.assertRaisesRegex(ValueError,'exists'):site_checks.save_client(self.public(),path)
            self.assertEqual(path.read_text(),'unrelated contents')

    def test_public_report_reuses_only_identical_regular_file(self):
        with tempfile.TemporaryDirectory() as temp:
            path=Path(temp)/'client-network.json';site_checks.save_client(self.public(),path);site_checks.save_client(self.public(),path)
            other=Path(temp)/'alias';other.symlink_to(path)
            with self.assertRaises(ValueError):site_checks.save_client(self.public(),other)
            self.assertEqual(json.loads(path.read_text()),self.public())

    def test_timeout_is_not_proof_of_tls_refusal(self):
        with self.assertRaisesRegex(ValueError,'timed out'):site_checks.check_rejected(124,'')

    def test_negotiated_tls_cannot_be_reported_as_refused(self):
        with self.assertRaisesRegex(ValueError,'accepted'):site_checks.check_rejected(0,'New, TLSv1.3, Cipher is TLS_AES_128_GCM_SHA256')
        with self.assertRaisesRegex(ValueError,'accepted'):site_checks.check_rejected(1,'New, TLSv1.3, Cipher is TLS_AES_128_GCM_SHA256')

    def test_positive_tls_requires_verified_curve_and_key(self):
        valid='New, TLSv1.3, Cipher is TLS_AES_256_GCM_SHA384\nPeer Temp Key: ECDH, secp384r1, 384 bits\nServer public key is 384 bit\nVerify return code: 0 (ok)\n'
        site_checks.check_allowed(0,valid)
        for bad in [valid.replace('secp384r1','X25519'),valid.replace('0 (ok)','20 (untrusted)'),valid.replace('384 bit','256 bit')]:
            with self.assertRaises(ValueError):site_checks.check_allowed(0,bad)
        with self.assertRaises(ValueError):site_checks.check_allowed(1,valid)

if __name__=='__main__':unittest.main()
