---
id: users-and-roles_permissions
title: Permisos
description: "Los permisos se asignan a los roles en Usuarios y Roles > Roles. Un usuario recibe los permisos de los roles que tiene asignados."
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

Los permisos se asignan a los roles en **Usuarios y Roles** > **Roles**. Un usuario recibe los permisos de los roles que se le asignan. Use el rol más restrictivo que cubra el trabajo del usuario; tener acceso a un proceso electoral no concede automáticamente acceso a todas las acciones ni a todos los campos del mismo.

## Permisos de los campos secretos del votante {#secret-voter-field-permissions}

Los campos secretos del votante tienen permisos de lectura y de escritura independientes, para que un operador pueda gestionar un valor sin necesariamente poder verlo.

| Permiso | Permite |
|---|---|
| `voter-secret-attribute-read` | Mostrar explícitamente un valor secreto, incluir campos secretos descifrados en una exportación de votantes, usar variables secretas declaradas en comunicaciones o informes a nivel de votante, y descargar exportaciones de votantes descifradas de acceso restringido. |
| `voter-secret-attribute-write` | Establecer, reemplazar, borrar o importar un valor secreto de votante. No permite mostrar un valor existente. |

Estos permisos complementan el permiso ordinario de la operación:

| Operación | Permisos necesarios |
|---|---|
| Mostrar un valor | Acceso de lectura de votantes y `voter-secret-attribute-read` |
| Crear o editar un valor secreto | El acceso correspondiente de creación o escritura de votantes y `voter-secret-attribute-write` |
| Importar un CSV que contenga una columna secreta | Acceso de importación o creación de votantes y `voter-secret-attribute-write` |
| Exportar columnas secretas descifradas | Acceso normal de exportación de votantes y `voter-secret-attribute-read` |
| Enviar o generar una salida que declare campos secretos | El permiso normal de comunicación o de informe y `voter-secret-attribute-read` |

Una lista o exportación estándar de votantes nunca necesita ninguno de los dos permisos secretos, porque las columnas secretas se omiten. Un documento de exportación de votantes descifrado sigue siendo de acceso restringido: al descargarlo se vuelve a comprobar `voter-secret-attribute-read`.

Asigne los permisos de lectura y de escritura de forma independiente cuando las funciones lo requieran. Por ejemplo, un operador de importación puede recibir escritura secreta sin lectura secreta, mientras que un operador de soporte que deba revisar un valor puede recibir lectura secreta sin permiso para cambiarlo.
