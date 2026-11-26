---
id: settings_voting-channels
title: Canales de votación
description: "La pestaña CANALES DE VOTACIÓN está en Configuración. Guarda un registro, a nivel de inquilino, de los canales de votación: en línea, quiosco y telefónico. Lo usa el administrador del inquilino."
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

La pestaña **CANALES DE VOTACIÓN** está en **Configuración**. Guarda un registro, a nivel de inquilino, de los canales de votación: en línea, quiosco y telefónico. Lo usa el administrador del inquilino.

:::note NOTA
Un nuevo proceso electoral no copia los valores de esta pestaña. Obtiene sus propios canales de votación al crearlo. Los canales de votación que acepta un proceso electoral se configuran en la pestaña **Datos** del proceso electoral, en la sección **Canales de Voto Permitidos**. Un cambio en esta pestaña no modifica los procesos electorales. Consulte [Datos del proceso electoral](../../02-election-event/03-election_management_election-event_data.md).
:::

## Cómo abrirlo {#how-to-open-it}

1. En el menú de la izquierda, haga clic en **Configuración**.
2. Haga clic en la pestaña **CANALES DE VOTACIÓN**. La pantalla muestra la etiqueta tal como se escribe en este manual.

## Permiso necesario {#required-permission}

Para abrir **Configuración**, su cuenta debe tener los permisos **Editar Inquilino** y **Ver configuración**. Si no los tiene, la pantalla muestra "No tienes permiso para acceder a la configuración."

## Campos {#fields}

| Interruptor | Significado | Valor predeterminado |
| --- | --- | --- |
| **Votación en Línea** | Los votantes votan por internet en el portal de votación. | Activado |
| **Votación en Quiosco** | Los votantes votan en un dispositivo de quiosco. | Desactivado |
| **Votación Telefónica** | Los votantes votan por teléfono. | Desactivado |

No hay botón **Guardar**. Cuando hace clic en un interruptor, el portal de administración guarda el cambio de inmediato.

## Cambiar un canal de votación {#change-a-voting-channel}

1. Abra la pestaña **CANALES DE VOTACIÓN**.
2. Haga clic en el interruptor del canal de votación.

**Resultado esperado:** el interruptor muestra la nueva posición.

## Si hay un problema {#if-there-is-a-problem}

| Problema | Acción |
| --- | --- |
| Aparece el mensaje "No tienes permiso para acceder a la configuración." | Pida a un administrador del inquilino o al soporte de Sequent que dé a su cuenta los permisos necesarios. |
| **Votación en Línea** aparece activada después de desactivarla y volver a abrir la pestaña. | La pestaña muestra siempre **Votación en Línea** activada al cargarse. Esto no tiene efecto en los procesos electorales. Configure los canales de votación de cada proceso electoral en su pestaña **Datos**. |
| Un proceso electoral no acepta un canal de votación. | Abra la pestaña **Datos** del proceso electoral y revise **Canales de Voto Permitidos**. Esta pestaña no lo configura. |

**Procedimiento relacionado:** [Configurar el inquilino](../../../03-procedures/01-tenant.md).
