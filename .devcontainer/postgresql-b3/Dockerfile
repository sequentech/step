# SPDX-FileCopyrightText: 2023-2024 Sequent Tech <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

FROM postgres:16-bullseye

RUN apt-get update \
    && apt-cache showpkg postgresql-$PG_MAJOR-pgaudit \
    && apt-get install -y --no-install-recommends \
    postgresql-$PG_MAJOR-pgaudit postgresql-contrib postgresql-$PG_MAJOR-wal2json \
    && rm -rf /var/lib/apt/lists/*

RUN mkdir -p /docker-entrypoint-initdb.d /etc/postgresql
COPY init.sh /docker-entrypoint-initdb.d/
COPY init-db.sql /docker-entrypoint-initdb.d/
COPY postgresql.conf /etc/postgresql/postgresql.conf
