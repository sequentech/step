FROM rust:1.72.0

# copy projects
WORKDIR /app
COPY ./braid/scripts/Cargo.toml ./Cargo.toml
COPY ./braid ./braid
COPY ./immu-board ./immu-board
COPY ./immudb-rs ./immudb-rs
COPY ./strand ./strand

RUN apt-get update && apt-get install -y protobuf-compiler libprotobuf-dev

# build projects
WORKDIR /app/immu-board
RUN cargo build --release
RUN cp /app/target/release/bb_helper /usr/bin
WORKDIR /app/braid
RUN cargo build --release --features protocol-manager
RUN cp /app/target/release/main /usr/bin/trustee
RUN cp /app/target/release/gen_trustee_config /usr/bin
RUN cp /app/target/release/protocol_manager /usr/bin
RUN cp /app/target/release/main /usr/bin/trustee
RUN cp /app/braid/scripts/*.sh /usr/bin/*.sh

# init index db
RUN mkdir /tmp/cache

CMD ["/usr/bin/trustee.sh"]
