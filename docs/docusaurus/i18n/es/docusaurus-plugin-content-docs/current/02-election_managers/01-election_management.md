---
id: election_management
title: Gestión electoral
sidebar_position: 0
description: "Este manual explica a los administradores electorales y a los fideicomisarios cómo preparar, ejecutar y cerrar una elección con el portal de administración de Sequent Online Voting, versión 10.0."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

import ManualPdfLink from '@site/src/components/ManualPdfLink';

# Gestión electoral

Este manual explica a los administradores electorales y a los fideicomisarios cómo preparar, ejecutar y cerrar una elección con el portal de administración de Sequent Online Voting, versión **10.0**.

<ManualPdfLink />

## Cómo usar este manual {#how-to-use-this-manual}

1. Lea primero [Antes de empezar](00-before-you-start.md). Explica los roles, los términos y las reglas de seguridad.
2. Haga los procedimientos en el orden de la tabla. Cada procedimiento indica lo que debe tener antes de empezar.
3. Use la parte de [Tutoriales](01-tutorials/07-admin_portal_tutorials_create-voters.md) cuando necesite una tarea en detalle: todas sus opciones, estados y errores.
4. Use la parte de [Referencia](02-reference/02-election-event/01-election_management_election-event_dashboard.md) cuando necesite los detalles de una pantalla.

## Procedimientos {#procedures}

| Paso | Procedimiento | Contenido | Quién | Resultado |
| --- | --- | --- | --- | --- |
|  | [Antes de empezar](00-before-you-start.md) | Roles, términos y reglas de seguridad | Todos | Sabe quién hace cada tarea. |
| 1 | [Configurar el inquilino](03-procedures/01-tenant.md) | Configuración, usuarios, roles y fideicomisarios | Administrador del inquilino | Los administradores y los fideicomisarios pueden iniciar sesión. |
| 2 | [Crear el proceso electoral](03-procedures/02-event.md) | Elecciones, preguntas, candidatos y áreas | Administrador electoral | El contenido de la papeleta está completo. |
| 3 | [Añadir los votantes](03-procedures/03-voters.md) | Importar, añadir y comprobar los votantes | Administrador electoral | La lista de votantes está completa y es correcta. |
| 4 | [Realizar la ceremonia de claves](03-procedures/04-keys.md) | Las claves de la elección | Administrador electoral y fideicomisarios | Existe la clave pública de la elección. Cada fideicomisario guarda un fragmento de clave. |
| 5 | [Publicar y gestionar el período de votación](03-procedures/05-publish.md) | Publicar la papeleta; abrir, pausar y cerrar la votación | Administrador electoral | Los votantes pueden votar. Después se cierra el período de votación. |
| 6 | [Realizar la ceremonia de recuento](03-procedures/06-tally.md) | Descifrar y contar los votos | Administrador electoral y fideicomisarios | Los votos se descifran y se cuentan. |
| 7 | [Obtener los resultados](03-procedures/07-results.md) | Resultados, informes y el sitio web de resultados | Administrador electoral | Tiene los documentos de resultados. El público puede ver los resultados. |

## Versiones e idiomas {#versions-and-languages}

Este manual corresponde a la versión de la plataforma que aparece en el menú de versiones, en la parte superior de la página. Para leer el manual de otra versión, selecciónela en el menú de versiones. Si no sabe la versión de su plataforma, pregunte al soporte de Sequent.

Use el menú de idiomas para seleccionar un idioma. Una página que no está traducida se muestra en inglés.
