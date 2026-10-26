# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
cat > /root/deploy/storage-check.yaml <<'EOF'
apiVersion: v1
kind: Pod
metadata:
  name: storage-check
  namespace: step-infra
spec:
  restartPolicy: Never
  containers:
    - name: check
      image: sequentech.local/trustee:latest
      imagePullPolicy: Never
      command: ["/bin/sh", "-c"]
      args:
        - |
          aws s3api list-objects-v2 --bucket public --prefix plugins/ \
            --query 'length(Contents || `[]`)'
          aws --no-sign-request s3api list-objects-v2 \
            --bucket election-event-documents 2>&1 | grep -o AccessDenied
          aws s3api get-bucket-policy --bucket election-event-documents 2>&1 \
            | grep -o NoSuchBucketPolicy
          p='{"Version":"2012-10-17","Statement":[{"Effect":"Allow",'
          p="$p"'"Principal":{"AWS":["*"]},"Action":["s3:GetObject"],'
          p="$p"'"Resource":["arn:aws:s3:::public/*"]}]}'
          aws s3api get-bucket-policy --bucket public --query Policy \
            --output text | tr -d ' \n' | grep -cxF "$p"
          aws s3api get-object-lock-configuration \
            --bucket electoral-log-checkpoints \
            --query ObjectLockConfiguration.ObjectLockEnabled --output text
      env:
        - name: AWS_ENDPOINT_URL
          value: "http://rustfs.step-infra.svc.cluster.local:9000"
        - name: AWS_REGION
          value: "us-east-1"
        - name: AWS_ACCESS_KEY_ID
          value: "admin"
        - name: AWS_SECRET_ACCESS_KEY
          valueFrom:
            secretKeyRef:
              name: step-secrets
              key: RUSTFS_ROOT_PASSWORD
EOF
k3s kubectl apply -f /root/deploy/storage-check.yaml
k3s kubectl -n step-infra wait pod/storage-check --timeout=2m \
  --for=jsonpath='{.status.phase}'=Succeeded
k3s kubectl -n step-infra logs storage-check
k3s kubectl -n step-infra delete pod storage-check
