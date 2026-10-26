# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
cd /root/deploy/kubernetes
sed -i 's|^  AWS_S3_PUBLIC_URI:.*|  AWS_S3_PUBLIC_URI: "https://'"$PORTAL_HOST"'"|' config/step-env.yaml
# REUSE-IgnoreStart
cat > rustfs/browser-ingress.yaml <<EOF
# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: LicenseRef-Proprietary
# Presigned S3 URLs include the bucket path; never strip or rewrite it.
apiVersion: traefik.io/v1alpha1
kind: Middleware
metadata:
  name: storage-browser-cors
  namespace: step-infra
spec:
  headers:
    accessControlAllowOriginList:
      - "https://$PORTAL_HOST"
      - "https://$ADMIN_HOST"
      - "https://$VERIFIER_HOST"
    accessControlAllowMethods:
      - GET
      - HEAD
      - PUT
      - OPTIONS
    accessControlAllowHeaders:
      - "*"
    accessControlExposeHeaders:
      - ETag
    addVaryHeader: true
---
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: storage-browser-ingress
  namespace: step-infra
  annotations:
    traefik.ingress.kubernetes.io/router.entrypoints: websecure
    traefik.ingress.kubernetes.io/router.middlewares: step-infra-storage-browser-cors@kubernetescrd
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
      http: &browser-storage
        paths:
          - path: /public
            pathType: Prefix
            backend:
              service:
                name: rustfs
                port:
                  number: 9000
          - path: /election-event-documents
            pathType: Prefix
            backend:
              service:
                name: rustfs
                port:
                  number: 9000
    - host: $ADMIN_HOST
      http: *browser-storage
    - host: $VERIFIER_HOST
      http: *browser-storage
EOF
# REUSE-IgnoreEnd
grep '^  AWS_S3_PUBLIC_URI:' config/step-env.yaml
grep -A3 'accessControlAllowOriginList:' rustfs/browser-ingress.yaml
