# SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

FROM quay.io/keycloak/keycloak:26.7.5@sha256:37dbaf6f0722c9ec246335f36e1ef8b2e6cb960f7c27e0d8c615121a3d475a85

COPY --chown=keycloak:keycloak \
    action-token-login-bridge/target/sequent.action-token-login-bridge.jar \
    conditional-authenticators/target/sequent.conditional-authenticators.jar \
    message-otp-authenticator/target/sequent.message-otp-authenticator.jar \
    voter-enrollment/target/sequent.voter-enrollment.jar \
    /opt/keycloak/providers/

RUN /opt/keycloak/bin/kc.sh build

ENTRYPOINT ["/opt/keycloak/bin/kc.sh"]
