
#!/bin/sh
# SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

echo "Checking if workspace is mounted but node_modules/ is not populated..."
if [ -d /workspaces/step/ ] && [ ! -d /workspaces/step/packages/node_modules ]
then
    echo "Populating node_modules/.."
    cp -r /app/node_modules /workspaces/step/packages/node_modules
else
    echo "Directory node_modules/ is already populated, skipping"
fi

echo "Checking if ui-core is built..."
if [ ! -d /workspaces/step/packages/ui-core/dist ]
then
    echo "Populating ui-core/.."
    cp -r /app/ui-core/dist /workspaces/step/packages/ui-core/dist
else
    echo "Directory ui-core/dist is already built, skipping"
fi

echo "Checking if ui-essentials is built..."
if [ ! -d /workspaces/step/packages/ui-essentials/dist ]
then
    echo "Populating ui-essentials/.."
    cp -r /app/ui-essentials/dist /workspaces/step/packages/ui-essentials/dist
else
    echo "Directory ui-essentials/dist is already built, skipping"
fi
yarn $1
