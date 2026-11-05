# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: AGPL-3.0-only

# The monitoring renderer: internal only, non-root, runs on a read-only root
# filesystem with a tmpfs at /tmp. Build from this directory:
#   docker build -t monitoring-renderer packages/monitoring-renderer
ARG PYTHON_IMAGE=python:3.12-slim@sha256:f77ac9e44ae96ef2c90b8053ea08c31f8be030f824196b0ae4db6d462c84e51f

FROM ${PYTHON_IMAGE} AS build
ENV PIP_NO_CACHE_DIR=1 PIP_DISABLE_PIP_VERSION_CHECK=1
RUN python -m venv /opt/venv
COPY requirements.txt /tmp/requirements.txt
# Every dependency is pinned with its hashes; nothing else is installed.
RUN /opt/venv/bin/pip install --no-deps --require-hashes -r /tmp/requirements.txt \
    && /opt/venv/bin/pip check

FROM ${PYTHON_IMAGE}
RUN groupadd --system --gid 10001 renderer \
    && useradd --system --uid 10001 --gid renderer --home-dir /nonexistent \
       --no-create-home --shell /usr/sbin/nologin renderer
COPY --from=build /opt/venv /opt/venv
COPY monitoring_renderer /app/monitoring_renderer
WORKDIR /app
ENV PATH=/opt/venv/bin:$PATH \
    PYTHONDONTWRITEBYTECODE=1 \
    PYTHONUNBUFFERED=1 \
    HOME=/tmp \
    TMPDIR=/tmp \
    XDG_CACHE_HOME=/tmp/.cache \
    DO_NOT_TRACK=1 \
    DBT_SEND_ANONYMOUS_USAGE_STATS=false \
    RENDERER_HOST=0.0.0.0 \
    RENDERER_PORT=8080
# Compiled now: the root filesystem is read-only at run time.
RUN python -m compileall -q /app /opt/venv/lib
USER 10001:10001
EXPOSE 8080
HEALTHCHECK --interval=10s --timeout=5s --start-period=30s --retries=3 \
    CMD ["python", "-c", "import sys, urllib.request; sys.exit(0 if urllib.request.urlopen('http://127.0.0.1:8080/ready', timeout=4).status == 200 else 1)"]
CMD ["python", "-m", "monitoring_renderer"]
