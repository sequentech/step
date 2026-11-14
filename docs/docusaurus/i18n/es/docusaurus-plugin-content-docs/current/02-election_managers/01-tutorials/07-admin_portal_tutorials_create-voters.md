---
id: admin_portal_tutorials_create-voters
title: Crear votantes
description: "Cree votantes uno a uno en el portal de administración, proteja los campos secretos del votante y exporte los votantes."
---

<!--
SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

## Crear un votante {#create-a-voter}

1. Abra el proceso electoral y seleccione la pestaña **Votantes**.
2. Seleccione **Añadir**.
3. Complete los campos del votante. Los campos personalizados y su orden provienen del perfil de usuario de Keycloak del proceso electoral.
4. Revise los cambios y guarde el votante.

Para obtener instrucciones sobre cómo definir campos personalizados, consulte [Añadir atributos de usuario a Keycloak](./99-admin_portal_tutorials_add-user-attributes-to-keycloak.md).

## Campos secretos del votante {#secret-voter-fields}

Los campos configurados con `sequent.secret=true` se almacenan cifrados. No están disponibles como columnas, filtros ni campos de ordenación de la lista de votantes, y su valor almacenado nunca se carga con el registro ordinario del votante.

Los permisos para mostrar y editar estos campos son independientes:

- Con `voter-secret-attribute-write`, escriba un valor al crear un votante o escriba un valor nuevo para reemplazarlo al editar uno. No es necesario mostrar el valor anterior para sustituirlo.
- Con `voter-secret-attribute-read`, haga clic en el botón del ojo para mostrar el valor. El botón está deshabilitado mientras se descifra el valor. Haga clic de nuevo para ocultarlo.
- Ambas operaciones también requieren el permiso correspondiente de lectura o escritura ordinaria de votantes.

Al editar un votante, un valor existente que no se ha mostrado aparece como `••••••••`. Si lo deja sin cambios, se conserva el valor cifrado. Use el icono **Borrar** (×) a la derecha del campo y guarde para eliminar un valor sin mostrarlo. Los campos obligatorios siguen necesitando un valor. En los campos multivalor, cada valor se puede editar, añadir o eliminar por separado. Las pantallas de revisión ocultan los valores secretos nuevos y modificados en lugar de repetirlos.

Un secreto también puede usarse como credencial de inicio de sesión con el [Multi-Attribute + Password Form](./101-admin_portal_tutorials_multi-attribute-password-login.md#optional-use-an-encrypted-voter-attribute-as-the-credential). En ese modo, rellene el campo secreto configurado (por ejemplo, **Mysecret**). Los controles de contraseña de Keycloak y de restablecimiento de contraseña no establecen ni restablecen ese secreto.

Los valores revelados se limitan al editor de votante abierto. Al cerrar el editor, se eliminan de su estado; la lista de votantes sigue recibiendo solo valores ocultos.

## Exportar votantes {#export-voters}

Seleccione **Exportar** en la pestaña **Votantes** para generar un CSV.

- La exportación predeterminada omite todas las columnas secretas configuradas. No exporta los sobres cifrados.
- Un usuario con `voter-secret-attribute-read` puede seleccionar **Incluir campos secretos descifrados del votante** en el cuadro de diálogo de confirmación. Así se añaden las columnas secretas con valores en texto plano.
- Trate una exportación descifrada como datos sensibles. El documento generado sigue protegido por el permiso de lectura de secretos cuando se descarga.

Para ver todas las combinaciones de permisos, consulte [Permisos](../02-reference/user-manual/users-and-roles/users-and-roles_permissions.md#secret-voter-field-permissions).
