{{- define "mcp.deployment" -}}
apiVersion: apps/v1
kind: Deployment
metadata:
  name: {{ include "pylos.mcp.name" .context }}
  namespace: {{ $.context.Values.namespace }}
  labels:
    {{- include "pylos.mcp.labels" .context | nindent 4 }}
spec:
  replicas: 1
  strategy:
    type: Recreate
  selector:
    matchLabels:
      {{- include "pylos.mcp.labels" .context | nindent 6 }}
  template:
    metadata:
      labels:
        {{- include "pylos.mcp.labels" .context | nindent 8 }}
    spec:
      securityContext:
        runAsNonRoot: true
      containers:
        - name: mcp-server
          securityContext:
            runAsUser: 1001
            runAsGroup: 1001
            allowPrivilegeEscalation: false
            capabilities:
              drop: ["ALL"]
            readOnlyRootFilesystem: true
          image: "{{ $.context.Values.mcp.image.repository }}:{{ $.context.Values.mcp.image.tag }}"
          imagePullPolicy: {{ $.context.Values.mcp.image.pullPolicy }}
          ports:
            - containerPort: {{ $.context.Values.mcp.service.port }}
              protocol: TCP
          env:
            - name: MCP_SERVER_NAME
              value: {{ .server.name }}
            - name: MCP_SERVER_TYPE
              value: {{ .server.server_type }}
            - name: MCP_TARGET_URL
              value: {{ .server.target_url | default "" }}
            {{- range $key, $value := .server.env_vars }}
            - name: {{ $key }}
              value: {{ $value | quote }}
            {{- end }}
          resources:
            requests:
              memory: {{ $.context.Values.mcp.resources.requests.memory }}
              cpu: {{ $.context.Values.mcp.resources.requests.cpu }}
            limits:
              memory: {{ $.context.Values.mcp.resources.limits.memory }}
              cpu: {{ $.context.Values.mcp.resources.limits.cpu }}
          livenessProbe:
            httpGet:
              path: /health
              port: {{ $.context.Values.mcp.service.port }}
            initialDelaySeconds: 5
            periodSeconds: 10
          readinessProbe:
            httpGet:
              path: /health
              port: {{ $.context.Values.mcp.service.port }}
            initialDelaySeconds: 3
            periodSeconds: 5
      affinity:
        podAntiAffinity:
          preferredDuringSchedulingIgnoredDuringExecution:
            - weight: 100
              podAffinityTerm:
                labelSelector:
                  matchExpressions:
                    - key: type
                      operator: In
                      values:
                        - mcp-server
                topologyKey: kubernetes.io/hostname
---
{{- end -}}
