---
title: "gRPC API"
sidebar_position: 0
---

# gRPC API
<a name="top"></a>

{{range .Files}}
{{$file_name := .Name}}
<a name="{{.Name | anchor}}"></a>

## {{.Name}}
{{.Description}}

{{range .Messages}}
<a name="{{.FullName | anchor}}"></a>

### {{.LongName}}
{{.Description}}

{{if .HasFields}}
| Field | Type | Label | Description |
| ----- | ---- | ----- | ----------- |
{{range .Fields -}}
  | {{.Name}} | [{{.LongType}}](#{{.FullType | anchor}}) | {{.Label}} | {{if (index .Options "deprecated"|default false)}}**Deprecated.** {{end}}{{nobr .Description}}{{if .DefaultValue}} Default: {{.DefaultValue}}{{end}} |
{{end}}
{{end}}

{{if .HasExtensions}}
| Extension | Type | Base | Number | Description |
| --------- | ---- | ---- | ------ | ----------- |
{{range .Extensions -}}
  | {{.Name}} | {{.LongType}} | {{.ContainingLongType}} | {{.Number}} | {{nobr .Description}}{{if .DefaultValue}} Default: {{.DefaultValue}}{{end}} |
{{end}}
{{end}}

{{end}}

{{range .Enums}}
<a name="{{.FullName | anchor}}"></a>

### {{.LongName}}
{{.Description}}

| Name | Number | Description |
| ---- | ------ | ----------- |
{{range .Values -}}
  | {{.Name}} | {{.Number}} | {{nobr .Description}} |
{{end}}

{{end}}

{{if .HasExtensions}}
<a name="{{$file_name | anchor}}-extensions"></a>

### File-level Extensions
| Extension | Type | Base | Number | Description |
| --------- | ---- | ---- | ------ | ----------- |
{{range .Extensions -}}
  | {{.Name}} | {{.LongType}} | {{.ContainingLongType}} | {{.Number}} | {{nobr .Description}}{{if .DefaultValue}} Default: `{{.DefaultValue}}`{{end}} |
{{end}}
{{end}}

{{range .Services}}
<a name="{{.FullName | anchor}}"></a>

### {{.Name}}
{{.Description}}

| Method Name | Request Type | Response Type | Description |
| ----------- | ------------ | ------------- | ------------|
{{range .Methods -}}
  | {{.Name}} | [{{.RequestLongType}}](#{{.RequestFullType | anchor}}){{if .RequestStreaming}} stream{{end}} | [{{.ResponseLongType}}](#{{.ResponseFullType | anchor}}){{if .ResponseStreaming}} stream{{end}} | {{nobr .Description}} |
{{end}}
{{end}}

{{end}}

## Scalar Value Types

| .proto Type | Notes | C++ | Java | Python | Go | C# | PHP | Ruby |
| ----------- | ----- | --- | ---- | ------ | -- | -- | --- | ---- |
{{range .Scalars -}}
  | <a name="{{.ProtoType | anchor}}" /> {{.ProtoType}} | {{.Notes}} | {{.CppType}} | {{.JavaType}} | {{.PythonType}} | {{.GoType}} | {{.CSharp}} | {{.PhpType}} | {{.RubyType}} |
{{end}}