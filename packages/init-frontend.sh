#!/bin/sh
# SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

echo "Checking if workspace is mounted but node_modules/ is not populated.."
if [ -d /workspaces/step/ ] && [ ! -d /workspaces/step/packages/node_modules ]
then
    echo "Populating node_modules/.."
    cp -r /app/node_modules /workspaces/step/packages/node_modules
else
    echo "Directory node_modules/ is already populated, skipping"
fi
yarn $1