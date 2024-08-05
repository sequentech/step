FROM rust:1.70.0

RUN cargo install wasm-pack

WORKDIR /app

ENV WASM_DIR=/app/wasm
ENV CONFIG_DIR=/app/config

RUN mkdir -p $WASM_DIR $CONFIG_DIR

COPY . .

COPY ./build_extensions.sh /app/build_extensions.sh
RUN chmod +x /app/build_extensions.sh

RUN ./build_extensions.sh

ENTRYPOINT ["cargo", "run", "--release"]