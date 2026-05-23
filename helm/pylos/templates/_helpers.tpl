{{- define "pylos.name" -}}
{{ .Values.backend.name }}
{{- end -}}

{{- define "pylos.labels" -}}
app: {{ include "pylos.name" . }}
{{- end -}}

{{- define "pylos.ui.name" -}}
{{ .Values.ui.name }}
{{- end -}}

{{- define "pylos.ui.labels" -}}
app: {{ include "pylos.ui.name" . }}
{{- end -}}

{{- define "pylos.dev.name" -}}
{{ .Values.dev.name }}
{{- end -}}

{{- define "pylos.dev.labels" -}}
app: {{ include "pylos.dev.name" . }}
{{- end -}}
