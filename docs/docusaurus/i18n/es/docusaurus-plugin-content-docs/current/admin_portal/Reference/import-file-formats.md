---
id: import_file_formats
title: Formatos de archivos de importación
description: "Los formatos de los archivos que puede importar en el portal de administración: qué contiene cada archivo, cómo crearlo y ejemplos."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

Esta página da el formato de cada archivo que el portal de administración puede importar. La
mayoría son archivos que exporta desde el portal de administración. Algunos, por ejemplo el
archivo de votantes, los crea usted.

## Resumen {#summary}

| Archivo | Dónde se importa | Tipo | ¿Lo crea usted? |
| --- | --- | --- | --- |
| [Proceso electoral](#election-event) | **Procesos Electorales** > **+** > **Importar Evento Electoral** | `.json`, `.zip`, `.ezip` | No. Use una exportación. |
| [Votantes](#voters) | Proceso electoral > **Votantes** > **Importar** | `.csv`, `.tsv` | Sí |
| [Áreas](#areas) | Proceso electoral > **Áreas** > **Importar** | `.csv` | Sí |
| [Solicitudes](#applications) | Proceso electoral > **Aprobaciones** > **Importar** | `.csv` | No. Use una exportación. |
| [Plantillas](#templates) | **Plantillas** > **Importar** | `.csv` | No. Use una exportación. |
| [Copia de seguridad del inquilino](#tenant-backup) | **Configuración** > **Copia de seguridad / Restaurar** | `.zip` | No. Use una exportación. |
| [Copia de la clave privada](#private-key-backup) | Ceremonia de claves y ceremonia de recuento | `.txt` | No. Use el archivo descargado. |
| [Presentación de la elección](#election-presentation) | Elección > **Datos** > configuración avanzada | `.json` | Sí, con cuidado |

Cada panel de importación tiene el campo **Verificación de Integridad (SHA 256)**. Escriba allí el
hash SHA-256 del archivo. Así el portal de administración rechaza un archivo que alguien ha
cambiado. Para obtener el hash de un archivo:

```bash
shasum -a 256 <archivo>
```

En Windows, use `certutil -hashfile <archivo> SHA256`.

## Proceso electoral {#election-event}

Importe un proceso electoral para hacer una copia de un proceso electoral anterior, o para moverlo
a otro inquilino. Cree el archivo con **Exportar** en el proceso electoral. No cambie el archivo.

| Tipo de archivo | Contenido |
| --- | --- |
| `.json` | Solo la configuración: el proceso electoral, sus elecciones, preguntas, candidatos, áreas y los vínculos entre áreas y preguntas. |
| `.zip` | El archivo JSON y, si los seleccionó en la exportación, los votantes, informes, eventos programados, publicaciones, documentos, imágenes, logs, claves y datos del recuento. |
| `.ezip` | Un archivo `.zip` cifrado con una contraseña. Escriba la contraseña en **Contraseña de Descifrado**. |

La exportación da al archivo el nombre `election-event-<ID>-export-sha256-<hash>.zip` (o `.ezip`).
El nombre contiene el hash SHA-256 del archivo.

La importación:

- Añade el proceso electoral al inquilino en el que ha iniciado sesión. No usa el ID de inquilino
  del archivo.
- Da nuevos ID al proceso electoral y a todas sus partes.
- Restablece el estado, los periodos de votación y las estadísticas. El proceso electoral no tiene
  clave de la elección. Haga una nueva ceremonia de claves.

:::caution PRECAUCIÓN: hash del archivo del proceso electoral
En esta versión el portal de administración no envía al servidor el hash de un archivo de proceso
electoral, por lo que la importación no lo comprueba. Compare el hash usted mismo antes de
importar: el hash que da `shasum` debe ser igual al hash del nombre del archivo. Una exportación
`.zip` o `.ezip` puede contener datos de votantes y claves. Guárdela en un lugar protegido.
:::

## Votantes {#voters}

Un archivo de texto con una fila por votante. La primera fila es el encabezado. El separador es una
coma. Para usar el tabulador como separador, dé al archivo la extensión `.tsv`. El orden de las
columnas no importa.

```csv
username,email,first_name,last_name,area_name,enabled,authorized-election-ids
voter001,voter001@example.com,Ana,Lopez,North,TRUE,
voter002,voter002@example.com,Ben,Smith,South,TRUE,election-1|election-2
```

| Columna | Contenido |
| --- | --- |
| `username` | El nombre de usuario. Si no hay columna `username`, la importación crea un nombre aleatorio. |
| `email` | La dirección de correo electrónico. |
| `first_name`, `last_name` | El nombre del votante. |
| `area_name` | El nombre del área, como en la pestaña **Áreas**. |
| `enabled` | `TRUE` o `FALSE`. No la deje vacía. |
| `password` | Opcional. La contraseña. La importación solo guarda un hash de ella. |
| `hashed_password`, `password_salt`, `num_of_iterations` | Opcional. Una contraseña ya cifrada con PBKDF2-SHA256, y su sal, en Base64. Úselas en lugar de `password`. |
| `authorized-election-ids` | Opcional. Los alias de las elecciones en las que el votante puede votar. |
| Otras columnas | Cada otra columna se convierte en un atributo del votante con el mismo nombre, por ejemplo `sequent.read-only.mobile-number` o `dateOfBirth`. |

Los nombres de columna solo pueden contener letras, dígitos, `.`, `_` y `-`. Para dar más de un
valor a un atributo, separe los valores con `|`. Para los pasos, consulte
[Añadir los votantes](../procedures/03-voters.md).

## Áreas {#areas}

Un archivo CSV sin fila de encabezado. Cada fila tiene cinco columnas:

```csv
North,,,North District,0
South,,,South District,0
```

| Columna | Contenido |
| --- | --- |
| 1 | El **Nombre** del área. |
| 2, 3 | No se usan. Déjelas vacías. |
| 4 | La **Descripción** del área. |
| 5 | `0` para importar la fila. Las filas con otro valor no se importan. |

La importación vincula cada área con todas las preguntas del proceso electoral. Para los pasos,
consulte [Crear el proceso electoral](../procedures/02-event.md#import-areas-alternative).

## Solicitudes {#applications}

Cree el archivo con **Exportar** en la pestaña **Aprobaciones**. El archivo tiene una fila de
encabezado y doce columnas:

```csv
id,created_at,updated_at,tenant_id,election_event_id,area_id,applicant_id,applicant_data,labels,annotations,verification_type,status
```

La importación usa el ID de inquilino y el ID de proceso electoral de cada fila. Importe el archivo
solo en el proceso electoral donde lo exportó. Consulte
[Aprobaciones](User-Manual/Election-Management/Election-Event/Approvals/election_management_election-event_approvals.md#import-file).

## Plantillas {#templates}

Cree el archivo con **Exportar** en la página **Plantillas**. El archivo tiene una fila de
encabezado y diez columnas:

```csv
alias,tenant_id,template,created_by,labels,annotations,created_at,updated_at,communication_method,type
```

La importación usa el ID de inquilino de cada fila. Importe el archivo solo en el inquilino donde
lo exportó. Consulte
[Plantillas](User-Manual/Templates/admin_portal_reference_user-manual_templates.md#import-templates).

## Copia de seguridad del inquilino {#tenant-backup}

Cree el archivo con **Respaldo** en **Configuración** > **Copia de seguridad / Restaurar**. El
archivo tiene el nombre `tenant-config-<ID del inquilino>-export.zip` y contiene:

| Archivo del ZIP | Contenido |
| --- | --- |
| `tenant_configuration-<ID del inquilino>.csv` | Los ajustes del inquilino. |
| `keycloak_admin-<ID del inquilino>.json` | La configuración de inicio de sesión del inquilino. La restauración solo usa su nombre visible y sus textos. |
| `export_permissions-<ID del inquilino>.csv` | Los roles y sus permisos, con el encabezado `role,permissions`. Los permisos de un rol se separan con `\|`. |

La restauración cambia el inquilino en el que ha iniciado sesión. No usa el ID de inquilino del
archivo. En esta versión debe seleccionar las tres opciones de restauración. Consulte
[Copia de seguridad / Restaurar](User-Manual/Settings/backup--restore/admin_portal_reference_user-manual_settings_backup--restore.md).

## Copia de la clave privada {#private-key-backup}

En la ceremonia de claves, cada fideicomisario descarga el archivo
`encrypted_private_key_trustee_<nombre de usuario>_<nombre del proceso electoral>.txt`. El
fideicomisario sube el mismo archivo para comprobarlo en la ceremonia de claves, y otra vez en la
ceremonia de recuento. No cambie el archivo, y no lo abra en un editor de texto que pueda guardar
cambios. El portal de administración lo compara con la clave cifrada que guarda. Si un carácter es
diferente, muestra "Invalid Encrypted Private Key Backup".

## Presentación de la elección {#election-presentation}

Una opción avanzada. En la pestaña **Datos** de una elección, puede subir un archivo `.json` en
**Archivo de configuración**. Cada clave de primer nivel del archivo sustituye a la misma clave de
la presentación de la elección. Las demás claves no cambian. Suba solo un archivo que recibió del
soporte de Sequent o de la presentación de otra elección.

## Otras importaciones {#other-imports}

- **Importar Candidatos** en la pestaña **Datos** lee el formato fijo de una integración externa
  específica. No es una importación de uso general. Para añadir candidatos, use el árbol del
  proceso electoral o importe un proceso electoral.
- El archivo de firma de una transmisión y las imágenes y documentos que añade a candidatos y
  materiales de apoyo no tienen un formato de Step.
