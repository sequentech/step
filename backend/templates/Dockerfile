FROM rust:1.70.0

ENV ROCKET_ADDRESS=0.0.0.0
ENV ROCKET_PORT=8400

WORKDIR /app
COPY . .

RUN apt-get update && apt-get install -y chromium
RUN rustup default nightly
RUN cargo build

CMD ["cargo", "run"]