---
id: import_format_voters
title: Archivo de votantes
sidebar_position: 3
description: "El formato del archivo de votantes: las columnas, las reglas, un archivo de ejemplo y lo que hace la importación con cada fila."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

El archivo de votantes es un archivo de texto con una fila para cada votante. Usted lo prepara, por ejemplo, a partir de una lista de votantes o de una hoja de cálculo. Lo importa en un proceso electoral, en la pestaña **Votantes**, con **Importar**. Para los pasos, consulte [Añadir los votantes](../../procedures/03-voters.md).

El mismo formato de archivo se usa en la página **Usuarios** del inquilino, para importar administradores. En ese caso, la importación no busca áreas. El valor de `area_name` se guarda tal como está escrito.

## Campos {#fields}

La primera fila del archivo es el encabezado. El encabezado da nombre a las columnas. El orden de las columnas no es importante. Todas las columnas son opcionales. La tabla muestra las columnas que tienen un significado especial.

| Columna | Obligatoria | Valores permitidos | Valor por defecto | Significado | Ejemplo |
| --- | --- | --- | --- | --- | --- |
| `username` | Opcional | Texto. La importación lo cambia a minúsculas. | Un ID aleatorio | El nombre de usuario del votante. Debe ser único en el proceso electoral. | `voter001` |
| `email` | Opcional | Texto. La importación lo cambia a minúsculas. La importación no comprueba el formato. | Vacío | La dirección de correo electrónico del votante. | `voter001@example.com` |
| `email_verified` | Opcional | `TRUE` o `FALSE`. La importación lo cambia a minúsculas. Una celda vacía detiene la importación. | `TRUE` si la columna no está en el archivo | Indica si la dirección de correo electrónico está verificada. | `TRUE` |
| `first_name` | Opcional | Texto | Vacío | El nombre del votante. | `Ana` |
| `last_name` | Opcional | Texto | Vacío | El apellido del votante. | `Lopez` |
| `enabled` | Opcional | `TRUE` o `FALSE`, en cualquier combinación de mayúsculas y minúsculas. Una celda vacía detiene la importación. | `TRUE` si la columna no está en el archivo | Si es `FALSE`, el votante no puede iniciar sesión. | `TRUE` |
| `area_name` | Opcional | El **Nombre** de un área del proceso electoral. | Sin área | El área del votante. La importación guarda el ID del área. | `North` |
| `area-id` | Opcional | El **Nombre** de un área. | Sin área | Igual que `area_name`. No use las dos columnas en un mismo archivo. | `North` |
| `group_name` | Opcional | El nombre de un grupo del realm. | `voter` | El grupo del votante. | `voter` |
| `password` | Opcional | Texto | Sin contraseña | La contraseña en texto plano. La importación guarda solo un hash de ella. | `Example-Pass-1` |
| `hashed_password` | Opcional | Texto en Base64 | Sin contraseña | Una contraseña que ya tiene hash. Úsela con `password_salt`. | `Zm9v...` |
| `password_salt` | Obligatoria con `hashed_password` | Texto en Base64 | Ninguno | La sal de la contraseña con hash. | `YmFy...` |
| `num_of_iterations` | Opcional | Un número entero, 1 o más | `27500` | Las iteraciones PBKDF2 de la contraseña con hash. | `27500` |
| `authorized-election-ids` | Opcional | Los ID externos de una o más elecciones, separados con `\|` | Vacío | Las elecciones en las que puede votar el votante. | `election-1\|election-2` |
| `id`, `election__<name>` | No se usa | Cualquiera | Ninguno | Se ignora en la importación. La exportación de votantes tiene la columna `id` y una columna `election__` para cada elección. | |
| Otras columnas | Opcional | Texto | Vacío | Cada otra columna se convierte en un atributo del votante con el mismo nombre. | `dateOfBirth` |

### Área {#area}

- La importación busca el área por su nombre, en el proceso electoral. El nombre debe ser exactamente igual que en la pestaña **Áreas**. La búsqueda distingue entre mayúsculas y minúsculas.
- Si la importación no encuentra el área, no se detiene. El votante no tiene área. Un votante sin área no se puede vincular a las elecciones de un área. Compruebe el área de cada votante después de la importación.
- `area_name` y `area-id` son el mismo campo. Si el archivo tiene las dos columnas, la importación se detiene con el error `Error executing create table query`. Mantenga solo una de las dos columnas.

### Grupo {#group}

La importación añade cada votante al grupo indicado en `group_name`. Si el archivo no tiene la columna `group_name`, el grupo es `voter`. Si el realm no tiene un grupo con este nombre, la importación no añade el votante a ningún grupo y no muestra un error.

### Contraseña {#password}

Use una de estas opciones. Si no usa ninguna, el votante no tiene contraseña. Este es el caso normal cuando los votantes inician sesión con otros datos, o cuando envía los datos de acceso más tarde.

- **Contraseña en texto plano.** Use la columna `password`. La importación crea una sal aleatoria para cada votante y calcula el hash de la contraseña con PBKDF2-SHA256 y 27 500 iteraciones. La importación guarda solo el hash. No añada la columna `num_of_iterations` a este archivo. La importación calcula el hash con 27 500 iteraciones, pero guarda el valor de `num_of_iterations`, por lo que la comprobación de la contraseña puede fallar.
- **Contraseña con hash.** Use las columnas `hashed_password` y `password_salt`. Los dos valores son texto en Base64. El algoritmo es PBKDF2-SHA256 y la longitud del hash es de 32 bytes. Añada `num_of_iterations` si el hash usa otro número de iteraciones. Si la celda está vacía, no es un número o es `0`, la importación usa 27 500.

Si el archivo tiene las columnas `password` y `hashed_password`, la importación usa `password`. Si `hashed_password` no tiene `password_salt`, la importación se detiene con el error `Password salt empty`.

Si el archivo tiene la columna `password` y una celda de esa columna está vacía, la importación guarda el hash de una contraseña vacía para ese votante. No deje vacía una celda de esta columna. Para votantes sin contraseña, use un archivo sin la columna `password`.

:::caution PRECAUCIÓN: archivo de votantes con contraseñas
No guarde un archivo con contraseñas en texto plano. Borre el archivo después de la importación. Si puede, dé a los votantes sus datos de acceso con la opción **Enviar** del portal de administración, y no use la columna `password`.
:::

### Elecciones autorizadas {#authorized-elections}

La columna `authorized-election-ids` limita las elecciones en las que puede votar el votante. Cada valor es el **ID externo** de una elección. Si una elección no tiene ID externo, use el ID de la elección. Separe los valores con `|`.

- Si la celda está vacía, el votante está autorizado para todas las elecciones del proceso electoral.
- Un valor que no coincide con ninguna elección se ignora.

El token del votante contiene los ID de las elecciones autorizadas, no los ID externos.

## Reglas {#rules}

| Regla | Valor |
| --- | --- |
| Tipo de archivo | `.csv` (separador: coma) o `.tsv` (separador: tabulación). La importación decide por el final del nombre del archivo. Cualquier otro nombre se lee como `.csv`. |
| Encabezado | La primera fila. Es obligatoria. |
| Codificación | Use UTF-8. |
| Nombres de columna | Solo las letras `a-z` y `A-Z`, los dígitos, `.`, `_` y `-`. Cualquier otro carácter detiene la importación. |
| Columnas duplicadas | Dos encabezados que tienen el mismo nombre después de que la importación cambie `.` y `-` a `_`, o que solo se diferencian en mayúsculas y minúsculas, detienen la importación. Por ejemplo, `area-id` y `area_name`, o `Email` y `email`. |
| Número de campos | Cada fila debe tener el mismo número de campos que el encabezado. Si no, la importación se detiene. |
| Varios valores | Separe los valores de un atributo con `\|`. Una celda vacía no crea ningún atributo. |
| Espacios | La importación no quita los espacios al principio ni al final de un valor. |
| Comillas | Si un valor tiene una coma (o un tabulador en un archivo `.tsv`), ponga el valor entre comillas dobles. |
| Tamaño | El código de la importación no comprueba el tamaño del archivo. |
| Orden de las filas | No es importante. |

:::note NOTA: columnas que no son atributos
Las columnas `password`, `hashed_password`, `password_salt`, `num_of_iterations`, `group_name` y `email_verified` no se guardan como atributos. Las columnas `id`, `username`, `email`, `first_name`, `last_name`, `enabled` y `election__...` tampoco se guardan como atributos. Todas las demás columnas se convierten en atributos.
:::


## Archivo de ejemplo {#example-file}

```csv
username,email,first_name,last_name,area_name,enabled,group_name,authorized-election-ids,sequent.read-only.mobile-number,dateOfBirth
voter001,voter001@example.com,Ana,Lopez,North,TRUE,voter,,+10000000001,1990-01-31
voter002,voter002@example.com,Ben,Smith,South,TRUE,voter,election-1|election-2,+10000000002,1985-06-15
voter003,voter003@example.com,Eva,Brown,North,FALSE,voter,,,
```

El primer votante está en el área North. La celda `authorized-election-ids` está vacía, por lo que el votante está autorizado para todas las elecciones del proceso electoral. El segundo votante está en el área South y solo está autorizado para `election-1` y `election-2`. El tercer votante está deshabilitado.

## Qué hace la importación {#what-the-import-does}

1. El portal de administración sube el archivo cuando usted lo selecciona. La notificación **Fichero subido al servidor - pero no importado aún** aparece cuando termina la subida. Después, escriba la **Verificación de Integridad (SHA 256)** y haga clic en **Importar**. Si el campo está vacío, el portal pregunta **Importar Sin Verificación de Integridad?**. Haga clic en **Sí, Importar Sin Verificación de Integridad** o en **Volver**.
2. El servidor comprueba que usted puede crear votantes (en la pestaña **Votantes**) o usuarios (en la página **Usuarios**). El portal inicia la tarea **Importar usuarios** y la muestra en el panel de tareas.
3. Si escribió un valor SHA-256, la tarea comprueba el hash. La tarea comprueba el encabezado. Lee las filas.
4. La tarea crea todos los votantes en una sola transacción de base de datos. Si una fila es incorrecta, la tarea no hace ningún cambio. El estado de la tarea es `FAILED` y el registro de la tarea indica el motivo. Corrija el archivo e impórtelo de nuevo.
5. Después de una importación correcta, los votantes aparecen en la pestaña **Votantes**. La importación no envía correos electrónicos ni mensajes.

Si el archivo tiene al menos una columna que se convierte en atributo (por ejemplo `area_name`), la importación también da a cada votante el atributo `tenant-id`.

Un archivo sin una columna de ese tipo no da a los votantes el atributo `tenant-id`.

### Mensajes y errores {#messages-and-errors}

| Mensaje | Dónde | Significado y acción |
| --- | --- | --- |
| Error Importando Votantes. | Notificación | El servidor devolvió un error cuando el portal inició la tarea. Inténtelo de nuevo. |
| Error subiendo el fichero | Notificación | La subida falló. Seleccione el archivo de nuevo. |
| Fichero subido al servidor - pero no importado aún | Notificación | El archivo está en el servidor. Haga clic en **Importar**. |
| `Failed to verify the integrity: Hash of voters file: ... does not match with the input hash: ...` | Registro de la tarea | El SHA-256 que escribió no es el hash del archivo. Compruebe el archivo y el hash. |
| `CSV Header contains characters not allowed: <name>` | Registro de la tarea | Un nombre de columna tiene un carácter que no está permitido. |
| `Error reading CSV record: <error>` | Registro de la tarea | Una fila tiene un número de campos distinto al del encabezado, o una fila no se puede leer como UTF-8. Corrija la fila. El mensaje indica la posición de la fila. |
| `Error executing create table query: <error>` | Registro de la tarea | Dos encabezados tienen el mismo nombre en la importación. Elimine uno de ellos. |
| `Password salt empty` | Registro de la tarea | Añada la columna `password_salt`, con un valor en cada fila. |
| `Error executing INSERT USER transaction: <error>` | Registro de la tarea | La base de datos rechazó una fila. El mensaje no indica qué fila ni qué regla falló. Motivos habituales: un nombre de usuario que ya existe, o una celda vacía en `enabled` o `email_verified`. |


## Páginas relacionadas {#related-pages}

- [Añadir los votantes](../../procedures/03-voters.md): los pasos para importar el archivo.
- [Votantes](../User-Manual/Election-Management/Election-Event/Voters/election_management_election-event_voters.md): la pestaña **Votantes**.
- [Archivo de áreas](04-areas.md): las áreas deben existir antes de la importación de votantes.
- [Formatos de archivo de importación](index.md): todos los archivos de importación.
