# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only

# Same scripts as .devcontainer/minio/Dockerfile, with the MinIO client taken
# from its GitHub release because its images are no longer public.
FROM debian:bookworm-slim
ARG MC_RELEASE=RELEASE.2025-08-13T08-35-41Z
ARG TARGETARCH
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/* \
    && case "$TARGETARCH" in \
        amd64) sum=01f866e9c5f9b87c2b09116fa5d7c06695b106242d829a8bb32990c00312e891 ;; \
        arm64) sum=14c8c9616cfce4636add161304353244e8de383b2e2752c0e9dad01d4c27c12c ;; \
        *) echo "Unsupported architecture: $TARGETARCH" >&2; exit 1 ;; \
    esac \
    && curl -fsSL -o /usr/local/bin/mc \
        "https://github.com/minio/mc/releases/download/$MC_RELEASE/mc.linux-$TARGETARCH.$MC_RELEASE" \
    && echo "$sum  /usr/local/bin/mc" | sha256sum -c - \
    && chmod 0755 /usr/local/bin/mc

WORKDIR /scripts
COPY . .

ENTRYPOINT ["/scripts/entrypoint.sh"]
