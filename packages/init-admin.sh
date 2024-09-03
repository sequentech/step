#!/bin/sh
# SPDX-FileCopyrightText: 2023-2024 Sequent Tech <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

cp -r /node_modules /workspaces/step/packages/
cp -r /ui-core/node_modules /workspaces/step/packages/ui-core/
cp -r /ui-core/dist /workspaces/step/packages/ui-core/
cp -r /ui-essentials/node_modules /workspaces/step/packages/ui-essentials/
cp -r /ui-essentials/dist /workspaces/step/packages/ui-essentials/
cp -r /admin-portal/node_modules /workspaces/step/packages/admin-portal/