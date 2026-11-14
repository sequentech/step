---
id: election_management_election_event_tally
title: Recuento
description: "La pestaña Recuento contiene las ceremonias de recuento y los informes de inicialización del proceso electoral. Una ceremonia de recuento descifra y cuenta los votos."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

La pestaña **Recuento** contiene las ceremonias de recuento y los informes de inicialización del
proceso electoral. Una ceremonia de recuento descifra y cuenta los votos. Los fideicomisarios suben
sus fragmentos de clave. Después el administrador electoral inicia el recuento y obtiene los
resultados. Para los procedimientos completos, consulte
[Realizar la ceremonia de recuento](../../../../../procedures/06-tally.md) y
[Obtener los resultados](../../../../../procedures/07-results.md).

## Abrir la pestaña {#open-the-tab}

1. En el menú de la izquierda, haga clic en el proceso electoral.
2. Haga clic en la pestaña **Recuento**.

La pestaña se muestra con el permiso **Ver Escrutinio del Evento Electoral** y uno de **Leer
Recuento** o **Iniciar Recuento**. Para crear un recuento, su rol necesita **Crear Ceremonia**.
Para gestionarlo, su rol necesita **Administrar Ceremonia de Claves**. Para descargar resultados,
su rol necesita **Exportar Ceremonia**. La pestaña está oculta cuando el proceso electoral está
confinado.

## Botones {#buttons}

| Botón | Uso |
| --- | --- |
| **Iniciar la Ceremonia de Recuento** | Crea un recuento del tipo **Resultados Electorales**. |
| **Crear Informe de Inicialización** | Crea un recuento del tipo **Resultados de Inicialización**. Muestra que la urna está vacía antes del período de votación. |

Los dos botones solo están disponibles cuando una ceremonia de claves tiene el estado `SUCCESS` y
la papeleta está publicada. Si no, la pestaña muestra uno de estos mensajes:

- "La Ceremonia de Recuento no puede comenzar hasta que la Ceremonia de Claves se haya completado
  con éxito."
- "La Ceremonia de Escrutinio no puede comenzar hasta que crees una publicación en la pestaña
  Publicar."

## Lista de recuentos {#list-of-tallies}

| Columna | Contenido |
| --- | --- |
| **Tipo de Conteo** | **Resultados Electorales** o **Resultados de Inicialización**. |
| **Etiquetas de Permisos** | Las etiquetas de permiso de las elecciones del recuento. |
| **Trustees** (fideicomisarios) | Los fideicomisarios del recuento. |
| **Número de Elecciones** | El número de elecciones del recuento. |
| **Estado** | El estado del recuento. La tabla de abajo da los valores. |
| **Acciones** | **Ver la Ceremonia de Cómputo**, **Cancelar la Ceremonia de Cómputo** y, para los fideicomisarios, **Agregar Clave de Cómputo**. |

| Estado | Significado |
| --- | --- |
| `STARTED` | El recuento está creado. Los fideicomisarios pueden subir sus fragmentos de clave. |
| `CONNECTED` | Suficientes fideicomisarios han subido su fragmento de clave. El administrador puede iniciar el recuento. |
| `IN_PROGRESS` | La plataforma mezcla, descifra y cuenta los votos. |
| `SUCCESS` | El recuento está completado. Los resultados están disponibles. |
| `CANCELLED` | El recuento está cancelado. |

**Cancelar la Ceremonia de Cómputo** solo está disponible antes de que empiece el recuento.
Pregunta "¿Estás seguro de que quieres cancelar el recuento?". Haga clic en **Cancelar recuento**
para confirmar. No puede deshacer una cancelación.

## Asistente de recuento (administrador) {#tally-wizard-administrator}

El asistente tiene cuatro pasos: **Inicio**, **Ceremonia**, **Recuento** y **Resultados**.

| Paso | Contenido |
| --- | --- |
| **Inicio** | **Elecciones para el Recuento**: las elecciones que se cuentan, con la columna **Seleccionadas**. **Ceremonia de Llaves**: la ceremonia de claves de estas elecciones. Haga clic en **Iniciar Ceremonia de Recuento** y después en **Ok**. |
| **Ceremonia** | **Trustee** (fideicomisarios): "Estado de importación del fragmento de clave", con el número de fideicomisarios que importaron la clave y el número necesario. Cuando el estado sea `CONNECTED`, haga clic en **Iniciar Recuento** y después otra vez en **Iniciar recuento** en la pregunta. |
| **Recuento** | **Progreso del Recuento de Elecciones**: cada elección con su **Estado** (`WAITING`, `MIXING`, `DECRYPTING`, `SUCCESS` o `ERROR`) y su **Progreso**. **Información General** y **Logs** (registros). |
| **Resultados** | **Resultados & Participación**: los resultados por elección, pregunta y área, y el menú de descarga. Consulte [Obtener los resultados](../../../../../procedures/07-results.md). |

**Iniciar Recuento** solo está disponible cuando todas las elecciones seleccionadas permiten el
recuento. **Permitir Recuento** de la elección debe ser **Permitido**, o **Permitir cuando termine
el periodo de votación** con el período de votación cerrado. Consulte
[Elección: Datos](../../Election/Data/election_management_election_data.md).

## Subir un fragmento de clave (fideicomisario) {#upload-a-key-fragment-trustee}

Un fideicomisario ve "Ha sido invitado a participar a una Ceremonia de Recuento.". El
fideicomisario hace clic en **Agregar Clave de Cómputo** en la fila del recuento, lee **Elecciones
para el Recuento** y sube el fragmento de clave en **Proceso del trustee**. Se muestra el mensaje
"Copia de seguridad verificada correctamente.". Después el fideicomisario hace clic en
**Siguiente**.

## Descargas de resultados {#result-downloads}

En el paso **Resultados**, el menú de acciones del proceso electoral, de una elección, de una
pregunta o de un área tiene elementos como "Exportar en formato PDF - Resultados de 'Election 1'".
Los formatos son **PDF**, **HTML**, **JSON**, **TAR_GZ** y **RECEIPTS_PDF**. Un formato solo se
muestra cuando la plataforma creó ese documento.

## Si hay un problema {#if-there-is-a-problem}

| Problema | Acción |
| --- | --- |
| "No puedes continuar con la ceremonia porque no se han seleccionado elecciones o las elecciones no están publicadas." | Seleccione una o más elecciones publicadas. |
| "No puedes continuar con la ceremonia porque la sesión de recuento no está conectada o el inicio de la ceremonia no está permitido." | Espere a más fideicomisarios. Después compruebe **Permitir Recuento** de cada elección y el período de votación. |
| "Error creando recuento" | Asegúrese de que las elecciones tienen la misma ceremonia de claves y están publicadas. |
| "Tu clave ya se había restaurado." | El fideicomisario ya ha subido el fragmento. No es necesaria ninguna acción. |
| Una elección muestra `ERROR`. | Contacte con el soporte de Sequent. No inicie un recuento nuevo. |

## Páginas relacionadas {#related-pages}

- [Realizar la ceremonia de recuento](../../../../../procedures/06-tally.md)
- [Obtener los resultados](../../../../../procedures/07-results.md)
- [Claves](../Keys/election_management_election-event_keys.md)
- [Pregunta: Hojas de recuento](../../Contest/Tally-Sheets/election_management_contest_tally-sheets.md)
- [Informes](../Reports/election_management_election-event_reports.md)
- [Transmisión](election_management_election-event_transmission.md)
