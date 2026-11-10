#!/bin/sh
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

# Renders the nginx configuration of the TLS endpoint from the three service
# hostnames of the installation, checks it and runs nginx in the foreground.
# The installation supplies the certificate, its key, the CA certificate of the
# internal upstream and the hostnames; until it has, nothing listens.
set -eu
umask 077
SITE=/var/lib/sequent-site/tls

# The image preloads the hardened_malloc memory allocator (LD_PRELOAD); nginx runs
# only with it, so this shell, which becomes nginx, must have it mapped.
grep -q ' /usr/lib/libhardened_malloc\.so$' "/proc/$$/maps" || {
    echo 'The hardened_malloc memory allocator is not loaded (LD_PRELOAD=/usr/lib/libhardened_malloc.so); nginx is not started' >&2
    exit 1
}

for file in tls.crt tls.key ca.pem hosts.json; do
    until [ -s "$SITE/$file" ]; do sleep 5; done
done

# Three distinct literal DNS hostnames, in lower case; anything else stops here.
hosts=$(jq -r '
    def dns_label: "[A-Za-z0-9](?:[A-Za-z0-9-]{0,61}[A-Za-z0-9])?";
    if type != "array" or length != 3 then
        error("Exactly three service hostnames are required")
    elif any(.[]; type != "string" or length > 253
            or (test("\\A" + dns_label + "(?:\\." + dns_label + ")*\\z") | not)) then
        error("Use literal DNS hostnames; wildcards are not permitted")
    elif (map(ascii_downcase) | unique | length) != 3 then
        error("The three service hostnames must be distinct")
    else
        .[] | ascii_downcase
    end' "$SITE/hosts.json")
# One argument per hostname: they have no blanks or glob characters.
set -- $hosts

{
    cat <<'EOF'
# Generated from the validated installation hostnames. Do not edit.
user nginx nginx;
worker_processes auto;
pid /run/sequent-nginx/nginx.pid;
error_log stderr warn;
events { worker_connections 1024; }
http {
  map $host $activity_host {
    default __other__;
EOF
    for host; do printf '    %s %s;\n' "$host" "$host"; done
    cat <<'EOF'
  }
  map $request_method $activity_method {
    default OTHER;
    GET GET; POST POST; PUT PUT; DELETE DELETE;
    HEAD HEAD; OPTIONS OPTIONS; PATCH PATCH;
  }
  log_format security_activity escape=json
    '{"timestamp":"$msec","host":"$activity_host","method":"$activity_method","status":$status}';
  access_log /var/log/sequent-security/requests.jsonl security_activity buffer=32k flush=1s;
  server_tokens off;
  client_max_body_size 0;
  client_body_temp_path /var/cache/sequent-nginx/body;
  proxy_temp_path /var/cache/sequent-nginx/proxy;
  map $http_upgrade $connection_upgrade {
    default upgrade;
    '' close;
  }
  # Paths that a later server could read differently from nginx are refused: a
  # parameter (;), a fragment (#), a backslash, or an encoded ;, / or backslash.
  map $request_uri $path_refused {
    default 0;
    "~*^[^?]*(?:[;#\x5c]|%(?:3b|2f|5c))" 1;
  }
  ssl_protocols TLSv1.3;
  ssl_conf_command Ciphersuites TLS_AES_256_GCM_SHA384;
  ssl_ecdh_curve secp384r1;
  ssl_session_tickets off;
  server {
    listen 443 ssl default_server;
    ssl_reject_handshake on;
    return 421;
  }
  server {
    listen 443 ssl;
EOF
    printf '    server_name %s %s %s;\n' "$1" "$2" "$3"
    cat <<'EOF'
    ssl_certificate /var/lib/sequent-site/tls/tls.crt;
    ssl_certificate_key /var/lib/sequent-site/tls/tls.key;
    if ($host != $ssl_server_name) { return 421; }
    if ($path_refused) { return 400; }
    # Keycloak's master realm is not served through this endpoint.
    location ~* ^/realms/master(?:/|$) { return 403; }
    location / {
      proxy_pass https://127.0.0.1:10443;
      proxy_ssl_protocols TLSv1.3;
      proxy_ssl_conf_command Ciphersuites TLS_AES_256_GCM_SHA384;
      proxy_ssl_conf_command Groups secp384r1;
      proxy_ssl_server_name on;
      proxy_ssl_name $host;
      proxy_ssl_verify on;
      proxy_ssl_verify_depth 5;
      proxy_ssl_trusted_certificate /var/lib/sequent-site/tls/ca.pem;
      proxy_ssl_session_reuse off;
      proxy_http_version 1.1;
      # OIDC redirects and session cookies can exceed nginx's page-sized default.
      # Keep response-header/body buffering explicitly bounded per connection.
      proxy_buffer_size 32k;
      proxy_buffers 4 32k;
      proxy_busy_buffers_size 64k;
      proxy_set_header Host $host;
      proxy_set_header X-Real-IP $remote_addr;
      proxy_set_header X-Forwarded-For $remote_addr;
      proxy_set_header X-Forwarded-Host $host;
      proxy_set_header X-Forwarded-Port 443;
      proxy_set_header X-Forwarded-Proto https;
      proxy_set_header Forwarded "";
      proxy_set_header X-Forwarded-Prefix "";
      proxy_set_header X-Forwarded-Server "";
      proxy_set_header X-Forwarded-Uri "";
      proxy_set_header X-Forwarded-Method "";
      proxy_set_header X-Forwarded-Tls-Client-Cert "";
      proxy_set_header X-Forwarded-Tls-Client-Cert-Info "";
      proxy_set_header Upgrade $http_upgrade;
      proxy_set_header Connection $connection_upgrade;
      proxy_read_timeout 300s;
      proxy_send_timeout 300s;
      proxy_request_buffering off;
    }
  }
}
EOF
} > /etc/nginx/nginx.conf
chmod 600 /etc/nginx/nginx.conf
nginx -t
exec nginx -g 'daemon off;'
