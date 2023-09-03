FROM rust:1.70.0

ENV ROCKET_ADDRESS=0.0.0.0
ENV ROCKET_PORT=8400

WORKDIR /app
COPY ./harvest ./harvest
COPY ./immu-board ./immu-board
COPY ./immudb-rs ./immudb-rs

RUN apt-get update && apt-get install -y chromium protobuf-compiler libprotobuf-dev
RUN rustup default nightly

WORKDIR /app/harvest
RUN cargo build

CMD ["cargo", "run"]
