#!/bin/bash
# SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

set -ex -o pipefail

echo "initializing localstack"

awslocal ses verify-domain-identity --domain sequent.vote
awslocal ses verify-domain-identity --domain comelec.gov.ph
