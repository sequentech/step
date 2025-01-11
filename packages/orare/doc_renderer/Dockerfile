ARG BASE_IMAGE=alpine:latest
ARG RUST_LOG="debug"
# FIXME(ereslibre): identify exactly which args to forward as envvars
# are required when the openwhisk backend is used and list only
# those. In the OpenWhisk case, we want to have these settings built
# into the image, as at invocation time it's tricky to set environment
# variables on the lambda.
ARG AWS_S3_PRIVATE_URI=""
ARG AWS_S3_PUBLIC_URI=""
ARG AWS_S3_BUCKET=""
ARG AWS_S3_PUBLIC_BUCKET=""

FROM rustlang/rust:nightly AS builder
ARG FEATURES=openwhisk

RUN apt-get update && \
    apt-get install -y musl-tools

ENV CC=musl-gcc
ENV RUSTFLAGS="-C target-feature=+crt-static"

WORKDIR /app
COPY ./sequent-core ./sequent-core
COPY ./orare ./orare
COPY ./strand ./strand
COPY ./Cargo.lock ./Cargo.lock
RUN rustup target add x86_64-unknown-linux-musl
RUN cd orare/doc_renderer && \
    cargo fetch --target x86_64-unknown-linux-musl && \
    cargo build --release --target x86_64-unknown-linux-musl --features=${FEATURES}

FROM ${BASE_IMAGE}
ARG RUST_LOG
ARG AWS_S3_PRIVATE_URI
ARG AWS_S3_PUBLIC_URI
ARG AWS_S3_BUCKET
ARG AWS_S3_PUBLIC_BUCKET
ENV RUST_LOG=${RUST_LOG}
ENV AWS_S3_PRIVATE_URI=${AWS_S3_PRIVATE_URI}
ENV AWS_S3_PUBLIC_URI=${AWS_S3_PUBLIC_URI}
ENV AWS_S3_BUCKET=${AWS_S3_BUCKET}
ENV AWS_S3_PUBLIC_BUCKET=${AWS_S3_PUBLIC_BUCKET}
RUN apk add ca-certificates chromium curl
ADD ./orare/doc_renderer/assets ./assets
COPY --from=builder \
    /app/orare/doc_renderer/target/x86_64-unknown-linux-musl/release/doc_renderer \
    /doc_renderer
EXPOSE 8080
ENTRYPOINT ["/doc_renderer"]
