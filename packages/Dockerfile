# SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

FROM node:20-alpine

# Install Yarn
RUN apk add --no-cache yarn

# Install dependencies
WORKDIR /usr/src/app/

EXPOSE 3000

ENTRYPOINT ["./init.sh", "start"]