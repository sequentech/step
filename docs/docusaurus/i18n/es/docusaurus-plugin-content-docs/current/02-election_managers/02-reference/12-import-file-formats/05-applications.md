---
id: import_format_applications
title: Archivo de solicitudes
sidebar_position: 5
description: "El formato del archivo de solicitudes: las doce columnas, el JSON de applicant_data, los estados permitidos y lo que hace la importación."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

El archivo de solicitudes contiene las solicitudes de los votantes que se inscribieron a través del formulario de inscripción del proceso electoral. Usted crea el archivo con **Exportar** en la pestaña **Aprobaciones**. Lo importa con **Importar** en la misma pestaña. No cree el archivo a mano.

Use la importación para restaurar las solicitudes que exportó antes, o para copiarlas a un proceso electoral nuevo. Para ver la pantalla, consulte [Aprobaciones](../02-election-event/14-election_management_election-event_approvals.md#import-file).

:::caution PRECAUCIÓN: inquilino y proceso electoral en el archivo de solicitudes
La importación lee el ID del inquilino y el ID del proceso electoral de cada fila. No usa el proceso electoral que tiene abierto. Importe el archivo solo en el proceso electoral donde lo exportó. Si lo importa de nuevo, obtendrá una segunda copia de cada solicitud.
:::

## Campos {#fields}

El archivo tiene una fila de encabezado y doce columnas. El orden de las columnas es fijo. La importación lee las columnas por posición, no por nombre.

| Columna | Posición | Obligatoria | Valores permitidos o formato | Valor predeterminado | Significado | Ejemplo |
| --- | --- | --- | --- | --- | --- | --- |
| `id` | 1 | Opcional | Un UUID | Ninguno | El ID de la solicitud. La importación no lo usa. Da a cada solicitud un ID nuevo. | `4f1c0b0e-0000-4000-8000-000000000001` |
| `created_at` | 2 | Opcional | Fecha y hora con desfase de zona horaria | La primera fecha del sistema (1970) | Cuándo se hizo la solicitud. | `2026-10-01 09:30:00 +00:00` |
| `updated_at` | 3 | Opcional | Igual que `created_at` | La primera fecha del sistema (1970) | Cuándo cambió la solicitud. | `2026-10-02 10:00:00 +00:00` |
| `tenant_id` | 4 | Obligatoria | Un UUID versión 4 | Ninguno | El inquilino de la solicitud. Una fila con un valor no válido se omite. | `90505c8a-23a9-4cdf-a26b-4e19f6a097d5` |
| `election_event_id` | 5 | Obligatoria | Un UUID versión 4 que existe | Ninguno | El proceso electoral de la solicitud. | `c2a1d6f0-0000-4000-8000-000000000002` |
| `area_id` | 6 | Obligatoria, vea las notas | Un UUID versión 4 | Ninguno | El área de la solicitud. | `7d3e5a10-0000-4000-8000-000000000003` |
| `applicant_id` | 7 | Obligatoria | Texto | Ninguno | El ID que el formulario de inscripción dio al solicitante. | `applicant-001` |
| `applicant_data` | 8 | Obligatoria | Un objeto JSON | `null` | Los datos que el solicitante escribió en el formulario. | Vea más abajo. |
| `labels` | 9 | Opcional | Texto (JSON) | Vacío | Etiquetas que el formulario envió con la solicitud. | `{}` |
| `annotations` | 10 | Opcional | Texto (JSON) | Vacío | Los datos que la plataforma añade durante la verificación. | Vea más abajo. |
| `verification_type` | 11 | Obligatoria | `AUTOMATIC` o `MANUAL` | Ninguno | Quién decidió: la plataforma (`AUTOMATIC`) o un administrador (`MANUAL`). | `AUTOMATIC` |
| `status` | 12 | Obligatoria | `PENDING`, `ACCEPTED` o `REJECTED` | Ninguno | El estado de la solicitud. | `ACCEPTED` |

Notas:

- El formato de fecha es el que escribe la exportación: `YYYY-MM-DD HH:MM:SS`, seguido de un espacio y del desfase, por ejemplo `2026-10-01 09:30:00 +00:00`. Si no se puede leer la fecha, la importación usa la primera fecha del sistema y no muestra ningún error.
- La exportación escribe un `area_id` vacío para una solicitud sin área. Una fila con `area_id` vacío detiene la importación con un error que empieza por `invalid UUID ''`.
- La importación no compara los valores de `verification_type` y `status` con la lista. El portal de administración solo muestra los valores de la tabla. Use solo estos valores.
- La exportación no escribe la etiqueta de permiso de una solicitud. Después de una importación, la etiqueta de permiso queda vacía.

### La columna `applicant_data` {#the-column-applicant_data}

`applicant_data` es un objeto JSON. Cada clave es el nombre de un campo del formulario de inscripción. Cada valor es el texto que escribió el solicitante. Las claves dependen del perfil de usuario del proceso electoral. El formulario puede enviar el nombre, los apellidos, la dirección de correo electrónico, la fecha de nacimiento y cualquier atributo del perfil de usuario.

```json
{"firstName":"Ana","lastName":"Lopez","email":"voter001@example.com","dateOfBirth":"1990-01-31","username":"voter001"}
```

- La plataforma añade la clave `username` cuando guarda la solicitud. Su valor es el nombre de usuario del votante que coincidió, o un texto vacío si no coincidió ningún votante.
- La lista de **Aprobaciones** muestra una columna para cada atributo del perfil de usuario. Lee el valor de la clave con el mismo nombre en `applicant_data`.
- Si el JSON no es válido, la importación guarda `null` y no muestra ningún error.

### La columna `annotations` {#the-column-annotations}

La verificación guarda aquí sus datos de trabajo. Estas son las claves que puede encontrar:

| Clave | Significado |
| --- | --- |
| `session_id`, `credentials` | Datos de la sesión del formulario. |
| `search-attributes` | Los atributos que la plataforma usa para buscar un votante. Separados por comas. |
| `unset-attributes` | Los atributos del votante que deben estar vacíos antes de que la plataforma acepte la solicitud. |
| `update-attributes` | Los atributos del votante que se deben actualizar, separados por comas. Cuando un administrador confirma la solicitud, la plataforma copia estas claves de `applicant_data` al votante. `email`, `firstName` y `lastName` establecen los campos propios del votante. |
| `fields_match` | Para cada campo buscado: `true` si la solicitud y el votante tienen el mismo valor. |
| `mismatches` | El número de campos que no coinciden. |
| `manual_verify_reason` | El motivo por el que un administrador debe revisar la solicitud. |
| `rejection_reason` | `insufficient-information`, `no-matching-voter`, `voter-already-approved` u `other`. |
| `rejection_message` | El texto del rechazo. Obligatorio para el motivo `other`. |
| `verified_by` | El administrador que decidió. |

## Reglas {#rules}

| Regla | Valor |
| --- | --- |
| Tipo de archivo | `.csv`. El separador es una coma. |
| Encabezado | La exportación escribe una fila de encabezado. La importación no la salta. La omite porque el valor `tenant_id` no es un UUID válido. Un archivo sin fila de encabezado también funciona. |
| Codificación | UTF-8. |
| Comillas | La exportación pone las columnas JSON entre comillas dobles y duplica cada comilla que hay dentro. No cambie esto. |
| Orden de las columnas | Fijo. No mueva ni elimine una columna. |
| Tamaño | El tamaño está limitado por la configuración de subida de archivos de la instalación. |
| Filtro de proceso electoral | Si exporta desde la pestaña **Aprobaciones** de una elección, el archivo tiene solo las solicitudes de esa elección. |

## Archivo de ejemplo {#example-file}

```csv
id,created_at,updated_at,tenant_id,election_event_id,area_id,applicant_id,applicant_data,labels,annotations,verification_type,status
4f1c0b0e-0000-4000-8000-000000000001,2026-10-01 09:30:00 +00:00,2026-10-01 09:30:05 +00:00,90505c8a-23a9-4cdf-a26b-4e19f6a097d5,c2a1d6f0-0000-4000-8000-000000000002,7d3e5a10-0000-4000-8000-000000000003,applicant-001,"{""firstName"":""Ana"",""lastName"":""Lopez"",""email"":""voter001@example.com"",""username"":""voter001""}","{}","{""search-attributes"":""firstName,lastName"",""mismatches"":0}",AUTOMATIC,ACCEPTED
```

## Qué hace la importación {#what-the-import-does}

1. El portal de administración sube el archivo y pide la **Verificación de Integridad (SHA 256)**. La tarea comprueba el hash si lo escribió.
2. La tarea empieza. Mire la pestaña **Tareas** para ver el resultado.
3. Para cada fila, la tarea lee las 12 columnas. Omite una fila si `tenant_id` no es un UUID válido.
4. La tarea añade todas las demás filas en una única transacción de la base de datos. Si una fila es incorrecta, la tarea no añade nada y el estado es **Failed**.
5. Cada solicitud recibe un ID nuevo. Las fechas, los datos, el estado y el tipo se mantienen como están en el archivo.
6. La importación no envía mensajes a los solicitantes. No crea ni cambia votantes.

Efecto conocido: la importación guarda `labels` y `annotations` como un texto JSON y no como un objeto JSON.

### Mensajes y errores {#messages-and-errors}

| Mensaje | Dónde aparece | Significado y acción |
| --- | --- | --- |
| Solicitudes importadas correctamente | Notificación | La tarea ha terminado. |
| Error al importar las solicitudes | Notificación | El portal no pudo iniciar la tarea. Inténtelo de nuevo. |
| `Failed to verify the integrity: Hash of voters file: ... does not match with the input hash: ...` | Registro de la tarea | El hash que escribió no es el hash del archivo. (El mensaje dice "voters file" también para este archivo.) |
| `Error reading CSV record` | Registro de la tarea | Una fila tiene un número incorrecto de columnas o comillas incorrectas. |
| `Error inserting application: ...` | Registro de la tarea | La base de datos rechazó una fila. Revise `election_event_id` y `area_id`. |
| `document not found` | Registro de la tarea | La subida falló. Suba el archivo de nuevo. |

## Páginas relacionadas {#related-pages}

- [Aprobaciones](../02-election-event/14-election_management_election-event_approvals.md): la pestaña **Aprobaciones**, **Importar** y **Exportar**.
- [Archivo de votantes](03-voters.md): los votantes con los que coinciden las solicitudes.
- [Formatos de archivos de importación](index.md): todos los archivos de importación.
