# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
FROM ghcr.io/cachix/devenv/devcontainer:v1.10
ARG STEP_ENV_FINGERPRINT
LABEL io.sequent.devenv-key=$STEP_ENV_FINGERPRINT
USER root
RUN mkdir -p /opt/step-env && chown vscode:vscode /opt/step-env
COPY --chown=vscode:vscode devenv.nix devenv.lock devenv.yaml /opt/step-env/
COPY .devcontainer/warm-env.sh /usr/local/bin/step-warm-env
USER vscode
WORKDIR /opt/step-env
RUN /nix-entrypoint.sh bash /usr/local/bin/step-warm-env
WORKDIR /workspaces
