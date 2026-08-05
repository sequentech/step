# SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

FROM rust:1.96.0-slim-bookworm AS builder
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    m4 make chromium libssl-dev protobuf-compiler libprotobuf-dev curl pkg-config wget && \
    wget https://download.java.net/java/GA/jdk22.0.2/c9ecb94cd31b495da20a27d4581645e8/9/GPL/openjdk-22.0.2_linux-x64_bin.tar.gz && \
    curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash && \
    cargo binstall -y cargo-watch && \
    # cleanups
    apt-get clean && \
    apt-get autoremove
WORKDIR /app
COPY . /app
RUN cd /app/packages && cargo build --release --bin=step-cli --bin=e2e

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
COPY --from=builder /app/packages/target/release/step-cli /opt/sequentech/step-cli
COPY --from=builder /app/packages/target/release/e2e /opt/sequentech/e2e
# The load-testing tool lives in beyond/packages/load-tool. Its dependencies and
# the tool itself are installed here rather than copied from this repo, which is
# what the janitor setup.sh this replaces already did: it downloaded load_tool.py
# from a gist, so the copy that used to sit in step was never the one that ran.
#
# FIXME the gist is not a versioned artifact. Publish load_tool.py from beyond
# and pin it here — see https://github.com/sequentech/meta/issues/12744.
ARG LOAD_TOOL_URL=https://gist.githubusercontent.com/edulix/875a9a5d26407e1530f7769419dd8961/raw/341d7bab0482913d6b56efad512f383e5869eb85/load_tool.py
RUN mkdir -p /opt/sequent-step && \
    python3 -m venv /opt/sequent-step/venv && \
    /opt/sequent-step/venv/bin/pip install --upgrade pip && \
    /opt/sequent-step/venv/bin/pip install \
        Faker==13.3.4 \
        psycopg2==2.9.10 \
        openpyxl==3.1.5 \
        pyzipper==0.3.6 \
        python-dotenv==1.0.1 \
        pybars3==0.9.7 && \
    wget -q -O /opt/sequent-step/load_tool.py "$LOAD_TOOL_URL"
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
RUN mkdir -p /workspaces/step/packages /nightwatch/src/admin-portal /nightwatch/src/e2e /nightwatch/src/loadtesting /nightwatch/src/voting-portal
COPY ./packages/loadtesting/nightwatch/ /nightwatch/
COPY ./packages/admin-portal/nightwatch/ /nightwatch/src/admin-portal
COPY ./packages/e2e/src/scenarios/ /nightwatch/src/e2e
COPY ./packages/voting-portal/nightwatch/ /nightwatch/src/voting-portal
RUN cd /nightwatch && npm install .

ENTRYPOINT ["/entrypoint.sh"]
