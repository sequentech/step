# SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

FROM quay.io/keycloak/keycloak:26.6.1

COPY --chown=keycloak:keycloak \
    action-token-login-bridge/target/sequent.action-token-login-bridge.jar \
    conditional-authenticators/target/sequent.conditional-authenticators.jar \
    message-otp-authenticator/target/sequent.message-otp-authenticator.jar \
    voter-enrollment/target/sequent.voter-enrollment.jar \
    /opt/keycloak/providers/

RUN /opt/keycloak/bin/kc.sh build

ENTRYPOINT ["/opt/keycloak/bin/kc.sh"]
