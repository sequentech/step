# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

"""`python -m monitoring_renderer`: serve on RENDERER_HOST:RENDERER_PORT
(0.0.0.0:8080)."""

from __future__ import annotations

import logging
import os

import uvicorn

from .app import Settings, create_app
from .netguard import block_outbound


def main() -> None:
    logging.basicConfig(level=logging.INFO, format="%(asctime)s %(levelname)s %(name)s %(message)s")
    settings = Settings.from_env()  # refuses to start without RENDERER_TOKEN
    block_outbound()
    uvicorn.run(
        create_app(settings),
        host=os.environ.get("RENDERER_HOST", "0.0.0.0"),
        port=int(os.environ.get("RENDERER_PORT", "8080")),
        workers=1,
        access_log=False,
        server_header=False,
        proxy_headers=False,
        timeout_keep_alive=30,
    )


if __name__ == "__main__":
    main()
