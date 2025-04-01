# SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

FROM localstack/localstack:4.3.0

RUN mkdir -p /etc/localstack/init/ready.d/
COPY setup.sh /etc/localstack/init/ready.d/setup.sh
