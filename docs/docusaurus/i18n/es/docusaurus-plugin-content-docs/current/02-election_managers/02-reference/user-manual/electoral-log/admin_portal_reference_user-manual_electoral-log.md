---
id: admin_portal_reference_user_manual_electoral_log
title: Registro Electoral
description: "La página Registro Electoral muestra el registro electoral y la urna de los procesos electorales de su inquilino, y permite a los administradores autorizados ejecutar consultas SQL de solo lectura sobre ellos."
---

<!--
-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

## Descripción general {#overview}

La página **Registro Electoral** muestra el registro electoral y la urna de los procesos electorales de su inquilino, y permite a los administradores autorizados ejecutar consultas SQL de solo lectura sobre ellos. Nada en esta página modifica datos.

Para abrirla, seleccione **Registro Electoral** en el panel izquierdo del portal de administración. Requiere el permiso `electoral-log-console-read`.

## Explorar tablas {#browsing-tables}

1. Elija el **Evento Electoral** y la **Tabla**:
   - **Entradas:** cada entrada del registro electoral del evento.
   - **Papeletas:** las papeletas que aceptó la urna del evento, con su estado, canal de votación y tamaño, pero no su contenido.
   - **Votantes:** cada votante que votó, con el número de veces y su última papeleta.
   - **Cola del Secuenciador:** papeletas aceptadas que todavía no están registradas en el registro electoral. Normalmente está vacía o tiene pocas filas mientras se vota.
2. Para reducir la tabla, rellene los filtros y seleccione **Aplicar Filtros**. **Limpiar** los elimina.
3. En **Orden**, elija **Más recientes primero** o **Más antiguos primero**.
4. Use las flechas debajo de la tabla para ir a la página siguiente o anterior. Encima de la tabla, la página muestra aproximadamente cuántas filas tiene la tabla antes de aplicar filtros.
5. En **Entradas**, seleccione el icono de ojo de una entrada para ver todos sus campos y su mensaje decodificado. **Copiar JSON** los copia.

## Datos personales {#personal-data}

Los nombres de usuario, las direcciones IP y los países aparecen como `hidden`, salvo que tenga el permiso `electoral-log-personal-data-read`.

## Consultas {#queries}

La pestaña **Consulta** aparece a los usuarios que tienen a la vez los permisos `electoral-log-console-query` y `electoral-log-personal-data-read`.

1. Escriba una consulta SQL.
2. Seleccione **Ejecutar Consulta** o pulse Ctrl+Intro. La página muestra las filas, cuánto tardó la consulta y, cuando hay más de 1.000 filas, que solo se muestran las primeras 1.000.

- Las consultas solo pueden leer. Se ejecutan en una transacción de solo lectura, con un usuario de base de datos que solo puede leer, y se detienen a los 30 segundos.
- Solo se ejecutan consultas que devuelven filas: `SELECT`, `VALUES`, `TABLE` o `WITH`.
- Las consultas leen la base de datos del registro electoral de su inquilino, que contiene todos los procesos electorales del inquilino. Sus tablas principales son `electoral_log_messages`, `ballot_box_ballot`, `ballot_box_voter`, `ballot_box_pending` y `ballot_box_sequencer`.
- Cada consulta queda registrada en los registros del servidor con el usuario que la ejecutó.
- Las consultas no están disponibles en instalaciones que guardan el registro electoral de cada inquilino en una misma base de datos.

Ejemplos:

```sql
-- Records of each kind, across the tenant's election events
SELECT statement_kind, count(*) AS records
FROM electoral_log_messages
GROUP BY statement_kind
ORDER BY records DESC;

-- Ballots accepted per hour and voting channel in an election event
SELECT date_trunc('hour', accepted_at) AS hour, voting_channel, count(*) AS ballots
FROM ballot_box_ballot
WHERE election_event_id = '<election event ID>'
GROUP BY 1, 2
ORDER BY 1, 2;
```

## Exportar {#exporting}

Seleccione **Exportar** encima de una tabla para descargarla como archivo CSV. En **Tablas** contiene la página actual; en **Consulta**, todas las filas que devolvió la consulta.
