# SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

# Pinned by digest: the tag moves with every upstream rebuild.
FROM postgres:18-bookworm@sha256:3725f4e2499eef5134592b3b4ab79a543ed7f8e533b05b5b637af926630f6650

RUN apt-get update \
    && apt-cache showpkg postgresql-$PG_MAJOR-pgaudit \
    && apt-get install -y --no-install-recommends \
    postgresql-$PG_MAJOR-pgaudit postgresql-contrib postgresql-$PG_MAJOR-wal2json \
    && rm -rf /var/lib/apt/lists/* /var/log/apt /var/log/dpkg.log /var/log/alternatives.log

RUN mkdir -p /docker-entrypoint-initdb.d /etc/postgresql
COPY init.sh /docker-entrypoint-initdb.d/
COPY postgresql.conf /etc/postgresql/postgresql.conf
