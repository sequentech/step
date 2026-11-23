---
id: election_management
title: Gestión electoral
sidebar_position: 0
description: "Este manual explica a los administradores electorales y a los fideicomisarios cómo preparar, realizar y cerrar una elección con el portal de administración de Sequent Online Voting, versión 9.0."
---


<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

import ManualPdfLink from '@site/src/components/ManualPdfLink';

# Gestión electoral

Este manual explica a los administradores electorales y a los fideicomisarios cómo preparar,
realizar y cerrar una elección con el portal de administración de Sequent Online Voting,
versión **9.0**.

<ManualPdfLink />

## Cómo usar este manual {#how-to-use-this-manual}

1. Lea primero [Antes de empezar](00-before-you-start.md). Esta página da los roles, los términos y las reglas de seguridad.
2. Haga los procedimientos en el orden de la tabla. Cada procedimiento indica lo que debe tener
   antes de empezar.
3. Use la parte de [Referencia](Reference/User-Manual/admin_portal_reference_user-manual.md) cuando
   necesite los detalles de una pantalla. Empiece por [Navegación básica](Reference/basic_navigation.md).
4. Use los Tutoriales para estudiar una tarea en detalle, por ejemplo
   [Configurar su primera elección](Tutorials/admin_portal_tutorials_setting-up-your-first-election.md).

## Procedimientos {#procedures}

| Paso | Procedimiento | Quién | Resultado |
| --- | --- | --- | --- |
| - | [Antes de empezar](00-before-you-start.md): roles, términos y reglas de seguridad | Todos | Usted sabe quién hace cada tarea. |
| 1 | [Configurar el inquilino](procedures/01-tenant.md): configuración, usuarios, roles y fideicomisarios | Administrador del inquilino | Los administradores y los fideicomisarios pueden iniciar sesión. |
| 2 | [Crear el proceso electoral](procedures/02-event.md): elecciones, preguntas, candidatos y áreas | Administrador electoral | El contenido de la papeleta y las áreas están completos. |
| 3 | [Añadir los votantes](procedures/03-voters.md): importar, añadir y comprobar los votantes | Administrador electoral | La lista de votantes está completa. |
| 4 | [Realizar la ceremonia de claves](procedures/04-keys.md) | Administrador electoral y fideicomisarios | La clave pública de la elección existe. Cada fideicomisario guarda un fragmento de clave. |
| 5 | [Publicar y gestionar el período de votación](procedures/05-publish.md): publicar la papeleta y abrir, pausar y cerrar el período de votación | Administrador electoral | Los votantes pueden votar. Después, el período de votación queda cerrado. |
| 6 | [Realizar la ceremonia de recuento](procedures/06-tally.md) | Administrador electoral y fideicomisarios | Los votos se descifran y se cuentan. |
| 7 | [Obtener los resultados](procedures/07-results.md): los resultados y los informes | Administrador electoral | Usted tiene los documentos de resultados. |

## Versiones e idiomas {#versions-and-languages}

Este manual corresponde a la versión de la plataforma que aparece en el menú de versiones, en la
parte superior de la página. Para leer el manual de otra versión, selecciónela en el menú de
versiones. Si no conoce la versión de su plataforma, pregunte al soporte de Sequent.

Use el menú de idiomas para seleccionar un idioma. Una página que no está traducida se muestra en
inglés.
