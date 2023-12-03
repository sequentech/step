ARG ENVIRONMENT=dev
FROM rust:1.70.0 as windmill-builder

ARG ENVIRONMENT=${ENVIRONMENT}
RUN apt-get update && apt-get install -y chromium protobuf-compiler libprotobuf-dev
RUN rustup default nightly

WORKDIR /app
COPY ./immu-board ./immu-board
COPY ./immudb-rs ./immudb-rs
COPY ./strand ./strand
COPY ./braid-messages ./braid-messages
COPY ./sequent-core ./sequent-core
COPY ./windmill ./windmill
COPY ./wrap-map-err ./wrap-map-err
COPY ./velvet ./velvet

WORKDIR /app/windmill
RUN if [ "$ENVIRONMENT" = "prod" ]; then \
        cargo build --release; \
    else \
        cargo build; \
    fi


FROM windmill-builder as dev

WORKDIR /app/windmill
CMD ["cargo", "run", "--bin", "main", "consume", "-q", "short_queue", "tally_queue", "beat", "reports_queue", "beat" ]


FROM debian:bullseye as prod
# Install any necessary runtime dependencies for production
RUN apt-get update && apt-get install -y libssl1.1 && rm -rf /var/lib/apt/lists/*

# Copy the compiled binary from the builder stage
COPY --from=windmill-builder /app/windmill/target/release/main /usr/local/bin/main

WORKDIR /usr/local/bin
CMD ["main", "consume", "-q", "short_queue", "tally_queue", "beat", "reports_queue", "beat"]

FROM ${ENVIRONMENT} as final