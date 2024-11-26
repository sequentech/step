# SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

FROM rust:1.70.0

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
RUN cargo build --features openwhisk

# Set up the OpenWhisk service
EXPOSE 8082
ENV PDF_TRANSPORT_NAME=orare-openwhisk
ENV CHROME_PATH=/usr/bin/chromium

CMD ["cargo", "run", "--features", "openwhisk"] 