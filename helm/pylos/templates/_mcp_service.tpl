{{- define "mcp.service" -}}
apiVersion: v1
kind: Service
metadata:
  name: {{ include "pylos.mcp.name" .context }}
  namespace: {{ $.context.Values.namespace }}
  labels:
    {{- include "pylos.mcp.labels" .context | nindent 4 }}
spec:
  type: ClusterIP
  ports:
    - port: {{ $.context.Values.mcp.service.port }}
      targetPort: {{ $.context.Values.mcp.service.port }}
      protocol: TCP
      name: http
  selector:
    {{- include "pylos.mcp.labels" .context | nindent 4 }}
---
{{- end -}}
