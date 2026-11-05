# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

# MinIO no longer publishes community images or binaries, so the backend E2E
# stack builds the final community release from source.
FROM golang:1.24-bookworm AS build
ARG MINIO_RELEASE=RELEASE.2025-10-15T17-29-55Z
ARG MINIO_COMMIT=9e49d5e7a648f00e26f2246f4dc28e6b07f8c84a
RUN git clone --depth 1 --branch "$MINIO_RELEASE" https://github.com/minio/minio.git /src \
    && test "$(git -C /src rev-parse HEAD)" = "$MINIO_COMMIT"
WORKDIR /src
RUN --mount=type=cache,target=/root/go/pkg/mod \
    --mount=type=cache,target=/root/.cache/go-build \
    CGO_ENABLED=0 go build -trimpath -tags kqueue \
        -ldflags "$(go run buildscripts/gen-ldflags.go)" -o /out/minio

FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*
COPY --from=build /out/minio /usr/bin/minio
ENTRYPOINT ["minio"]
