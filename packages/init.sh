#!/bin/sh
# SPDX-FileCopyrightText: 2023-2024 Sequent Tech <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

yarn --pure-lockfile --non-interactive
yarn build:ui-core
yarn build:ui-essentials
yarn $1