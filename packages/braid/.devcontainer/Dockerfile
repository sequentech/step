# SPDX-FileCopyrightText: 2022 X-Truder <jaka@x-truder.net>
# SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
#
# SPDX-License-Identifier: MIT

# Reuse a base image made for devcontainers.
FROM mcr.microsoft.com/vscode/devcontainers/base:bullseye

# update and install base packages
RUN apt update -y
RUN apt -y install --no-install-recommends \
    sudo \
    ca-certificates \
    psmisc \
    procps \
    less \
    xz-utils \
    vim-tiny \
    nano \
    curl \
    clang \
    git \
    ssh \
    direnv \
    gnupg2 \
    iproute2 \
    inetutils-ping \
    rsync \
    lsb-release \
    dialog \
    locales \
    man-db \
    bash-completion

# create at least locae for en_US.UTF-8
RUN echo "en_US.UTF-8 UTF-8" >> /etc/locale.gen && locale-gen

# create non-root user and group and add it sudoers
ARG USERNAME=vscode
RUN echo $USERNAME ALL=\(root\) NOPASSWD:ALL > /etc/sudoers.d/devcontainer && \
    chmod 0440 /etc/sudoers.d/devcontainer

# copy nix configuration
COPY etc/nix.conf /etc/nix/nix.conf

# install nix
ARG NIX_INSTALL_SCRIPT=https://nixos.org/nix/install
RUN curl -L ${NIX_INSTALL_SCRIPT} | sudo -u ${USERNAME} NIX_INSTALLER_NO_MODIFY_PROFILE=1 sh

# install devcontainer extra profile that loads nix and has vscode user env probe
COPY etc/devcontainer.sh /etc/profile.d/devcontainer.sh

# install bash config
COPY etc/bash.bashrc /etc/bash.bashrc

# set env for non interactve shell to load nix
COPY etc/envrc /etc/envrc
ENV ENV="/etc/envrc" BASH_ENV="/etc/envrc"

# copy direnv config and set conig path
COPY etc/direnv.toml /etc
ENV DIRENV_CONFIG=/etc

USER ${USERNAME}

# Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

# cache /nix
VOLUME /nix