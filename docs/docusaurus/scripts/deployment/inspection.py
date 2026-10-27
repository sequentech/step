#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Explicit operator inspections using the delivered configuration."""
import json
from pathlib import Path
import install
import site_checks
import verification


def main(configuration, check, realm=None):
    actions = {'kit': site_checks.kit, 'network': site_checks.network,
               'tls': site_checks.tls, 'client-network': site_checks.client_network,
               'platform': verification.platform, 'election': verification.event}
    install.require(check in actions, 'Choose kit, network, tls, client-network, platform or election')
    install.require((check == 'election') == (realm is not None),
                    'Supply a realm only for the election inspection')
    path = Path(configuration)
    install.require(path.is_file() and path.stat().st_size <= 1024 * 1024,
                    'Provide the inspection configuration file (at most 1 MiB)')
    try:
        data = json.loads(path.read_text())
    except (ValueError, UnicodeError):
        raise ValueError('Inspection configuration is not valid JSON') from None
    if check != 'client-network':
        install.validate(data)
    if check == 'election':
        actions[check](data, realm)
    else:
        actions[check](data)
