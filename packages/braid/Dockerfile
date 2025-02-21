# SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

FROM rustlang/rust:nightly-slim

# copy projects
WORKDIR /app
COPY ./braid/scripts/Cargo.toml ./Cargo.toml
COPY ./braid ./braid
COPY ./b3 ./b3
COPY ./strand ./strand
COPY ./Cargo.lock ./Cargo.lock
COPY ./.cargo ./.cargo

RUN apt-get update && apt-get install -y --no-install-recommends \
    protobuf-compiler \
    libprotobuf-dev \
    m4 \
    make \
    unzip \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Install the specific nightly toolchain
RUN rustup toolchain install nightly-2024-09-01 \
    && rustup default nightly-2024-09-01

ARG VAULT_VERSION=1.15.5
#install vault
RUN if [ "${VAULT_VERSION}" = "latest" ]; then \
        VAULT_VERSION=$(curl -s https://releases.hashicorp.com/vault/ | grep -oP 'vault/\K[0-9.]+(?=/")' | head -n 1); \
    fi \
    && curl -o vault.zip https://releases.hashicorp.com/vault/${VAULT_VERSION}/vault_${VAULT_VERSION}_linux_amd64.zip \
    && unzip vault.zip -d /usr/local/bin/ \
    && rm -f vault.zip
# build projects
WORKDIR /app/braid
RUN cargo build --release
RUN cp /app/target/release/main /usr/bin/trustee
RUN cp /app/target/release/gen_trustee_config /usr/bin/
RUN cp /app/target/release/main /usr/bin/trustee
RUN cp /app/braid/scripts/*.sh /usr/bin/
RUN mkdir -p /opt/braid/
RUN cp /app/braid/scripts/*.toml /opt/braid/

# init index db
RUN mkdir /tmp/cache

CMD ["/usr/bin/trustee.sh"]
