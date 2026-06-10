{{- define "pylos.mcp.name" -}}
{{ .Values.mcp.name }}-{{ .name }}
{{- end -}}

{{- define "pylos.mcp.labels" -}}
app: {{ include "pylos.mcp.name" . }}
type: mcp-server
{{- end -}}
