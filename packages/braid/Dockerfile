FROM rustlang/rust:nightly-slim

# copy projects
WORKDIR /app
COPY ./braid/scripts/Cargo.toml ./Cargo.toml
COPY ./braid ./braid
COPY ./board-messages ./board-messages
COPY ./sequent-core ./sequent-core
COPY ./immu-board ./immu-board
COPY ./immudb-rs ./immudb-rs
COPY ./strand ./strand
COPY ./wrap-map-err ./wrap-map-err

RUN apt-get update && apt-get install -y --no-install-recommends \
    protobuf-compiler \
    libprotobuf-dev \
    m4 \
    make \
    && rm -rf /var/lib/apt/lists/*
RUN rustup default nightly

# build projects
WORKDIR /app/immu-board
RUN cargo build --release
RUN cp /app/target/release/bb_helper /usr/bin
WORKDIR /app/braid
RUN cargo build --release
RUN cp /app/target/release/main /usr/bin/trustee
RUN cp /app/target/release/gen_trustee_config /usr/bin/
RUN cp /app/target/release/main /usr/bin/trustee
RUN cp /app/braid/scripts/*.sh /usr/bin/
RUN mkdir -p /opt/braid/
RUN cp /app/braid/scripts/*.toml /opt/braid/

# init index db
RUN mkdir /tmp/cache

CMD ["/usr/bin/trustee.sh"]
