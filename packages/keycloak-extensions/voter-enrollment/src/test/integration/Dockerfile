# SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

FROM quay.io/keycloak/keycloak:26.8.0@sha256:b0f60d489d51c5d113390bdf5461d4c06e6051be026c05549f2e1e10ec352bcc

COPY --chown=keycloak:keycloak \
    action-token-login-bridge/target/sequent.action-token-login-bridge.jar \
    conditional-authenticators/target/sequent.conditional-authenticators.jar \
    message-otp-authenticator/target/sequent.message-otp-authenticator.jar \
    voter-enrollment/target/sequent.voter-enrollment.jar \
    /opt/keycloak/providers/

RUN /opt/keycloak/bin/kc.sh build

ENTRYPOINT ["/opt/keycloak/bin/kc.sh"]
