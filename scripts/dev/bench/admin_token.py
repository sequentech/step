# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
"""Prints an administrator access token of the local stack's tenant.

``python3 -m scripts.dev.bench.admin_token`` from the checkout root, in the
devenv shell: the password grant the backend journeys use
(``scripts.e2e.journeys.bootstrap.admin_token``), with the endpoints and
credentials of this checkout's environment. Load benchmarks run it as their
``--token-command`` whenever the token is about to expire.
"""

from __future__ import annotations

import os


def main() -> None:
    from scripts.dev.scenario import cli

    context = cli._context(cli.OutputFormat.TEXT)
    os.environ.update(context.environment)
    # The journeys' clients read their endpoints from the environment on import.
    from scripts.e2e.journeys import bootstrap, client

    print(bootstrap.admin_token(client.Keycloak()))


if __name__ == "__main__":
    main()
