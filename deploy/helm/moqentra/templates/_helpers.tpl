{{- define "moqentra.fullname" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" -}}
{{- end -}}

{{- define "moqentra.labels" -}}
app.kubernetes.io/name: {{ include "moqentra.fullname" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
app.kubernetes.io/version: {{ .Chart.AppVersion | quote }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
{{- end -}}

{{- define "moqentra.requireProduction" -}}
{{- if .Values.productionMode -}}
  {{- if not .Values.external.db -}}
    {{- fail "productionMode requires external.db=true (managed PostgreSQL)" -}}
  {{- end -}}
  {{- if not .Values.external.s3 -}}
    {{- fail "productionMode requires external.s3=true" -}}
  {{- end -}}
  {{- if not .Values.external.oidc -}}
    {{- fail "productionMode requires external.oidc=true" -}}
  {{- end -}}
  {{- if not .Values.networkPolicy.enabled -}}
    {{- fail "productionMode requires networkPolicy.enabled=true" -}}
  {{- end -}}
  {{- if not .Values.tls.enabled -}}
    {{- fail "productionMode requires tls.enabled=true" -}}
  {{- end -}}
{{- end -}}
{{- end -}}
