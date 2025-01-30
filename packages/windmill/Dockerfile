# SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

FROM rust:1.82.0

WORKDIR /app
COPY ./immu-board ./immu-board
COPY ./immudb-rs ./immudb-rs
COPY ./strand ./strand
COPY ./board-messages ./board-messages
COPY ./sequent-core ./sequent-core
COPY ./windmill ./windmill
COPY ./wrap-map-err ./wrap-map-err
COPY ./velvet ./velvet

RUN cp /app/windmill/external-bin/ecies-tool.jar /usr/local/bin/

RUN apt-get update && apt-get install -y chromium protobuf-compiler libprotobuf-dev wget software-properties-common \
    wget https://download.java.net/java/GA/jdk22.0.2/c9ecb94cd31b495da20a27d4581645e8/9/GPL/openjdk-22.0.2_linux-x64_bin.tar.gz \
    && tar xvf openjdk-22.0.2_linux-x64_bin.tar.gz \
    && mv jdk-22.0.2  /usr/local/ \
    && update-alternatives --install /usr/bin/java java /usr/local/jdk-22.0.2/bin/java 1 \
    && update-alternatives --install /usr/bin/javac javac /usr/local/jdk-22.0.2/bin/javac 1

# java -jar ecies-tool.jar create-keys public.pem private.pem
RUN rustup default nightly

WORKDIR /app/windmill
RUN cargo build

CMD ["cargo", "run", "--bin", "main", "consume", "-q", "short_queue", "tally_queue", "beat", "reports_queue" ]