{{- define "mcp.networkpolicy" -}}
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: {{ include "pylos.mcp.name" .context }}-network
  namespace: {{ $.context.Values.namespace }}
spec:
  podSelector:
    matchLabels:
      {{- include "pylos.mcp.labels" .context | nindent 6 }}
  policyTypes:
    - Ingress
    - Egress
  ingress:
    - from:
        - podSelector:
            matchLabels:
              app: {{ $.context.Values.backend.name }}
      ports:
        - port: {{ $.context.Values.mcp.service.port }}
          protocol: TCP
  egress:
    - to:
        - podSelector:
            matchLabels:
              app: {{ $.context.Values.backend.name }}
      ports:
        - port: {{ $.context.Values.mcp.service.port }}
          protocol: TCP
    - to:
        - namespaceSelector: {}
          podSelector:
            matchLabels:
              k8s-app: kube-dns
      ports:
        - port: 53
          protocol: UDP
---
{{- end -}}
