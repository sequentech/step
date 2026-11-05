# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only
"""The enrollment approval matrix the janitor adds to a delivery
(``templates/<client>/approvalMatrix.json``): the fields compared with the
registry, the ordered rules and the last rule. The import saves it as the
election event's version 1.

Kept apart from run.py, which runs a delivery when it is imported, so the
pieces can be tested on their own.
"""

import json

DEFAULT_PATH = "templates/COMELEC/approvalMatrix.json"


def load(path):
    with open(path, "r") as file:
        return json.load(file)


def add_to_bundle(final_json, matrix):
    """The election event bundle with the approval matrix."""
    final_json["approval_matrix"] = matrix
    return final_json
