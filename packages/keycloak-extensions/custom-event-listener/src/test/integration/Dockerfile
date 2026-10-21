# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

FROM quay.io/keycloak/keycloak:26.7.5@sha256:37dbaf6f0722c9ec246335f36e1ef8b2e6cb960f7c27e0d8c615121a3d475a85

COPY --chown=keycloak:keycloak \
    action-token-login-bridge/target/sequent.action-token-login-bridge.jar \
    message-otp-authenticator/target/sequent.message-otp-authenticator.jar \
    custom-event-listener/target/sequent.custom-event-listener.jar \
    /opt/keycloak/providers/

RUN mkdir -p /opt/keycloak/data/import && /opt/keycloak/bin/kc.sh build --db=postgres

ENTRYPOINT ["/opt/keycloak/bin/kc.sh"]
