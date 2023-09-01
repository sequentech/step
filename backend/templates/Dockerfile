FROM rust:1.70.0

ENV ROCKET_ADDRESS=0.0.0.0
ENV ROCKET_PORT=8400

WORKDIR /app
COPY ./templates ./templates
COPY ./immu-board ./immu-board
COPY ./immudb-rs ./immudb-rs
COPY ./strand ./strand
COPY ./braid ./braid

RUN apt-get update && apt-get install -y chromium protobuf-compiler libprotobuf-dev
RUN rustup default nightly

WORKDIR /app/templates
RUN cargo build

CMD ["cargo", "run"]