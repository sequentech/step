---
id: election_management_election_event_approvals
title: Aprobaciones
description: "La pestaña Aprobaciones muestra las solicitudes de inscripción del proceso electoral. Un votante envía una solicitud cuando se inscribe en la página de inscripción de votantes."
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
-- SPDX-License-Identifier: AGPL-3.0-only
-->

# Aprobaciones

La pestaña **Aprobaciones** muestra las solicitudes de inscripción del proceso electoral. Un votante envía una solicitud cuando se inscribe en la página de inscripción de votantes. La plataforma puede aceptar o rechazar algunas solicitudes de forma automática. Un administrador electoral revisa las demás solicitudes en esta pestaña. Para aprobar una solicitud, vincúlela con un votante de la lista de votantes. Para rechazar una solicitud, indique un motivo.

:::note
La pestaña **Aprobaciones** es solo para la inscripción de votantes. No aprueba las acciones de otros administradores.
:::

## Abrir la pestaña {#open-the-tab}

1. Abra el proceso electoral.
2. Haga clic en la pestaña **Aprobaciones**.

La pestaña solo aparece si se cumplen estas condiciones:

- Su rol tiene permiso para ver la pestaña de aprobaciones del proceso electoral.
- El proceso electoral no está confinado. Consulte **Estado de Confinamiento** en la pestaña **Datos**.

Para aprobar o rechazar una solicitud, su rol también debe tener permiso para cambiar las solicitudes. Los botones **Importar** y **Exportar** solo aparecen si su rol tiene permiso para importar o exportar solicitudes.

Cada elección también tiene una pestaña **Aprobaciones**. Muestra una parte de estas solicitudes. Consulte [Aprobaciones de la elección](../03-election/06-election_management_election_approvals.md).

## Lista de solicitudes {#application-list}

La lista muestra 10 solicitudes por página, de la más reciente a la más antigua. Al abrir la pestaña, la lista muestra las solicitudes con el estado **Pendiente**.

| Columna | Significado |
| --- | --- |
| **ID** | El identificador de la solicitud. |
| **Creado El** | La fecha y la hora en que el votante envió la solicitud. |
| **Actualizado El** | La fecha y la hora del último cambio en la solicitud. |
| **ID del Solicitante** | El identificador del votante al que está vinculada la solicitud. Aparece un guion (-) si no hay ningún votante. |
| **Tipo de Verificación** | `AUTOMATIC` si la plataforma tomó la decisión. `MANUAL` si la decisión la tomó un administrador. |
| **Estado** | `PENDING`, `ACCEPTED` o `REJECTED`. |
| **Aprobado Por** | El nombre del administrador que aprobó o rechazó la solicitud. Aparece un guion (-) si nadie lo hizo. |
| Atributos del votante | Una columna por cada atributo del perfil del votante, con el valor que escribió el votante. El nombre, los apellidos, el correo electrónico, el nombre de usuario y la fecha de nacimiento aparecen por defecto. Use **Columnas** para mostrar los demás atributos. |
| **Acciones** | El icono de vista. Abre la solicitud. |

### Filtros {#filters}

Haga clic en **Añadir filtro** para añadir un filtro.

| Filtro | Uso |
| --- | --- |
| **Estado** | Seleccione **Pendiente**, **Aceptado** o **Rechazado**. El portal de administración guarda su selección en este navegador. |
| **Tipo de Verificación** | Seleccione **Manual** o **Automático**. |
| **ID del Solicitante** | Escriba el identificador del votante vinculado. |
| **ID** | Escriba el identificador de la solicitud. |
| Atributos del votante | Un filtro por cada atributo del perfil del votante. |

### Botones {#buttons}

| Botón | Uso |
| --- | --- |
| **Columnas** | Muestra u oculta columnas. |
| **Añadir filtro** | Añade un filtro a la lista. |
| **Importar** | Importa solicitudes desde un archivo CSV. Se abre el panel **Importar Aplicaciones**. |
| **Exportar** | Exporta todas las solicitudes del proceso electoral al archivo `export-applications.csv`. |

## Archivo de importación {#import-file}

Importe solo un archivo que haya creado con **Exportar**. No cree el archivo a mano. La importación añade cada fila como una solicitud nueva.

El archivo tiene una fila de encabezado y doce columnas, en este orden:

```csv
id,created_at,updated_at,tenant_id,election_event_id,area_id,applicant_id,applicant_data,labels,annotations,verification_type,status
```

| Columna | Contenido |
| --- | --- |
| `id` | El ID de la solicitud. La importación da a cada solicitud un ID nuevo. |
| `created_at`, `updated_at` | Fechas. |
| `tenant_id`, `election_event_id` | El inquilino y el proceso electoral. La importación añade la solicitud a ellos. |
| `area_id` | El área del solicitante. |
| `applicant_id` | El identificador del votante vinculado. |
| `applicant_data` | Los datos que escribió el votante, en JSON. |
| `labels`, `annotations` | Datos adicionales. Puede dejarlos vacíos. |
| `verification_type` | Manual o automático. |
| `status` | Pendiente, aceptado o rechazado. |

:::caution PRECAUCIÓN
La importación usa el ID del inquilino y el ID del proceso electoral de cada fila del archivo, no el proceso electoral que tiene abierto. Las filas con un ID de inquilino no válido se omiten. Importe un archivo solo en el proceso electoral donde lo exportó.
:::

## Página de la solicitud {#application-page}

Al hacer clic en el icono de vista, se abre la página de la solicitud. Tiene dos partes:

- **Solicitud de Aprobación**: una tabla con los datos que escribió el votante. Si la solicitud está rechazada, la tabla también muestra el **Motivo del rechazo**.
- **Votantes** ("Buscar votantes coincidentes"): la lista de votantes del proceso electoral. El portal de administración rellena los filtros con los datos de la solicitud, para que la lista muestre los votantes que coinciden. Puede cambiar los filtros.

| Control | Uso |
| --- | --- |
| **Rechazar la solicitud** | Rechaza la solicitud. Solo aparece cuando el estado es `PENDING`. |
| Icono de aprobación (**Aprobar**) | En la columna **Acciones** de un votante. Aprueba la solicitud y la vincula con este votante. Solo aparece cuando el estado es `PENDING` o `REJECTED`. |
| **Atrás** | Vuelve a la lista de solicitudes. |

## Aprobar una solicitud {#approve-an-application}

Al aprobar una solicitud, la plataforma hace lo siguiente:

- Vincula la solicitud con el votante que selecciona.
- Cambia el estado a `ACCEPTED` y el tipo de verificación a `MANUAL`.
- Copia los atributos configurados de la solicitud al votante.
- Envía al votante el mensaje de respuesta configurado.

:::caution PRECAUCIÓN
Asegúrese de seleccionar el votante correcto antes de aprobar. No puede deshacer una aprobación.
:::

1. En la pestaña **Aprobaciones**, haga clic en el icono de vista de la solicitud.
2. Compare los datos de **Solicitud de Aprobación** con los votantes de la lista **Votantes**.
3. Si no aparece ningún votante, cambie los filtros de la lista **Votantes**.
4. En la fila del votante correcto, haga clic en el icono de aprobación.
5. Lea el mensaje "¿Estás seguro de que deseas aprobar a este votante? Esta acción no se puede deshacer."
6. Haga clic en **Aprobar**.

**Resultado esperado:** aparece el mensaje "Votante aprobado". El portal de administración abre la lista de solicitudes.

## Rechazar una solicitud {#reject-an-application}

:::caution PRECAUCIÓN
Asegúrese de que la solicitud no es válida antes de rechazarla. La plataforma envía el mensaje de rechazo al votante de inmediato.
:::

1. En la pestaña **Aprobaciones**, haga clic en el icono de vista de la solicitud.
2. Haga clic en **Rechazar la solicitud**.
3. Lea el mensaje "¿Estás seguro de que deseas rechazar a este votante? Esta acción no es reversible."
4. En **Motivo del rechazo**, seleccione el motivo. La tabla indica los valores.
5. Si el motivo es **Otro**, escriba el motivo en **Escribe aquí el motivo del rechazo**.
6. Haga clic en **Rechazar la solicitud**.

| **Motivo del rechazo** | Uso |
| --- | --- |
| **Datos Faltantes** | La solicitud no tiene suficientes datos. |
| **Votante no Coincidente** | Ningún votante de la lista coincide con la solicitud. |
| **Ya Aprobado** | El votante ya está aprobado. |
| **Otro** | Otro motivo. Debe escribirlo. |

**Resultado esperado:** aparece el mensaje "Votante rechazado". El portal de administración abre la lista de solicitudes. La plataforma envía al votante el mensaje de respuesta configurado.

:::note
El portal de administración muestra también el icono de aprobación para una solicitud rechazada. Si rechazó una solicitud por error, ábrala y apruébela. Consulte [Aprobar una solicitud](#approve-an-application).
:::

## Exportar las solicitudes {#export-the-applications}

1. En la pestaña **Aprobaciones**, haga clic en **Exportar**.
2. Lea el mensaje "Exportar puede ser una operación larga. ¿Está seguro de que desea exportar los registros?".
3. Haga clic en **Exportar**.
4. Espere hasta que empiece la descarga.

**Resultado esperado:** se descarga el archivo `export-applications.csv`. Aparece el mensaje "La exportación de aplicaciones se completó con éxito".

## Si hay un problema {#if-there-is-a-problem}

| Problema | Acción |
| --- | --- |
| No ve la pestaña **Aprobaciones**. | Asegúrese de que el proceso electoral no está confinado. Pida a su administrador de inquilino el permiso para ver la pestaña de aprobaciones. |
| "El votante ya está aprobado." | El votante seleccionado ya está aprobado. Seleccione otro votante, o rechace la solicitud con **Ya Aprobado**. |
| "Error al aprobar al votante" | Asegúrese de que su rol tiene permiso para cambiar las solicitudes. Inténtelo de nuevo. Si el problema continúa, contacte con el soporte de Sequent. |
| "Error al rechazar al votante" | Asegúrese de que su rol tiene permiso para cambiar las solicitudes. Inténtelo de nuevo. |
| "Se requiere un mensaje de rechazo para la opción 'Otro'" | Escriba el motivo en **Escribe aquí el motivo del rechazo**. |
| "Error al exportar las aplicaciones" | Inténtelo de nuevo. Mire la pestaña **Tareas** para ver el estado de la exportación. |
| "Error al importar las aplicaciones" | Asegúrese de que el archivo es un CSV. Mire la pestaña **Tareas** para ver el error. |
| La lista está vacía. | Cambie el filtro **Estado**. La lista muestra solo solicitudes **Pendientes** cuando la abre. |

## Páginas relacionadas {#related-pages}

- [Votantes](05-election_management_election-event_voters.md)
- [Tareas](10-election_management_election-event_tasks.md)
- [Crear el proceso electoral](../../03-procedures/02-event.md)
