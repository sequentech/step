# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
install -d -m 700 /var/lib/sequent-site/tls
install -m 644 /root/deploy/tls/tls.crt /var/lib/sequent-site/tls/tls.crt
install -m 600 /root/deploy/tls/tls.key /var/lib/sequent-site/tls/tls.key
install -m 644 /root/deploy/tls/ca.pem /var/lib/sequent-site/tls/ca.pem
install -m 644 /root/deploy/tls/hosts.json /var/lib/sequent-site/tls/hosts.json
systemctl restart nginx
systemctl is-active nginx
ss -Hltnp 'sport = :443' | sed 's/.*users:(("\([^"]*\)".*/\1/' | sort -u
