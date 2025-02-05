# SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

FROM rustlang/rust:nightly-slim

WORKDIR /app

RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    m4 make chromium libssl-dev protobuf-compiler libprotobuf-dev curl pkg-config wget software-properties-common && \
    wget https://download.java.net/java/GA/jdk22.0.2/c9ecb94cd31b495da20a27d4581645e8/9/GPL/openjdk-22.0.2_linux-x64_bin.tar.gz \
    && tar xvf openjdk-22.0.2_linux-x64_bin.tar.gz \
    && mv jdk-22.0.2  /usr/local/ \
    && update-alternatives --install /usr/bin/java java /usr/local/jdk-22.0.2/bin/java 1 \
    && update-alternatives --install /usr/bin/javac javac /usr/local/jdk-22.0.2/bin/javac 1 \
    && curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash && \
    cargo binstall -y cargo-watch && \
    # cleanups
    apt-get clean && \
    apt-get autoremove

#WORKDIR /app/e2e/src/mock_server
#RUN cargo build

#CMD ["cargo", "run"]