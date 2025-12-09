ARG BASE_IMAGE=alpine:3.17@sha256:8fc3dacfb6d69da8d44e42390de777e48577085db99aa4e4af35f483eb08b989
ARG FEATURES
ARG RUST_LOG=debug
ARG AWS_S3_PRIVATE_URI=""
ARG AWS_S3_PUBLIC_URI=""
ARG AWS_S3_BUCKET=""
ARG AWS_S3_PUBLIC_BUCKET=""
ARG AWS_REGION=""
ARG AWS_S3_ACCESS_KEY=""
ARG AWS_S3_ACCESS_SECRET=""
ARG AWS_S3_MAX_UPLOAD_BYTES=""
ARG AWS_S3_UPLOAD_EXPIRATION_SECS=""
ARG AWS_S3_FETCH_EXPIRATION_SECS=""

FROM rust:1.90.0-slim-bookworm AS builder
ARG FEATURES

RUN apt-get update && \
    apt-get install -y musl-tools

ENV CC=musl-gcc
ENV RUSTFLAGS="-C target-feature=+crt-static"

WORKDIR /app
COPY ./sequent-core ./sequent-core
COPY ./orare ./orare
COPY ./strand ./strand
COPY ./Cargo.lock ./Cargo.lock
COPY ./.cargo ./.cargo
RUN rustup target add x86_64-unknown-linux-musl
RUN cd orare/doc_renderer && \
    cargo fetch --target x86_64-unknown-linux-musl && \
    cargo build \
        --release \
        --target x86_64-unknown-linux-musl \
        --no-default-features \
        --features=${FEATURES}

FROM ${BASE_IMAGE}
ARG RUST_LOG
ARG AWS_S3_PRIVATE_URI
ARG AWS_S3_PUBLIC_URI
ARG AWS_S3_BUCKET
ARG AWS_S3_PUBLIC_BUCKET
ARG AWS_REGION
ARG AWS_S3_ACCESS_KEY
ARG AWS_S3_ACCESS_SECRET
ARG AWS_S3_MAX_UPLOAD_BYTES
ARG AWS_S3_UPLOAD_EXPIRATION_SECS
ARG AWS_S3_FETCH_EXPIRATION_SECS
ENV RUST_LOG=${RUST_LOG}
ENV AWS_S3_PRIVATE_URI=${AWS_S3_PRIVATE_URI}
ENV AWS_S3_PUBLIC_URI=${AWS_S3_PUBLIC_URI}
ENV AWS_S3_BUCKET=${AWS_S3_BUCKET}
ENV AWS_S3_PUBLIC_BUCKET=${AWS_S3_PUBLIC_BUCKET}
ENV AWS_REGION=${AWS_REGION}
ENV AWS_S3_ACCESS_KEY=${AWS_S3_ACCESS_KEY}
ENV AWS_S3_ACCESS_SECRET=${AWS_S3_ACCESS_SECRET}
ENV AWS_S3_MAX_UPLOAD_BYTES=${AWS_S3_MAX_UPLOAD_BYTES}
ENV AWS_S3_UPLOAD_EXPIRATION_SECS=${AWS_S3_UPLOAD_EXPIRATION_SECS}
ENV AWS_S3_FETCH_EXPIRATION_SECS=${AWS_S3_FETCH_EXPIRATION_SECS}
RUN apk add ca-certificates chromium curl
ADD ./orare/doc_renderer/assets ./assets
COPY --from=builder \
    /app/orare/doc_renderer/target/x86_64-unknown-linux-musl/release/doc_renderer \
    /doc_renderer
EXPOSE 8080
ENTRYPOINT ["/doc_renderer"]
