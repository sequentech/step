# File is based on the example found at
# https://www.keycloak.org/server/containers#_provide_initial_admin_credentials_when_running_in_a_container
#
# Newer versions may be available at 
# https://quay.io/repository/keycloak/keycloak?tab=tags

FROM registry.access.redhat.com/ubi9 AS ubi-micro-build
RUN mkdir -p /mnt/rootfs
RUN dnf install --installroot /mnt/rootfs curl --releasever 9 --setopt install_weak_deps=false --nodocs -y; dnf --installroot /mnt/rootfs clean all
RUN echo $(ls -lah /mnt/rootfs/bin/curl)
RUN echo $(ls /mnt/rootfs/usr/bin/curl)

FROM quay.io/keycloak/keycloak:22.0 as builder
COPY --from=ubi-micro-build /mnt/rootfs /
COPY --from=ubi-micro-build /mnt/rootfs/usr/bin/curl /usr/bin/curl
COPY --from=ubi-micro-build /mnt/rootfs/bin/curl /bin/curl
RUN echo $(ls -lah /usr/bin/curl)
RUN echo $(ls -lah /bin/curl)

# Enable health and metrics support
ENV KC_HEALTH_ENABLED=true
ENV KC_METRICS_ENABLED=true

# Configure a database vendor
ENV KC_DB=postgres

WORKDIR /opt/keycloak

# for demonstration purposes only, please make sure to use proper certificates
# in production instead
RUN keytool \
   -genkeypair \
   -storepass password \
   -storetype PKCS12 \
   -keyalg RSA \
   -keysize 2048 \
   -dname "CN=server" \
   -alias keycloak-server -ext "SAN:c=DNS:localhost,IP:127.0.0.1" \
   -keystore conf/server.keystore
RUN /opt/keycloak/bin/kc.sh build

FROM quay.io/keycloak/keycloak:22.0
COPY --from=builder /opt/keycloak/ /opt/keycloak/
COPY --from=builder /bin/curl /bin/curl
COPY --from=builder /usr/bin/curl /usr/bin/curl

ENV KC_DB=postgres
ENTRYPOINT ["/opt/keycloak/bin/kc.sh"]
