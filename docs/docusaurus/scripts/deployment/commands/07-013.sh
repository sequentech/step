# SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
# SPDX-License-Identifier: AGPL-3.0-only
cat > harvest/service.yaml <<'EOF'
apiVersion: v1
kind: Service
metadata:
  name: harvest
  namespace: step-apps
spec:
  selector:
    app: harvest
  ports:
    - port: 8400
      targetPort: 8400
EOF
