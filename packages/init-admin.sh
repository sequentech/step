#!/bin/sh
# SPDX-FileCopyrightText: 2023-2024 Sequent Tech <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

rm -rf /workspaces/step/packages/node_modules
cp -r /node_modules /workspaces/step/packages/node_modules
rm -rf /workspaces/step/packages/ui-core/node_modules
cp -r /ui-core/node_modules /workspaces/step/packages/ui-core/node_modules
rm -rf /workspaces/step/packages/ui-core/dist
cp -r /ui-core/dist /workspaces/step/packages/ui-core/dist
rm -rf /workspaces/step/packages/ui-essentials/node_modules
cp -r /ui-essentials/node_modules /workspaces/step/packages/ui-essentials/node_modules
rm -rf /workspaces/step/packages/ui-essentials/dist
cp -r /ui-essentials/dist /workspaces/step/packages/ui-essentials/dist
rm -rf /workspaces/step/packages/admin-portal/node_modules
cp -r /admin-portal/node_modules /workspaces/step/packages/admin-portal/node_modules
# yarn start:admin-portal