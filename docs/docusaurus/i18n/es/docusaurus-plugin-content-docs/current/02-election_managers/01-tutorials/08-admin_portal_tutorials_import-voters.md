---
id: admin_portal_tutorials_import-voters
title: Importar la lista de votantes
description: "Importe la lista de votantes de un proceso electoral desde un archivo CSV: las columnas, los formatos y las comprobaciones."
---

<!--
SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

## Preparar el CSV {#prepare-the-csv}

Use una fila por votante. Los nombres de las columnas deben coincidir con los nombres canónicos de los atributos del votante y del perfil de usuario de Keycloak; la etiqueta que se muestra en el portal de administración no es el encabezado de importación. Por ejemplo, un atributo llamado `customerReference` usa este encabezado:

```csv
username,email,customerReference
voter-001,voter-001@example.com,REF-12345
```

Use `|` entre los valores de un atributo de perfil de usuario con varios valores. Mantenga el CSV de origen en privado: contiene los valores en texto plano, incluso cuando una columna está configurada como secreta.

## Importar los votantes {#import-the-voters}

1. Abra el proceso electoral y seleccione la pestaña **Votantes**.
2. Seleccione la acción de importar votantes y suba el CSV.
3. Revise el resultado de la tarea y corrija los errores de fila o de encabezado que se indiquen.

## Importar campos secretos del votante {#importing-secret-voter-fields}

Un CSV puede contener un campo configurado con `sequent.secret=true`. Introduzca el valor normal en texto plano; no lo cifre previamente y no copie un valor cifrado `seqenc:` de Keycloak. El proceso de importación crea el ID del votante y cifra cada celda secreta no vacía antes de escribirla en Keycloak.

El usuario que inicia la importación necesita `voter-secret-attribute-write`, además del acceso normal de importación y creación de votantes. Si el CSV contiene un encabezado secreto y el usuario no tiene ese permiso, se rechaza toda la importación y el error indica las columnas afectadas.

Después de la importación:

- las vistas y exportaciones ordinarias de votantes no devuelven el valor secreto ni su sobre cifrado;
- los usuarios con permiso de lectura de secretos pueden mostrarlo desde el editor de votantes;
- una exportación descifrada puede incluirlo solo cuando lo solicita explícitamente un usuario autorizado.

Consulte [Proteger un atributo del votante como secreto](./99-admin_portal_tutorials_add-user-attributes-to-keycloak.md#protecting-a-voter-attribute-as-secret) para ver las restricciones de configuración, y [Permisos](../02-reference/user-manual/users-and-roles/users-and-roles_permissions.md#secret-voter-field-permissions) para ver los detalles de autorización.
