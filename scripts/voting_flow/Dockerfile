# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

# PostgreSQL 18 server binaries and psycopg for the disposable-cluster suites.
FROM postgres:18-bookworm
RUN apt-get update \
    && apt-get install --yes --no-install-recommends python3 python3-psycopg \
    && rm -rf /var/lib/apt/lists/*
USER postgres
