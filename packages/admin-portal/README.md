<!--
SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->
# Admin-portal

Admin portal for Sequent Voting Platform (2nd-gen).

## Launch development environment from `packages/` directory

Enter into the `packages/` directory and then launch the development environment
with:

```bash
cd packages/
yarn start:admin
```

## Update graphql JSON schema

The file `packages/admin-portal/graphql.schema.json` contains the GraphQL/Hasura
schema. If the schema changes you might need to update this file. In order to do
so,
[follow this guide](https://hasura.io/docs/latest/schema/common-patterns/export-graphql-schema/) 
to export the json schema from Hasura, specifically you'll need to run something
like:

```bash
cd packages/admin-portal/
gq http://graphql-engine:8080/v1/graphql \
    -H "X-Hasura-Admin-Secret: admin" \
    --introspect  \
    --format json \
    > graphql.schema.json
```

Afterwards, you need to regenerate the typescript auto-generated types using
`graphql-codegen` with:

```bash
yarn generate
```
## Compile the Ui Core library

This package uses the common UI librarry [ui-core] as a github submodule.
You need to compile ui-essentials:

```bash
cd packages/ui-core
yarn
yarn build
```

## Compile the Ui library

This package uses the common UI librarry [ui-essentials] as a github submodule.
You need to compile ui-essentials:

```bash
cd packages/ui-essentials
yarn
yarn build
```

### Use sequent-core

This package uses [sequent-core] as a npm package. You need to compile it in
another place and then copy it to `rust/sequent-core-0.1.0.tgz`. Note that if
its version is changed you may need to update its hash in
`packages/admin-portal/yarn.lock` (use `sha1sum rust/sequent-core-0.1.0.tgz` to
get the hash, or `shasum` instead of `sha1sum` if you're in Mac OS X.


### Run Nightwatch end to end tests

Chrome workspaces e2e test running configurations

Run the following commands to setup and configure chrome driver and google
chrome for running the tests:

```
cd /workspaces/step/packages/admin-portal

sudo apt-get update && \
sudo apt-get install -y libnss3 libglib2.0-0 libatk1.0-0 && \
wget https://mirror.cs.uchicago.edu/google-chrome/pool/main/g/google-chrome-stable/google-chrome-stable_92.0.4515.159-1_amd64.deb && \
sudo dpkg -i google-chrome-stable_92.0.4515.159-1_amd64.deb;
rm google-chrome-stable_92.0.4515.159-1_amd64.deb && \
sudo apt-get install -f -y && \
wget https://chromedriver.storage.googleapis.com/92.0.4515.107/chromedriver_linux64.zip && \
unzip chromedriver_linux64.zip && \
rm chromedriver_linux64.zip && \
sudo mv chromedriver /usr/bin/chromedriver && \
sudo chown root:root /usr/bin/chromedriver && \
sudo chmod +x /usr/bin/chromedriver
```