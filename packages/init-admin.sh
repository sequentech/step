#!/bin/sh
# SPDX-FileCopyrightText: 2023-2024 Sequent Tech <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

cp -rf /node_modules /workspaces/step/packages/
cp -rf /ui-core/node_modules /workspaces/step/packages/ui-core/
cp -rf /ui-core/dist /workspaces/step/packages/ui-core/
cp -rf /ui-essentials/node_modules /workspaces/step/packages/ui-essentials/
cp -rf /ui-essentials/dist /workspaces/step/packages/ui-essentials/
cp -rf /admin-portal/node_modules /workspaces/step/packages/admin-portal/
yarn start:admin-portal