FROM rust:1.70.0

WORKDIR /app
COPY ./immu-board ./immu-board
COPY ./immudb-rs ./immudb-rs
COPY ./strand ./strand
COPY ./braid-messages ./braid-messages
COPY ./sequent-core ./sequent-core
COPY ./windmill ./windmill

RUN apt-get update && apt-get install -y chromium protobuf-compiler libprotobuf-dev
RUN rustup default nightly

WORKDIR /app/windmill
RUN cargo build

CMD ["cargo", "run", "consume"]