#!/bin/sh

# SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

# sample commands
# ./immudbadmin -a immudb login immudb
# ./immudbadmin -a immudb database list
wget https://github.com/vchain-us/immudb/releases/download/v1.3.0/immuadmin-v1.3.0-linux-amd64
mv immuadmin-v1.3.0-linux-amd64 immudbadmin
chmod +x immudbadmin