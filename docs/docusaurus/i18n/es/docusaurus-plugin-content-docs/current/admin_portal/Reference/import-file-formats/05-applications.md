---
id: import_format_applications
title: Archivo de aplicaciones
sidebar_position: 5
description: "El formato del archivo de aplicaciones: las doce columnas, el JSON de applicant_data, los estados permitidos y qué hace la importación."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

El archivo de aplicaciones contiene las aplicaciones de los votantes que se inscribieron mediante
el formulario de inscripción del proceso electoral. Usted crea el archivo con **Exportar** en la
pestaña **Aprobaciones**. Lo importa con **Importar** en la misma pestaña. No cree el archivo a
mano.

Use la importación para restaurar las aplicaciones que exportó antes, o para copiarlas a un nuevo
proceso electoral. Para ver la pantalla, consulte
[Aprobaciones](../User-Manual/Election-Management/Election-Event/Approvals/election_management_election-event_approvals.md#import-file).

:::caution PRECAUCIÓN: inquilino y proceso electoral en el archivo de aplicaciones
La importación lee el ID del inquilino y el ID del proceso electoral de cada fila. No usa el
proceso electoral que tiene abierto. Importe el archivo solo en el proceso electoral donde lo
exportó. Si lo importa de nuevo, obtendrá una segunda copia de cada aplicación.
:::

## Campos {#fields}

El archivo tiene una fila de encabezado y doce columnas. El orden de las columnas es fijo. La
importación lee las columnas por posición, no por nombre.

| Columna | Posición | Obligatorio | Valores permitidos o formato | Predeterminado | Significado | Ejemplo |
| --- | --- | --- | --- | --- | --- | --- |
| `id` | 1 | Opcional | Un UUID | Ninguno | El ID de la aplicación. La importación no lo usa. Asigna un nuevo ID a cada aplicación. | `4f1c0b0e-0000-4000-8000-000000000001` |
| `created_at` | 2 | Opcional | Fecha y hora con desfase de zona horaria | La primera fecha del sistema (1970) | Cuándo se creó la aplicación. | `2026-10-01 09:30:00 +00:00` |
| `updated_at` | 3 | Opcional | Igual que `created_at` | La primera fecha del sistema (1970) | Cuándo cambió la aplicación. | `2026-10-02 10:00:00 +00:00` |
| `tenant_id` | 4 | Obligatorio | Un UUID versión 4 | Ninguno | El inquilino de la aplicación. Se omite una fila con un valor que no es válido. | `90505c8a-23a9-4cdf-a26b-4e19f6a097d5` |
| `election_event_id` | 5 | Obligatorio | Un UUID versión 4 | Ninguno | El proceso electoral de la aplicación. Un valor que no es un UUID versión 4 detiene la importación. | `c2a1d6f0-0000-4000-8000-000000000002` |
| `area_id` | 6 | Obligatorio, vea las notas | Un UUID versión 4 | Ninguno | El área de la aplicación. | `7d3e5a10-0000-4000-8000-000000000003` |
| `applicant_id` | 7 | Opcional | Texto | Vacío | El ID que el formulario de inscripción dio al solicitante. | `applicant-001` |
| `applicant_data` | 8 | Obligatorio | Un objeto JSON | `null` | Los datos que el solicitante escribió en el formulario. | Vea abajo. |
| `labels` | 9 | Opcional | Texto (JSON) | Vacío | Etiquetas que el formulario envió con la aplicación. | `{}` |
| `annotations` | 10 | Opcional | Texto (JSON) | Vacío | Los datos que la plataforma añade durante la verificación. | Vea abajo. |
| `verification_type` | 11 | Obligatorio | `AUTOMATIC` o `MANUAL` | Ninguno | Quién decidió: la plataforma (`AUTOMATIC`) o un administrador (`MANUAL`). | `AUTOMATIC` |
| `status` | 12 | Obligatorio | `PENDING`, `ACCEPTED` o `REJECTED` | Ninguno | El estado de la aplicación. | `ACCEPTED` |

Notas:

- El formato de fecha es el que escribe la exportación: `YYYY-MM-DD HH:MM:SS`, seguido de un espacio
  y el desfase, por ejemplo `2026-10-01 09:30:00 +00:00`. Si no se puede leer la fecha, la
  importación usa la primera fecha del sistema y no muestra ningún error. La exportación también
  puede escribir fracciones de segundo, por ejemplo `09:30:00.250`. La importación las lee.
- La exportación escribe un `area_id` vacío para una aplicación sin área. Una fila con un
  `area_id` vacío, o con un `area_id` o `election_event_id` que no es un UUID versión 4, detiene la
  importación con un error que empieza por `invalid UUID`.
- La importación no comprueba los valores de `verification_type` y `status` contra la lista. El
  portal de administración solo muestra los valores de la tabla. Use solo estos valores.
- La exportación no escribe la etiqueta de permiso de una aplicación. Después de una importación,
  la etiqueta de permiso queda vacía.

### La columna `applicant_data` {#the-column-applicant_data}

`applicant_data` es un objeto JSON. Cada clave es el nombre de un campo del formulario de
inscripción. Cada valor es el texto que escribió el solicitante. Las claves dependen del perfil de
usuario del proceso electoral. El formulario puede enviar el nombre, los apellidos, la dirección de
correo electrónico, la fecha de nacimiento y cualquier atributo del perfil de usuario.

```json
{"firstName":"Ana","lastName":"Lopez","email":"voter001@example.com","dateOfBirth":"1990-01-31","username":"voter001"}
```

- La plataforma añade la clave `username` al guardar la aplicación. Su valor es el nombre de usuario
  del votante que coincidió, o un texto vacío si no coincidió ningún votante.
- La lista de **Aprobaciones** muestra una columna por cada atributo del perfil de usuario. Lee el
  valor de la clave con el mismo nombre en `applicant_data`.
- Si el JSON no es válido, la importación guarda `null` y no muestra ningún error.

### La columna `annotations` {#the-column-annotations}

La verificación guarda aquí sus datos de trabajo. Claves que puede encontrar:

| Clave | Significado |
| --- | --- |
| `session_id`, `credentials` | Datos de la sesión del formulario. |
| `search-attributes` | Los atributos que la plataforma usa para buscar un votante. Separados por comas. |
| `unset-attributes` | Los atributos del votante que deben estar vacíos antes de que la plataforma acepte la aplicación. |
| `update-attributes` | Los atributos del votante que se deben actualizar, separados por comas. Cuando un administrador confirma la aplicación, la plataforma copia estas claves de `applicant_data` al votante. `email`, `firstName` y `lastName` establecen los campos propios del votante. |
| `fields_match` | Para cada campo buscado: `true` si la aplicación y el votante tienen el mismo valor. |
| `mismatches` | El número de campos que difieren. |
| `manual_verify_reason` | El motivo por el que un administrador debe revisar la aplicación. |
| `rejection_reason` | `insufficient-information`, `no-matching-voter`, `voter-already-approved` u `other`. |
| `rejection_message` | El texto del rechazo. Obligatorio para el motivo `other`. |
| `verified_by` | El administrador que decidió. |

## Reglas {#rules}

| Regla | Valor |
| --- | --- |
| Tipo de archivo | `.csv`. El separador es una coma. |
| Encabezado | La exportación escribe una fila de encabezado. La importación no la omite. La omite porque el valor `tenant_id` no es un UUID válido. Un archivo sin fila de encabezado también funciona. |
| Codificación | UTF-8. |
| Comillas | La exportación pone las columnas JSON entre comillas dobles y duplica cada comilla interior. No cambie esto. |
| Orden de las columnas | Fijo. No mueva ni elimine una columna. |
| Tamaño | El tamaño está limitado por la configuración de la instalación para la subida de archivos. |
| Permiso | El usuario necesita el permiso **Importación de Aplicaciones**. |
| Filtro de elección | Si exporta desde la pestaña **Aprobaciones** de una elección, el archivo solo tiene las aplicaciones de esa elección. |

## Archivo de ejemplo {#example-file}

```csv
id,created_at,updated_at,tenant_id,election_event_id,area_id,applicant_id,applicant_data,labels,annotations,verification_type,status
4f1c0b0e-0000-4000-8000-000000000001,2026-10-01 09:30:00 +00:00,2026-10-01 09:30:05 +00:00,90505c8a-23a9-4cdf-a26b-4e19f6a097d5,c2a1d6f0-0000-4000-8000-000000000002,7d3e5a10-0000-4000-8000-000000000003,applicant-001,"{""firstName"":""Ana"",""lastName"":""Lopez"",""email"":""voter001@example.com"",""username"":""voter001""}","{}","{""search-attributes"":""firstName,lastName"",""mismatches"":0}",AUTOMATIC,ACCEPTED
```

## Qué hace la importación {#what-the-import-does}

1. El portal de administración sube el archivo y pide la **Verificación de Integridad (SHA 256)**.
   La tarea comprueba el hash si usted lo escribió.
2. La tarea se inicia. Consulte la pestaña **Tareas** para ver el resultado.
3. Para cada fila, la tarea lee las 12 columnas. Omite una fila si `tenant_id` no es un UUID válido.
4. La tarea añade todas las demás filas en una sola transacción de base de datos. Si una fila es
   incorrecta, la tarea no añade nada y el estado es **Failed**.
5. Cada aplicación recibe un nuevo ID. Las fechas, los datos, el estado y el tipo se mantienen
   como en el archivo.
6. La importación no envía mensajes a los solicitantes. No crea ni cambia votantes.

Efecto conocido: la importación guarda `labels` y `annotations` como texto JSON y no como un objeto
JSON.

### Mensajes y errores {#messages-and-errors}

| Mensaje | Dónde | Significado y acción |
| --- | --- | --- |
| Aplicaciones importadas con éxito | Notificación | La tarea terminó. |
| Error al importar las aplicaciones | Notificación | El portal no pudo iniciar la tarea. Inténtelo de nuevo. |
| `Failed to verify the integrity: Hash of voters file: ... does not match with the input hash: ...` | Registro de la tarea | El hash que escribió no es el hash del archivo. (El mensaje dice "voters file" también para este archivo.) |
| `Error reading CSV record` | Registro de la tarea | Una fila tiene un número incorrecto de columnas o comillas incorrectas. |
| `Error inserting application: ...` | Registro de la tarea | La base de datos rechazó una fila. Compruebe `election_event_id` y `area_id`. |
| `document not found` | Registro de la tarea | La subida falló. Suba el archivo de nuevo. |

## Páginas relacionadas {#related-pages}

- [Aprobaciones](../User-Manual/Election-Management/Election-Event/Approvals/election_management_election-event_approvals.md): la pestaña **Aprobaciones**, **Importar** y **Exportar**.
- [Archivo de votantes](03-voters.md): los votantes que coinciden con las aplicaciones.
- [Formatos de archivos de importación](index.md): todos los archivos de importación.
