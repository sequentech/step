#!/bin/sh
# SPDX-FileCopyrightText: 2023-2024 Sequent Tech <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

cp -rf /workspaces/step/.devcontainer/keycloak/import /opt/keycloak/data/import
mkdir /opt/keycloak/tmp
/opt/keycloak/bin/kc.sh start-dev --features=preview --health-enabled=true --spi-user-profile-declarative-user-profile-read-only-attributes=area-id,tenant-id --spi-user-profile-declarative-user-profile-admin-read-only-attributes=sequent.admin-read-only.* -Dkeycloak.profile.feature.upload_scripts=enabled -Djava.io.tmpdir=/opt/keycloak/tmp --spi-email-sender-provider=dummy --spi-email-sender-dummy-enabled=true --spi-email-sender-default-enabled=false --spi-sms-sender-provider=dummy --spi-sms-sender-dummy-enabled=true --spi-sms-sender-aws-enabled=false --import-realm