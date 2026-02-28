# SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

# Build context must be .devcontainer/ (set in docker-compose).
FROM nginx:alpine
COPY keycloak-nginx/keycloak-mtls.conf /etc/nginx/conf.d/keycloak-mtls.conf
COPY certs/nginx-tls.crt               /etc/nginx/certs/nginx-tls.crt
COPY certs/nginx-tls.key               /etc/nginx/certs/nginx-tls.key
COPY minio/public-assets/client-ca.pem /etc/nginx/client-ca/client-ca.pem
