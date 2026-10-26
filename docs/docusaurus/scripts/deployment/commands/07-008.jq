# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
def without(names): map(select(.name as $n | names | index($n) | not));
"-Djdk.tls.client.protocols=TLSv1.3"
  + " -Djdk.tls.client.cipherSuites=TLS_AES_256_GCM_SHA384"
  + " -Djdk.tls.namedGroups=secp384r1" as $tls
| .spec.template.spec.containers |= map(if .name != "keycloak" then . else
    .args = ["start", "--import-realm"]
    | .env = (.env | without(["KC_HOSTNAME_STRICT", "KC_HOSTNAME_STRICT_HTTPS"]))
      + [{name: "KC_HOSTNAME", value: "https://\($ENV.PORTAL_HOST)"},
         {name: "KC_HTTP_ENABLED", value: "true"},
         {name: "KC_TRUSTSTORE_PATHS",
          value: "/opt/keycloak/conf/truststores/ca.pem"},
         {name: "JAVA_OPTS_APPEND", value: $tls}]
    | .envFrom = (.envFrom // []) + [{secretRef: {name: "keycloak-bootstrap-admin"}}]
    | .volumeMounts += [{name: "auth-server-ca", readOnly: true,
        mountPath: "/opt/keycloak/conf/truststores"}]
  end)
| .spec.template.spec.volumes |= map(if .name == "realm-import"
    then {name, secret: {secretName: "keycloak-realm"}} else . end)
  + [{name: "auth-server-ca", configMap: {name: "auth-server-ca"}}]
