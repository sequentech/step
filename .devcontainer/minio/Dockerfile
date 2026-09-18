# SPDX-FileCopyrightText: 2023-2024 Sequent Tech <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

FROM quay.io/minio/mc

WORKDIR /scripts
COPY . .

ENTRYPOINT ["/scripts/entrypoint.sh"]
