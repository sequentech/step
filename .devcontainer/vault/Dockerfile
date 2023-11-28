FROM hashicorp/vault:latest

RUN mkdir -p /opt/vault
COPY config.hcl /opt/vault
COPY volume.tar.gz /opt/vault
COPY entrypoint.sh /opt/vault
ENTRYPOINT ["/opt/vault/entrypoint.sh"]