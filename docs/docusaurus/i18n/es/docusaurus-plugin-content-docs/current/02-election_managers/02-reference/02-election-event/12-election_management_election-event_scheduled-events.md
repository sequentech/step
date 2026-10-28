---
id: election_management_election_event_scheduled_events
title: Eventos Programados
description: "Permite automatizar un proceso electoral programando acciones clave para que se ejecuten automáticamente en momentos concretos del ciclo de vida del proceso."
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->


Permite automatizar un proceso electoral programando acciones clave para que se ejecuten automáticamente en momentos concretos del ciclo de vida del proceso.

### Añadir un evento programado {#adding-a-scheduled-event}

- Seleccione **Añadir**.
- Seleccione un **Tipo de evento**.
- Seleccione una **Elección**.
  - Si no selecciona ninguna elección, el evento programado se aplicará al proceso electoral y a todas las elecciones relacionadas.
- Para **Inicio del Período de Votación** o **Fin del Período de Votación**, seleccione uno o más **Canales de Voto**: **En línea**, **Kiosco**, **Votación anticipada** o **Votación telefónica**. **En línea** y **Kiosco** están seleccionados de forma predeterminada.
  - Los cuatro canales siempre están disponibles en el formulario. Cuando se ejecuta la programación, solo cambia los canales habilitados en cada elección afectada.
  - Las programaciones existentes sin selección de canales (o con una selección vacía en la carga) usan En línea y Kiosco tanto para el inicio como para el fin. Al editar, exportar o importar una programación, se conserva su selección de canales.
- Seleccione la **Fecha y hora de inicio** de este evento.
- Seleccione la **Fecha y hora de inicio** en la que se activa este evento programado.

> **Nota:**  
> - La fecha y hora de inicio de **Inicio del Período de Votación** es la hora en que empieza el período de votación.
> - La fecha y hora de inicio de **Fin del Período de Votación** es la hora en que termina el período de votación.  
