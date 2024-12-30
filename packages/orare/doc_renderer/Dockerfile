ARG BASE_IMAGE
ARG PDF_TRANSPORT_NAME

FROM rustlang/rust:nightly AS builder
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
RUN rustup target add x86_64-unknown-linux-musl
RUN cd orare/doc_renderer && \
    cargo fetch --target x86_64-unknown-linux-musl && \
    cargo build --release --target x86_64-unknown-linux-musl --features=${FEATURES}

FROM ${BASE_IMAGE}
ARG PDF_TRANSPORT_NAME
RUN apk add ca-certificates chromium curl
ENV PDF_TRANSPORT_NAME=${PDF_TRANSPORT_NAME}
ENV RUST_LOG=debug
COPY --from=builder \
    /app/orare/doc_renderer/target/x86_64-unknown-linux-musl/release/doc_renderer \
    /doc_renderer
EXPOSE 8080
ENTRYPOINT ["/doc_renderer"]
