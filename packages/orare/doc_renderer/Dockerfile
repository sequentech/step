FROM rustlang/rust:nightly AS builder

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
    cargo build --release --target x86_64-unknown-linux-musl

FROM openwhisk/dockerskeleton
ENV FLASK_PROXY_PORT 8080
RUN apk --no-cache add ca-certificates chromium
COPY --from=builder \
    /app/orare/doc_renderer/target/x86_64-unknown-linux-musl/release/doc_renderer \
    ./action/exec
CMD ["/bin/bash", "-c", "python -u actionproxy.py"]
