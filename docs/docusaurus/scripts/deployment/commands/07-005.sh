# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
cd /root/deploy && umask 077 && set -o noclobber
{
  for v in POSTGRES_PASSWORD RUSTFS_PASSWORD ACTIONS_ADMIN_SECRET MASTER_SECRET \
      HASURA_ADMIN_SECRET QUEUE_DB_OWNER_PASSWORD QUEUE_DB_WORKER_PASSWORD \
      QUEUE_DB_PRODUCER_PASSWORD QUEUE_DB_READER_PASSWORD ELECTORAL_LOG_PG_PASSWORD \
      ELECTORAL_LOG_PG_PROVISIONING_PASSWORD ELECTORAL_LOG_PG_READER_PASSWORD \
      KC_ADMIN_PASSWORD KC_SERVICE_ACCOUNT_SECRET KC_API_KEY_CLIENT_SECRET \
      TRUSTEE1_PASSWORD TRUSTEE2_PASSWORD; do
    echo "export $v=$(openssl rand -hex 32)"
  done
  echo 'export S3_SECRET="$RUSTFS_PASSWORD"'
  echo 'export KC_ADMIN_USER=<name-of-the-service-administrator>'
} > secrets.env
set +o noclobber
stat -c '%a %U' secrets.env      # 600 root
grep -c '^export ' secrets.env   # 19
