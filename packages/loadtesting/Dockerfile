# SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

FROM rust:1.88.0-slim-bookworm AS builder
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    m4 make chromium libssl-dev protobuf-compiler libprotobuf-dev curl pkg-config wget && \
    wget https://download.java.net/java/GA/jdk22.0.2/c9ecb94cd31b495da20a27d4581645e8/9/GPL/openjdk-22.0.2_linux-x64_bin.tar.gz && \
    curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash && \
    rustup toolchain install nightly-2025-05-04 && \
    rustup default nightly-2025-05-04 && \
    cargo binstall -y cargo-watch && \
    # cleanups
    apt-get clean && \
    apt-get autoremove
WORKDIR /app
COPY . /app
# Uncomment when the step-cli or the e2e binary is used in the e2e tests
# RUN cd /app/packages && cargo build --release --bin=step-cli --bin=e2e

FROM ubuntu:noble
RUN apt-get update -y && \
    DEBIAN_FRONTEND=noninteractive apt-get install -y \
    python3 python3-pip python3-venv python3-dev libpq-dev \
    curl wget gnupg2 procps chromium-browser tree && \
    rm -rf /var/lib/apt/lists/*
RUN curl -sL https://deb.nodesource.com/setup_20.x -o /tmp/nodesource_setup.sh && \
      bash /tmp/nodesource_setup.sh && \
      apt install -y nodejs
WORKDIR /opt/sequentech
RUN mkdir -p /opt/sequentech
# Uncomment when the step-cli is used in the e2e tests
# COPY --from=builder /app/packages/target/release/step-cli /opt/sequentech/step-cli
# Uncomment when the e2e binary is used in the e2e tests
# COPY --from=builder /app/packages/target/release/e2e /opt/sequentech/e2e
COPY ./packages/windmill/external-bin/janitor/scripts/setup.sh /opt/sequentech/janitor-setup
RUN mkdir -p /opt/sequentech/janitor
COPY ./packages/windmill/external-bin/janitor/ /opt/sequentech/janitor
RUN chmod +x /opt/sequentech/janitor-setup
RUN DEBIAN_FRONTEND=noninteractive /opt/sequentech/janitor-setup
RUN rm /opt/sequentech/janitor-setup
ENV CHROME=/usr/bin/chromium-browser
ENV PATH="$PATH:/opt/sequentech"
RUN echo '#!/bin/bash\nsource /opt/sequent-step/venv/bin/activate\nexec python /opt/sequent-step/load_tool.py "$@"' > /opt/sequent-step/load-tool && \
    chmod +x /opt/sequent-step/load-tool
RUN echo '#!/bin/bash\nsource /opt/sequent-step/venv/bin/activate\nexec "$@"' > /opt/sequent-step/entrypoint_with_venv.sh && \
    chmod +x /opt/sequent-step/entrypoint_with_venv.sh

COPY ./packages/loadtesting/entrypoint.sh /entrypoint.sh
COPY ./packages/loadtesting/run_bg_voting.sh /run_bg_voting.sh
RUN chmod +x /entrypoint.sh
RUN chmod +x /run_bg_voting.sh
RUN mkdir -p /workspaces/step/packages /nightwatch/src/admin-portal/nightwatch /nightwatch/src/e2e /nightwatch/src/loadtesting /nightwatch/src/voting-portal/nightwatch
COPY ./packages/loadtesting/nightwatch/ /nightwatch/
COPY ./packages/admin-portal/test/ /nightwatch/src/admin-portal
COPY ./packages/voting-portal/test/ /nightwatch/src/voting-portal
RUN cd /nightwatch && npm install .

ENTRYPOINT ["/entrypoint.sh"]
