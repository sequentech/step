# SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

# Build stage
FROM openwhisk/rust1.70 as builder

WORKDIR /app
COPY ./sequent-core ./sequent-core
COPY ./orare ./orare

# Install Chrome dependencies and Chrome itself
RUN apt-get update && apt-get install -y \
    chromium \
    protobuf-compiler \
    libprotobuf-dev \
    && rm -rf /var/lib/apt/lists/*

# Build orare with OpenWhisk feature
WORKDIR /app/orare/doc_renderer
RUN cargo build --release --features openwhisk

# OpenWhisk runtime image
FROM openwhisk/action-rust-v1.70

# Copy the built binary, dependencies, and action config
COPY --from=builder /app/orare/doc_renderer/target/release/doc_renderer /action/exec
COPY --from=builder /usr/bin/chromium /usr/bin/chromium
COPY ./orare/doc_renderer/action.yml /action/action.yml

# Set up environment
ENV PDF_TRANSPORT_NAME=orare-openwhisk \
    CHROME_PATH=/usr/bin/chromium \
    OPENWHISK_ACTION=true \
    OPENWHISK_ACTION_NAME=pdf-renderer \
    OPENWHISK_ACTION_VERSION=1.0.0

# OpenWhisk specific setup
RUN chmod 755 /action/exec && \
    # Register the action with OpenWhisk
    wsk action create pdf-tools/pdf-renderer \
        --kind rust:1.70 \
        --main main \
        --web true \
        --annotation provide-api-key true \
        --annotation raw-http true \
        /action/action.yml

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8082/health || exit 1

CMD ["cargo", "run", "--features", "openwhisk"] 