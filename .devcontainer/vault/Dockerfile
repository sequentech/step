# SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

FROM hashicorp/vault:latest

RUN mkdir -p /opt/vault
COPY config.hcl /opt/vault
COPY volume.tar.gz /opt/vault
COPY entrypoint.sh /opt/vault
ENTRYPOINT ["/opt/vault/entrypoint.sh"]