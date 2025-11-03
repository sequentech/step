# SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

FROM rust:1.82.0

WORKDIR /app
COPY ./immu-board ./immu-board
COPY ./immudb-rs ./immudb-rs
COPY ./strand ./strand
COPY ./board-messages ./board-messages
COPY ./sequent-core ./sequent-core
COPY ./windmill ./windmill
COPY ./wrap-map-err ./wrap-map-err
COPY ./velvet ./velvet

RUN cp /app/windmill/external-bin/ecies-tool.jar /usr/local/bin/

RUN rustup toolchain install nightly-2024-09-01 \
    && rustup default nightly-2024-09-01

# Use Docker build arguments to determine the architecture
ARG TARGETARCH

# Set Java version
ENV JAVA_VERSION=22.0.2
ENV JAVA_BUILD=c9ecb94cd31b495da20a27d4581645e8/9

# Default to linux-x64, which is the most common.
# This will be overridden by the conditional logic below.
ENV JAVA_OS_ARCH=linux-x64
ENV JAVA_FILENAME=openjdk-${JAVA_VERSION}_linux-x64_bin.tar.gz

# Conditional logic to set the correct download URL and filename based on architecture
RUN case "${TARGETARCH}" in \
    "amd64") \
        export JAVA_OS_ARCH=linux-x64; \
        export JAVA_FILENAME=openjdk-${JAVA_VERSION}_linux-x64_bin.tar.gz; \
        ;; \
    "arm64") \
        export JAVA_OS_ARCH=linux-aarch64; \
        export JAVA_FILENAME=openjdk-${JAVA_VERSION}_linux-aarch64_bin.tar.gz; \
        ;; \
    *) \
        echo "Unsupported architecture: ${TARGETARCH}"; \
        exit 1; \
        ;; \
    esac; \
    echo "Downloading OpenJDK for ${JAVA_OS_ARCH}"; \
    echo "Download url https://download.java.net/java/GA/jdk${JAVA_VERSION}/${JAVA_BUILD}/GPL/${JAVA_FILENAME}"; \
    echo "JAVA_FILENAME=${JAVA_FILENAME}" >> /etc/profile.d/java.sh; \
    apt-get update && apt-get install -y chromium protobuf-compiler libprotobuf-dev wget software-properties-common \
    wget https://download.java.net/java/GA/jdk${JAVA_VERSION}/${JAVA_BUILD}/GPL/${JAVA_FILENAME} \
    && tar xvf ${JAVA_FILENAME} \
    && mv jdk-${JAVA_VERSION} /usr/local/ \
    && update-alternatives --install /usr/bin/java java /usr/local/jdk-${JAVA_VERSION}/bin/java 1 \
    && update-alternatives --install /usr/bin/javac javac /usr/local/jdk-${JAVA_VERSION}/bin/javac 1

# java -jar ecies-tool.jar create-keys public.pem private.pem
RUN rustup default nightly

WORKDIR /app/windmill
RUN cargo build

CMD ["cargo", "run", "--bin", "main", "consume", "-q", "short_queue", "tally_queue", "beat", "reports_queue" ]