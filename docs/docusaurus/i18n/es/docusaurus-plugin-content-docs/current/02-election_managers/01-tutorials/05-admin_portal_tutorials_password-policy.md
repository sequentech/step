---
id: admin_portal_tutorials_password-policy
title: Configurar la política de contraseñas
description: "Cada proceso electoral tiene su propio realm de Keycloak y su propia política de contraseñas."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

Cada proceso electoral tiene su propio realm de Keycloak y su propia política de contraseñas. Los administradores electorales pueden ver y actualizar las partes gestionadas de esa política directamente desde el portal de administración, en lugar de editar el realm en la consola de administración de Keycloak.

La política se aplica cuando un administrador cambia la contraseña de un votante y también se usa para generar la credencial del votante que se incluye en una carta de información para el votante.

## Permisos necesarios {#required-permissions}

| Operación                            | Permisos necesarios       |
| ------------------------------------ | ------------------------- |
| Abrir la pestaña **Datos** del proceso electoral | `election-event-data-tab` |
| Ver la política actual               | `election-event-read`     |
| Cambiar y guardar la política        | `election-event-write`    |

Un administrador completo normalmente tiene estos permisos. Los roles personalizados con permisos mínimos deben incluirlos de forma explícita.

## Configurar la política {#configure-the-policy}

1. En el portal de administración, seleccione el proceso electoral.
2. Abra la pestaña **Datos**.
3. Expanda **Password Policy**.
4. Establezca la longitud mínima y la longitud máxima.
5. Seleccione las clases de caracteres que debe contener cada contraseña nueva.
6. Seleccione **Guardar** para actualizar el realm del proceso electoral.

Pase el cursor sobre el icono de información junto a un campo para ver su significado.

| Campo                          | Significado                                                 |
| ------------------------------ | ----------------------------------------------------------- |
| **Longitud mínima**            | El número mínimo de caracteres que requiere la contraseña.  |
| **Longitud máxima**            | El número máximo de caracteres que se permite en la contraseña. |
| **Incluir letras mayúsculas**  | Exige al menos un carácter en mayúscula.                    |
| **Incluir letras minúsculas**  | Exige al menos un carácter en minúscula.                    |
| **Incluir dígitos**            | Exige al menos un dígito numérico.                          |
| **Incluir caracteres especiales** | Exige al menos un carácter especial admitido.            |

Las longitudes deben ser números enteros del 1 al 256, y el mínimo no puede superar al máximo. Al seleccionar una clase de caracteres, se exige al menos un carácter de esa clase. Al desmarcarla, se elimina ese requisito; no prohíbe que esa clase de caracteres aparezca en una contraseña.

:::info Política predeterminada

Si el realm del proceso electoral todavía no tiene una política de contraseñas, el portal de administración muestra los valores predeterminados: 12 caracteres como mínimo, 72 caracteres como máximo y las cuatro clases de caracteres seleccionadas. Al guardar, se aplican esos valores predeterminados al realm.

:::

## Qué cambia {#what-changes}

Al guardar se actualizan estas reglas de Keycloak en el realm del proceso electoral:

- `length`
- `maxLength`
- `upperCase`
- `lowerCase`
- `digits`
- `specialChars`

Las demás reglas de la política de contraseñas de Keycloak que no gestiona este formulario se conservan.

La actualización afecta a las contraseñas que se establezcan después de guardar la política. No sustituye automáticamente las contraseñas existentes de los votantes. Si un administrador escribe una contraseña que no cumple la política en **Votantes > Cambiar contraseña**, el cuadro de diálogo permanece abierto, marca como no válidos ambos campos de contraseña y explica que la contraseña debe corregirse.

:::warning Cartas de información para el votante

No se puede generar una carta de información para el votante hasta que el proceso electoral tenga una política de contraseñas configurada. Cada generación de carta crea una credencial de votante nueva que cumple la política actual.

:::
