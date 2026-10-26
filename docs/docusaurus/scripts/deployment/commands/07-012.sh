# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
cat > keycloak/master-blocked.yaml <<EOF
apiVersion: traefik.io/v1alpha1
kind: Middleware
metadata:
  name: deny-all
  namespace: step-infra
spec:
  ipAllowList:
    sourceRange:
      - 192.0.2.1/32
---
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: keycloak-master-blocked
  namespace: step-infra
  annotations:
    traefik.ingress.kubernetes.io/router.entrypoints: websecure
    traefik.ingress.kubernetes.io/router.middlewares: step-infra-deny-all@kubernetescrd
spec:
  ingressClassName: traefik
  tls:
    - hosts:
        - $PORTAL_HOST
        - $ADMIN_HOST
        - $VERIFIER_HOST
      secretName: step-tls-cert
  rules:
    - host: $PORTAL_HOST
      http: &master-realm
        paths:
          - path: /realms/master
            pathType: Prefix
            backend:
              service:
                name: keycloak
                port:
                  number: 8090
    - host: $ADMIN_HOST
      http: *master-realm
    - host: $VERIFIER_HOST
      http: *master-realm
EOF
grep -E '^    - host:' keycloak/master-blocked.yaml
