---
id: import_format_tenant_backup
title: Archivo de copia de seguridad del inquilino
sidebar_position: 7
description: "El formato del archivo de copia de seguridad del inquilino: los tres archivos del ZIP, cada columna y clave, y lo que usa la restauración."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

El archivo de copia de seguridad del inquilino es un archivo ZIP con la configuración de un inquilino. Usted lo crea con **Respaldo** en **Configuración** > **Copia de seguridad / Restaurar**. Lo usa con **Restaurar** en la misma página. No cree el archivo a mano.

Use la copia de seguridad para guardar una copia de la configuración del inquilino, o para copiar la configuración a otro inquilino. La copia de seguridad no contiene procesos electorales, votantes ni votos.

:::caution PRECAUCIÓN: archivo de copia de seguridad del inquilino
Guarde el archivo de copia de seguridad en un lugar protegido. La configuración del inquilino puede contener la clave de un servicio externo (`gapi_key`). Cualquier persona que tenga el archivo puede leerla.
:::

Necesita el permiso **Leer Inquilino** (`tenant-read`) para crear la copia de seguridad. Necesita el permiso **Editar Inquilino** (`tenant-write`) para restaurarla.

## Archivos del ZIP {#files-in-the-zip}

El portal de administración descarga el archivo con el nombre `tenant-config-<tenant ID>-export.zip`. El ZIP tiene tres archivos. `<id>` es el ID del inquilino.

| Archivo | Contenido | Lo usa la restauración |
| --- | --- | --- |
| `tenant_configuration-<id>.csv` | La configuración del inquilino. Una fila de encabezado y una fila de datos. | Opción **Importar configuraciones del inquilino** |
| `keycloak_admin-<id>.json` | La configuración de inicio de sesión del inquilino (un realm de Keycloak), en JSON. | Opción **Importar configuraciones de Keycloak**. Solo el nombre para mostrar y los textos. |
| `export_permissions-<id>.csv` | Los roles y sus permisos. | Opción **Importar configuraciones de roles y permisos** |

La restauración encuentra un archivo por una parte de su nombre: `tenant_configuration`, `keycloak_admin` y `export_permissions`. La restauración omite las entradas que empiezan por `.` o `__MACOSX/`, y las carpetas. Un archivo ZIP que vuelva a crear con un gestor de archivos funciona si los nombres de los archivos son los mismos.

## Campos de `tenant_configuration-<id>.csv` {#fields-of-tenant_configuration-idcsv}

El archivo tiene una fila de encabezado y una fila de datos. La restauración lee las columnas por posición, no por nombre. La exportación escribe cada valor como JSON. Un valor de texto tiene sus propias comillas dobles, así que en el archivo CSV aparece con tres comillas a cada lado.

| Columna | Posición | Obligatoria | Valores permitidos o formato | Significado | Lo usa la restauración | Ejemplo |
| --- | --- | --- | --- | --- | --- | --- |
| `id` | 0 | Sí | Un UUID | El ID del inquilino. | No. La restauración cambia el inquilino en el que ha iniciado sesión. | `"90505c8a-23a9-4cdf-a26b-4e19f6a097d5"` |
| `slug` | 1 | Sí | Texto | El nombre corto del inquilino. | No. La restauración lo lee, pero no lo guarda. | `"example"` |
| `created_at` | 2 | Opcional | Fecha y hora en ISO 8601 | Cuándo se creó el inquilino. | Sí | `"2026-01-05T10:00:00+00:00"` |
| `updated_at` | 3 | Opcional | Fecha y hora en ISO 8601 | Cuándo cambió el inquilino. | Sí | `"2026-10-01T09:30:00+00:00"` |
| `labels` | 4 | Opcional | JSON | Datos adicionales. | Sí | `null` |
| `annotations` | 5 | Opcional | Objeto JSON | El aspecto del inquilino: `logo_url` y `css`. | Sí | `{"logo_url":"https://example.com/logo.png","css":""}` |
| `is_active` | 6 | Sí | `true` o `false` | Indica si el inquilino está activo. | Sí | `true` |
| `voting_channels` | 7 | Opcional | Objeto JSON, vea más abajo | Los canales de voto del inquilino. | Sí | `{"online":true,"kiosk":false,"telephone":false}` |
| `settings` | 8 | Opcional | Objeto JSON, vea más abajo | La configuración del inquilino. | Sí | Vea más abajo. |
| `test` | 9 | Opcional | Un número entero o `null` | Una marca para los inquilinos de prueba. | No. Vea la nota siguiente. | `null` |

Si `is_active` falta o no es `true` ni `false`, la restauración se detiene con el error `Error deserializing is_active`. La restauración pone `created_at` y `updated_at` con los valores del archivo. Si no se puede leer una fecha, la restauración guarda una fecha vacía.

### Claves de `voting_channels` {#keys-of-voting_channels}

| Clave | Tipo | Significado |
| --- | --- | --- |
| `online` | `true` o `false` | Votación por internet. Si falta la clave, el portal de administración muestra `true`. |
| `kiosk` | `true` o `false` | Votación en un quiosco. Si falta la clave, el portal de administración muestra `false`. |
| `telephone` | `true` o `false` | Votación por teléfono. Si falta la clave, el portal de administración muestra `false`. |

### Claves de `settings` {#keys-of-settings}

Todas las claves son opcionales.

| Clave | Tipo | Significado |
| --- | --- | --- |
| `language_conf` | Objeto | Los idiomas. Tiene `enabled_language_codes` (una lista de códigos, por ejemplo `["en","es"]`), `default_language_code` (por ejemplo `"en"`) y `language_detection_policy` (`browser-detect` o `force-default`). |
| `voting_countries` | Lista de textos | Los países donde se bloquea la votación. Vea **Bloqueo de Países**. |
| `enroll_countries` | Lista de textos | Los países donde se bloquea la inscripción. Vea **Bloqueo de Países**. |
| `help_links` | Lista de objetos | Los enlaces de ayuda del portal de votación. Cada objeto tiene `url`, `title` y `i18n` opcional. |
| `i18n` | Objeto | Los textos del inquilino para cada idioma. |
| `sms`, `mail` | `true` o `false` | Los canales para los mensajes. |
| `schedules`, `schedulesIds` | Lista | Los eventos programados del inquilino. |
| `gapi_key`, `gapi_email` | Objeto, texto | Las credenciales de un servicio externo (Integraciones). |

### Nota sobre la columna `test` {#note-on-the-column-test}

La restauración lee el valor de `test` en la posición 8, que es la columna `settings`, y no en la posición 9. La columna `test` del archivo no se usa. Como `settings` no es un número entero, la restauración guarda un valor vacío (`NULL`) en el campo `test` del inquilino. El archivo de exportación no es incorrecto. La restauración siempre borra el campo `test`.

## Campos de `keycloak_admin-<id>.json` {#fields-of-keycloak_admin-idjson}

El archivo es el JSON completo del realm de Keycloak del inquilino. La restauración lee solo dos partes:

| Clave | Tipo | Significado |
| --- | --- | --- |
| `displayName` | Texto | El nombre para mostrar del realm. La restauración lo pone en el realm del inquilino. |
| `localizationTexts` | Objeto | Los textos de las páginas de inicio de sesión para cada idioma. La restauración reemplaza los textos del realm. |

La restauración ignora todas las demás partes del JSON: usuarios, clientes, flujos, grupos, roles y la configuración del realm. Si falta una parte en el archivo, la restauración no la cambia.

## Campos de `export_permissions-<id>.csv` {#fields-of-export_permissions-idcsv}

Un archivo CSV con una fila de encabezado y dos columnas. El separador es una coma.

| Columna | Posición | Obligatoria | Formato | Significado | Ejemplo |
| --- | --- | --- | --- | --- | --- |
| `role` | 0 | Sí | Texto | El nombre de un grupo del realm. En el portal de administración es un **rol**. | `Election Manager` |
| `permissions` | 1 | Sí | Nombres de permisos, separados con `\|` | Los permisos del rol. | `voter-create\|voter-read\|tenant-read` |

Ejemplo:

```csv
role,permissions
Election Manager,voter-create|voter-read|tenant-read
Auditor,voter-read|tenant-read
```

La exportación lista los grupos del realm que tienen roles de realm. El orden de las filas no es fijo.

Para cada fila, la restauración hace lo siguiente:

1. Si el realm tiene un grupo con el nombre de `role`, la restauración hace que los permisos del grupo sean iguales a la lista. Añade los permisos que faltan. Elimina los permisos que no están en la lista. La restauración ignora un nombre de permiso que no existe en el realm.
2. Si el realm no tiene ese grupo, la restauración lo crea y le añade los permisos de todas las filas leídas hasta ese momento, no solo los permisos de su propia fila.

:::caution PRECAUCIÓN: archivo de permisos de la copia de seguridad del inquilino
La restauración elimina permisos de un rol que existe. Revise el archivo antes de restaurar.
:::

## Reglas {#rules}

| Regla | Valor |
| --- | --- |
| Tipo de archivo | `.zip` |
| Separador CSV | Coma. La codificación es UTF-8. |
| Varios valores | En `permissions`, separe los nombres con `\|`. |
| Nombres de los archivos | Mantenga los nombres de los tres archivos. |
| Inquilino | La restauración cambia el inquilino en el que ha iniciado sesión. No usa el ID del inquilino del archivo. |
| Tamaño | El tamaño está limitado por la configuración de subida de archivos de la instalación. |

## Opciones de restauración {#restore-options}

El panel **Restaurar** tiene tres opciones:

- **Importar configuraciones del inquilino**
- **Importar configuraciones de Keycloak**
- **Importar configuraciones de roles y permisos**

:::note NOTA: opciones de restauración de la copia de seguridad del inquilino
En esta versión, el botón **Restaurar** funciona solo si selecciona las tres opciones. La tarea lee cada opción por separado. Un archivo que no tiene uno de los tres archivos no da error para esa opción: la restauración no hace nada para ella.
:::

## Ejemplo {#example}

Un archivo ZIP completo para un inquilino con el ID `90505c8a-23a9-4cdf-a26b-4e19f6a097d5`:

```text
tenant-config-90505c8a-23a9-4cdf-a26b-4e19f6a097d5-export.zip
  tenant_configuration-90505c8a-23a9-4cdf-a26b-4e19f6a097d5.csv
  keycloak_admin-90505c8a-23a9-4cdf-a26b-4e19f6a097d5.json
  export_permissions-90505c8a-23a9-4cdf-a26b-4e19f6a097d5.csv
```

Contenido de `tenant_configuration-<id>.csv` (la fila de datos está en una sola línea):

```csv
id,slug,created_at,updated_at,labels,annotations,is_active,voting_channels,settings,test
"""90505c8a-23a9-4cdf-a26b-4e19f6a097d5""","""example""","""2026-01-05T10:00:00+00:00""","""2026-10-01T09:30:00+00:00""",null,"{""logo_url"":""https://example.com/logo.png"",""css"":""""}",true,"{""online"":true,""kiosk"":false,""telephone"":false}","{""language_conf"":{""enabled_language_codes"":[""en"",""es""],""default_language_code"":""en"",""language_detection_policy"":""browser-detect""}}",null
```

## Qué hace la restauración {#what-the-restore-does}

1. El portal de administración sube el archivo y pide la **Verificación de Integridad (SHA 256)**. La tarea comprueba el hash si lo escribió.
2. La tarea **Importar Configuración del Cliente** empieza. Mire el widget de la tarea o la pestaña **Tareas**.
3. La tarea lee el realm del inquilino desde el servicio de inicio de sesión.
4. Para cada opción seleccionada, la tarea lee el archivo correspondiente del ZIP y cambia el inquilino (base de datos), el realm (nombre para mostrar y textos) o los roles (grupos y permisos).
5. La restauración cambia el inquilino existente. No crea un inquilino nuevo.

La tarea hace los cambios en el servicio de inicio de sesión uno tras otro. Si la tarea falla a mitad, algunos cambios pueden quedar.

Si la confirmación en la base de datos falla, la tarea aparece igualmente como completada, y los cambios del inquilino no se guardan. Los cambios hechos en el servicio de inicio de sesión antes de la confirmación permanecen.

### Mensajes y errores {#messages-and-errors}

| Mensaje | Dónde aparece | Significado y acción |
| --- | --- | --- |
| Importar Configuración del Cliente | Nombre de la tarea | El nombre de la tarea en la pestaña **Tareas**. |
| `Failed to verify the integrity: Hash of voters file: ... does not match with the input hash: ...` | Registro de la tarea | El hash que escribió no es el hash del archivo. (El mensaje dice "voters file" también para este archivo.) |
| `Zip archive error: ...` | Registro de la tarea | El archivo no es un archivo ZIP. |
| `Error deserializing is_active` | Registro de la tarea | La columna `is_active` del archivo del inquilino no es `true` ni `false`. |
| `Invalid header name: ...` | Registro de la tarea | Un nombre de columna del archivo del inquilino tiene un carácter que no está permitido. |
| `Failed to deserialize Keycloak realm configuration: ...` | Registro de la tarea | El archivo JSON no es válido. |
| `Error obtaining realm` | Registro de la tarea | El servicio de inicio de sesión no respondió. Inténtelo de nuevo. |
| `Error adding roles to new group ...`, `Error creating group ...` | Registro de la tarea | El servicio de inicio de sesión rechazó un cambio de los roles. |

## Páginas relacionadas {#related-pages}

- [Configurar el inquilino](../../03-procedures/01-tenant.md): la configuración del inquilino.
- [Archivo de plantillas](06-templates.md): las plantillas no están en la copia de seguridad. Expórtelas por separado.
- [Formatos de archivos de importación](index.md): todos los archivos de importación.
