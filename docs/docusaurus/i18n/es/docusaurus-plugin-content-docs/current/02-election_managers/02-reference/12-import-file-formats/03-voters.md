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

El archivo de votantes es un archivo de texto con una fila para cada votante. Usted lo crea, por
ejemplo a partir de una lista de votantes o de una hoja de cálculo. Lo importa en un proceso
electoral, en la pestaña **Votantes**, con **Importar**. Para los pasos, consulte
[Añadir los votantes](../../03-procedures/03-voters.md).

El mismo formato de archivo se usa en la página **Usuarios** del inquilino, para importar
administradores. En ese caso, la importación no busca áreas y no lee atributos secretos.

## Campos {#fields}

La primera fila del archivo es el encabezado. El encabezado da el nombre de las columnas. El orden de
las columnas no es importante. Todas las columnas son opcionales. La tabla muestra las columnas con
un significado especial.

| Columna | Obligatorio | Valores permitidos | Valor por defecto | Significado | Ejemplo |
| --- | --- | --- | --- | --- | --- |
| `username` | Opcional | Texto. La importación lo cambia a minúsculas. | Un ID aleatorio | El nombre de usuario del votante. Debe ser único en el proceso electoral. | `voter001` |
| `email` | Opcional | Una dirección de correo electrónico. La importación la cambia a minúsculas. | Vacío | La dirección de correo electrónico del votante. | `voter001@example.com` |
| `email_verified` | Opcional | `TRUE` o `FALSE`. La importación lo cambia a minúsculas. | `TRUE` | Indica si la dirección de correo electrónico está verificada. | `TRUE` |
| `first_name` | Opcional | Texto | Vacío | El nombre del votante. | `Ana` |
| `last_name` | Opcional | Texto | Vacío | Los apellidos del votante. | `Lopez` |
| `enabled` | Opcional | `TRUE` o `FALSE`, en cualquier forma | `TRUE` | Si es `FALSE`, el votante no puede iniciar sesión. Una celda vacía es `TRUE`. | `TRUE` |
| `area_name` | Opcional | El **Nombre** de un área del proceso electoral. | Sin área | El área del votante. La importación guarda el ID del área. | `North` |
| `area-id` | Opcional | El **Nombre** de un área. | Sin área | Igual que `area_name`. No use las dos columnas en un mismo archivo. | `North` |
| `group_name` | Opcional | El nombre de un grupo del realm. | `voter` | El grupo del votante. | `voter` |
| `password` | Opcional | Texto | Sin contraseña | La contraseña en texto plano. La importación guarda solo un hash de ella. | `Example-Pass-1` |
| `hashed_password` | Opcional | Texto Base64 | Sin contraseña | Una contraseña que ya tiene hash. Úsela con `password_salt`. | `Zm9v...` |
| `password_salt` | Obligatorio con `hashed_password` | Texto Base64 | Ninguno | La sal de la contraseña con hash. | `YmFy...` |
| `num_of_iterations` | Opcional | Un número entero, 1 o más | `27500` | Las iteraciones PBKDF2 de la contraseña con hash. | `27500` |
| `vote-weight` | Opcional | Un número entero de 1 a 100 000 | Vacío (peso 1) | El peso del voto del votante. | `3` |
| `authorized-election-ids` | Opcional | Una o más elecciones, separadas con `\|` | Vacío | Las elecciones en las que puede votar el votante. | `election-1\|election-2` |
| `id`, `election__<name>` | No se usa | Cualquiera | Ninguno | Se ignora en la importación. La exportación de votantes tiene la columna `id` y una columna `election__` para cada elección. | |
| Otras columnas | Opcional | Texto | Vacío | Cada otra columna se convierte en un atributo del votante con el mismo nombre. | `dateOfBirth` |

### Área {#area}

- La importación busca el área por su nombre, en el proceso electoral. El nombre debe ser exactamente
  igual que en la pestaña **Áreas**.
- Si la importación no encuentra el área, no se detiene. El votante no tiene área. Un votante sin
  área no se puede vincular a las elecciones de un área. Compruebe el área de cada votante después de
  la importación.
- `area_name` y `area-id` son el mismo campo. Si el archivo tiene los dos, la importación se detiene
  con el error `Duplicate column`.

### Grupo {#group}

La importación añade cada votante al grupo de `group_name`. Si el archivo no tiene la columna
`group_name`, el grupo es `voter`. Si el realm no tiene un grupo con este nombre, la importación no
añade el votante a ningún grupo y no muestra un error.

### Contraseña {#password}

Use una de estas opciones. Si no usa ninguna, el votante no tiene contraseña. Este es el caso normal
cuando los votantes inician sesión con otros datos, o cuando envía los datos de acceso más tarde.

- **Contraseña en texto plano.** Use la columna `password`. La importación crea una sal aleatoria para
  cada votante, aplica hash a la contraseña con PBKDF2-SHA256 y 27 500 iteraciones. La importación
  guarda solo el hash.
- **Contraseña con hash.** Use las columnas `hashed_password` y `password_salt`. Los dos valores son
  texto Base64. El algoritmo es PBKDF2-SHA256 y la longitud del hash es de 32 bytes. Añada
  `num_of_iterations` si el hash usa otro número de iteraciones. Si la celda está vacía, no es un
  número o es `0`, la importación usa 27 500.

Si el archivo tiene las columnas `password` y `hashed_password`, la importación usa `password`. Si
`hashed_password` no tiene `password_salt`, la importación se detiene con el error `Password salt empty`.

:::caution PRECAUCIÓN: archivo de votantes con contraseñas
No guarde un archivo con contraseñas en texto plano. Borre el archivo después de la importación. Si
puede, dé a los votantes sus datos de acceso con la opción **Enviar** del portal de administración, y
no use la columna `password`.
:::

### Peso del voto {#vote-weight}

Use `vote-weight` solo si el proceso electoral tiene la política de voto ponderado **Votación
Ponderada por Votantes**. Si la política es otra, la columna se guarda, pero el recuento no la usa.

- El valor es un número entero de 1 a 100 000. La importación también acepta `+5` y `007`, y los
  guarda como `5` y `7`.
- Una celda vacía significa peso 1.
- El nombre de la columna es exactamente `vote-weight`, en minúsculas, con un guion. Los nombres como
  `vote_weight` o `VoteWeight` hacen que la importación se detenga con un error.
- La suma de los pesos de los votantes de un área de una pregunta está limitada a 1 000 000. La
  importación no se detiene si el archivo supera este límite. Escribe una advertencia en el registro
  del servidor. Después, el recuento se niega a contar los votantes de esa área.

### Elecciones autorizadas {#authorized-elections}

La columna `authorized-election-ids` limita las elecciones en las que puede votar el votante. Cada
valor es el **ID externo** de una elección. Si una elección no tiene ID externo, use el ID de la
elección. Separe los valores con `|`.

- Si la celda está vacía y el votante tiene área, el votante puede votar en las elecciones del área.
  Si el área no tiene elecciones, el votante está autorizado en todas las elecciones del proceso
  electoral.
- Si la celda está vacía y el votante no tiene área, el votante está autorizado en todas las elecciones
  del proceso electoral.
- Un valor que no coincide con ninguna elección se ignora.

Escriba el ID externo de la elección. El token del votante usa el ID externo, o el ID si la elección no
tiene ninguno. El recuento solo compara el ID externo, así que en una elección sin ID externo el
recuento no coincide con los valores de los votantes.

### Atributos secretos {#secret-attributes}

Un atributo se puede marcar como secreto en el perfil de usuario del proceso electoral. Entonces, la
importación guarda el valor cifrado.

- El usuario debe tener el permiso **Editar Campos Secretos del Votante** (`voter-secret-attribute-write`).
  Si el archivo tiene una columna secreta y el usuario no tiene el permiso, la importación se detiene
  con el error `Importing encrypted voter attributes requires voter-secret-attribute-write`.
- Un valor puede tener como máximo 150 bytes. Un valor más largo detiene la importación. Use `|` para
  dar más de un valor. Cada valor tiene el límite.
- Una celda vacía no se cifra.
- Estos atributos no pueden ser secretos: `area-id`, `authorized-election-ids`,
  `authorized-to-election-alias`, `dateOfBirth`, `disable-comment`, `email`, `firstName`,
  `first_name`, `lastName`, `last_name`, `permission_labels`,
  `sequent.read-only.id-card-number-validated`, `sequent.read-only.mobile-number`, `tenant-id`,
  `username`, `vote-weight` y `voted-channel`. Si el perfil de usuario marca uno de ellos como
  secreto, la importación se detiene.
- Un atributo secreto no puede usar validadores de valor de Keycloak. Solo se permite el validador
  `person-name-prohibited-characters`.
- La importación escribe una entrada en el registro electoral con los nombres de las columnas secretas
  y el usuario que hizo la importación.

## Reglas {#rules}

| Regla | Valor |
| --- | --- |
| Tipo de archivo | `.csv` (separador: coma) o `.tsv` (separador: tabulador). La importación decide por el final del nombre del archivo. Cualquier otro nombre se lee como `.csv`. |
| Encabezado | La primera fila. Es obligatorio. |
| Codificación | Use UTF-8. |
| Nombres de columna | Solo las letras `a-z` y `A-Z`, los dígitos, `.`, `_` y `-`. Otro carácter detiene la importación. |
| Nombres reservados | `sequent_internal_user_id` no puede ser un nombre de columna. |
| Columnas duplicadas | Dos encabezados que son iguales después de que la importación cambie `.` y `-` por `_` e ignore mayúsculas y minúsculas detienen la importación con `Duplicate column`. Por ejemplo, `area-id` y `area_id`, o `Email` y `email`. |
| Varios valores | Separe los valores de un atributo con `\|`. |
| Comillas | Si un valor tiene una coma (o un tabulador en un archivo `.tsv`), ponga el valor entre comillas dobles. |
| Tamaño | El código de la importación no comprueba el tamaño del archivo. |
| Orden de las filas | No es importante. |

:::note NOTA: columnas opcionales del archivo de votantes
Las columnas `password`, `hashed_password`, `password_salt`, `num_of_iterations`, `group_name`,
`email_verified` y `sequent_internal_user_id` nunca se guardan como atributos. Todas las demás
columnas que no son `username`, `email`, `first_name`, `last_name`, `enabled` y `election__...` se
convierten en atributos.
:::


## Archivo de ejemplo {#example-file}

```csv
username,email,first_name,last_name,area_name,enabled,group_name,password,vote-weight,authorized-election-ids,sequent.read-only.mobile-number,dateOfBirth
voter001,voter001@example.com,Ana,Lopez,North,TRUE,voter,Example-Pass-1,1,,+10000000001,1990-01-31
voter002,voter002@example.com,Ben,Smith,South,TRUE,voter,Example-Pass-2,3,election-1|election-2,+10000000002,1985-06-15
voter003,voter003@example.com,Eva,Brown,North,FALSE,voter,,,,,
```

El primer votante vota con peso 1 en todas las elecciones del área North. El segundo votante vota con
peso 3 en `election-1` y `election-2`. El tercer votante está deshabilitado y no tiene contraseña.

## Qué hace la importación {#what-the-import-does}

1. El portal de administración sube el archivo. Después, le pide la **Verificación de Integridad (SHA 256)**.
   Si hace clic en **Importar** con el campo vacío, el portal pregunta **Importar Sin Verificación de Integridad?**.
   Haga clic en **Sí, Importar Sin Verificación de Integridad** o en **Volver**.
2. El portal inicia la tarea **Importar usuarios** y la muestra en el widget de tareas. El mensaje
   **Importación de Votantes lanzada en segundo plano con éxito.** no es un resultado de la
   importación. Solo confirma que la tarea ha empezado.
3. La tarea comprueba el hash. Comprueba el encabezado. Comprueba el permiso para las columnas secretas.
4. La tarea crea todos los votantes en una sola transacción de base de datos. Si una fila es incorrecta,
   la tarea no hace ningún cambio. El estado de la tarea es **Failed** y el registro de la tarea tiene
   el motivo. Corrija el archivo e impórtelo de nuevo.
5. Después de una buena importación, los votantes aparecen en la pestaña **Votantes**. La importación no
   envía correos electrónicos ni mensajes.

Si el archivo tiene al menos una columna que se convierte en atributo (por ejemplo `area_name`), la
importación también da a cada votante el atributo `tenant-id`.

Un archivo sin una columna de este tipo no da a los votantes el atributo `tenant-id`.

### Mensajes y errores {#messages-and-errors}

| Mensaje | Dónde | Significado y acción |
| --- | --- | --- |
| Importación de Votantes lanzada en segundo plano con éxito. | Notificación | La tarea ha empezado. Abra la tarea y espere a **Success**. |
| Error Importando Votantes. | Notificación | El portal no pudo iniciar la tarea. Inténtelo de nuevo. |
| Error subiendo el fichero | Notificación | La subida falló. Seleccione el archivo de nuevo. |
| Fichero subido al servidor - pero no importado aún | Notificación | El archivo está en el servidor. Haga clic en **Importar**. |
| `Failed to verify the integrity: Hash of voters file: ... does not match with the input hash: ...` | Registro de la tarea | El SHA-256 que escribió no es el hash del archivo. Compruebe el archivo y el hash. |
| `CSV Header contains characters not allowed: <name>` | Registro de la tarea | Un nombre de columna tiene un carácter que no está permitido. |
| `CSV header sequent_internal_user_id is reserved` | Registro de la tarea | Cambie el nombre de la columna. |
| `Duplicate column <name> in the import file: two headers map to the same field` | Registro de la tarea | Quite la columna duplicada. |
| `Column <name> is not recognised. The per-voter vote weight column is spelled exactly vote-weight ...` | Registro de la tarea | Cambie el nombre de la columna a `vote-weight`. |
| `Invalid vote-weight value "<value>" on row <n>: must be between 1 and 100000` | Registro de la tarea | Corrija el valor de esa fila. El número de fila cuenta el encabezado como fila 1. |
| `Importing encrypted voter attributes requires voter-secret-attribute-write: <columns>` | Registro de la tarea | No tiene el permiso para escribir atributos secretos. Pida ayuda a un administrador, o quite las columnas. |
| `Voter secret attribute <name> exceeds the 150-byte plaintext limit` | Registro de la tarea | Acorte el valor. |
| `Password salt empty` | Registro de la tarea | Añada la columna `password_salt`. |
| `Error executing INSERT USER transaction` | Registro de la tarea | La base de datos rechazó una fila. El mensaje no dice qué fila ni qué regla falló. Motivos habituales: un nombre de usuario que ya existe, una dirección de correo electrónico que ya existe (si el realm no permite correos duplicados), o un valor en `enabled` o `email_verified` que no es `TRUE` ni `FALSE`. |


## Páginas relacionadas {#related-pages}

- [Añadir los votantes](../../03-procedures/03-voters.md): los pasos para importar el archivo.
- [Votantes](../02-election-event/05-election_management_election-event_voters.md): la pestaña **Votantes**.
- [Archivo de áreas](04-areas.md): las áreas deben existir antes de la importación de votantes.
- [Formatos de archivo de importación](index.md): todos los archivos de importación.
