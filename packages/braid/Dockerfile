ARG ENVIRONMENT=dev

FROM rust:1.72.0 as braid-builder

ARG ENVIRONMENT=${ENVIRONMENT}
RUN apt-get update && apt-get install -y protobuf-compiler libprotobuf-dev
RUN rustup default nightly

# copy projects
WORKDIR /app
COPY ./braid/scripts/Cargo.toml ./Cargo.toml
COPY ./braid ./braid
COPY ./braid-messages ./braid-messages
COPY ./sequent-core ./sequent-core
COPY ./immu-board ./immu-board
COPY ./immudb-rs ./immudb-rs
COPY ./strand ./strand
COPY ./wrap-map-err ./wrap-map-err


# build projects
WORKDIR /app/immu-board
RUN cargo build --release

WORKDIR /app/braid
RUN cargo build --release

#=====Dev==================#
FROM braid-builder as dev
WORKDIR /app
#==========================#

#----Prod------------------#
FROM debian:bullseye as prod

# Some shared libraries (extra-runtime-dependencies) may need to be installed:
RUN apt-get update && apt-get install -y libssl1.1 && rm -rf /var/lib/apt/lists/*
#--------------------------#

#====Final=================#
FROM ${ENVIRONMENT} as final

# Setup binaries, scripts & config
COPY --from=braid-builder /app/target/release/bb_helper /usr/bin
COPY --from=braid-builder /app/target/release/main /usr/bin/trustee
COPY --from=braid-builder /app/target/release/gen_trustee_config /usr/bin/
COPY --from=braid-builder /app/target/release/main /usr/bin/trustee
COPY --from=braid-builder /app/braid/scripts/*.sh /usr/bin/
RUN mkdir -p /opt/braid/
COPY --from=braid-builder /app/braid/scripts/*.toml /opt/braid/

# init index db
RUN mkdir /tmp/cache

CMD ["/usr/bin/trustee.sh"]
#==========================#