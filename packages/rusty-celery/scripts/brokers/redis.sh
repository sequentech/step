#!/bin/sh

# SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

set -e

docker run -p 127.0.0.1:6379:6379 --rm redis
