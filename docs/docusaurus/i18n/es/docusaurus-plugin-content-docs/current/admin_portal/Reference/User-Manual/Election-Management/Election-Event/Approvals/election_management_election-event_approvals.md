---
id: election_management_election_event_approvals
title: Aprobaciones
description: "La pestaña Aprobaciones lista las solicitudes de inscripción del proceso electoral. Cuando los votantes se inscriben en el portal de votación, cada solicitud recibe el estado Pending, Accepted o Rejected."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

La pestaña **Aprobaciones** lista las solicitudes de inscripción del proceso electoral. Cuando los
votantes se inscriben en el portal de votación, cada solicitud recibe el estado **Pending**
(pendiente), **Accepted** (aceptada) o **Rejected** (rechazada). En esta pestaña usted comprueba
una solicitud, busca el votante que coincide y la aprueba o la rechaza.

La pestaña solo es útil cuando **Inscripción** está en **Habilitado** en la pestaña **Datos**, en
**Voting Portal Countdown Policy** (configuraciones avanzadas) > **Autenticación de votantes**.

## Abrir la pestaña {#open-the-tab}

1. En el menú de la izquierda, haga clic en el proceso electoral.
2. Haga clic en la pestaña **Aprobaciones**.

La pestaña se muestra con el permiso **Ver Aprobaciones del Evento Electoral**. Está oculta cuando
el proceso electoral está confinado. Usted solo ve las solicitudes de las elecciones que permiten
sus etiquetas de permiso.

## Barra de herramientas {#toolbar}

| Botón | Uso | Permiso |
| --- | --- | --- |
| **Añadir filtro** | Filtra por **Estado**, **Tipo de Verificación** (**Manual** o **Automatic**), **ID del Solicitante**, **ID** y los datos del solicitante. | - |
| **Importar** | Abre **Importar Aplicaciones** para importar solicitudes desde un archivo CSV. | **Importación de Aplicaciones** |
| **Exportar** | Abre **Exportar Aplicaciones** para exportar las solicitudes como archivo CSV. | **Exportación de Aplicaciones** |

## Archivo de importación {#import-file}

Importe solo un archivo que hizo con **Exportar**. No haga el archivo a mano. La importación añade
cada fila como una solicitud nueva.

El archivo tiene una fila de encabezado y doce columnas en este orden:

```csv
id,created_at,updated_at,tenant_id,election_event_id,area_id,applicant_id,applicant_data,labels,annotations,verification_type,status
```

| Columna | Contenido |
| --- | --- |
| `id` | El ID de la solicitud. La importación da a cada solicitud un ID nuevo. |
| `created_at`, `updated_at` | Fechas. |
| `tenant_id`, `election_event_id` | El tenant y el evento electoral. La importación añade la solicitud a ellos. |
| `area_id` | El área del solicitante. |
| `applicant_id` | El identificador del votante vinculado. |
| `applicant_data` | Los datos que escribió el votante, en JSON. |
| `labels`, `annotations` | Datos adicionales. Puede dejarlos vacíos. |
| `verification_type` | Manual o automática. |
| `status` | Pendiente, aceptada o rechazada. |

:::caution PRECAUCIÓN
La importación usa el ID de tenant y el ID de evento electoral de cada fila del archivo, no el
evento electoral que tiene abierto. Las filas con un ID de tenant no válido se omiten. Importe un
archivo solo en el evento electoral donde lo exportó.
:::

## Columnas {#columns}

| Columna | Contenido |
| --- | --- |
| **ID** | El identificador de la solicitud. |
| **Creado El** y **Actualizado El** | Las fechas de la solicitud. |
| **ID del Solicitante** | El identificador del solicitante, o `-`. |
| **Tipo de Verificación** | `MANUAL` o `AUTOMATIC`. |
| **Estado** | El estado de la solicitud. |
| **Aprobado Por** | El administrador que aprobó o rechazó la solicitud. |
| Datos del solicitante | Los datos que escribió el votante, una columna para cada campo. |
| **Acciones** | El icono de ver. Abre la solicitud. |

## Revisar una solicitud {#review-an-application}

1. En la fila de la solicitud, haga clic en el icono de ver.
2. Lea **Solicitud de Aprobación**: los datos del solicitante.
3. Mire la lista de votantes de debajo. La lista muestra los votantes que coinciden con los datos.
4. Para aprobar, haga clic en el icono de aprobar en la fila del votante correcto. Lea "¿Estás
   seguro de que deseas aprobar a este votante? Esta acción no se puede deshacer." Haga clic en
   **Aprobar**.
5. Para rechazar, haga clic en **Rechazar la solicitud**. Seleccione el **Motivo del rechazo**. Si
   selecciona **Otro**, escriba el motivo. Confirme la acción.

**Resultado esperado:** se muestra el mensaje "Votante aprobado" o "Votante rechazado".

| **Motivo del rechazo** | Uso |
| --- | --- |
| **Datos Faltantes** | La solicitud no tiene suficientes datos. |
| **Votante no Coincidente** | Ningún votante coincide con la solicitud. |
| **Ya Aprobado** | El votante ya está aprobado. |
| **Otro** | Otro motivo. Debe escribirlo. |

:::caution PRECAUCIÓN
No puede deshacer una aprobación ni un rechazo. Compruebe los datos antes de confirmar.
:::

## Si hay un problema {#if-there-is-a-problem}

| Problema | Acción |
| --- | --- |
| "El votante ya está aprobado." | El votante se aprobó antes. No es necesaria ninguna acción. |
| "Se requiere un mensaje de rechazo para la opción 'Otro'" | Escriba el motivo del rechazo. |
| "Error al aprobar al votante" o "Error al rechazar al votante" | Inténtelo de nuevo. Si el error continúa, contacte con el soporte de Sequent. |
| La lista está vacía. | Compruebe el filtro **Estado**. La pestaña recuerda el último estado que seleccionó. |

## Páginas relacionadas {#related-pages}

- [Elección: Aprobaciones](../../Election/Approvals/election_management_election_approvals.md)
- [Datos](../Data/election_management_election-event_data.md)
- [Votantes](../Voters/election_management_election-event_voters.md)
- [Monitoreo](../Monitoring/election_management_election-event_monitoring.md)
