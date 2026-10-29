---
id: decline_to_vote
title: Abstención de voto
sidebar_position: 7
description: "La política de abstención de voto permite a los votantes abstenerse formalmente de una elección completa, en todas sus preguntas a la vez, en lugar de emitir selecciones de candidatos."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

La política de **abstención de voto** permite a los votantes abstenerse formalmente de una elección completa, en todas sus preguntas a la vez, en lugar de emitir selecciones de candidatos. Es una acción **a nivel de elección**: un votante se abstiene en toda la papeleta o vota normalmente en todas las preguntas.

Esta función está disponible **solo cuando el proceso electoral usa la política de cifrado Varios concursos**. No se aplica al cifrado de Concurso único.

> **Nota sobre la Segunda Vuelta Instantánea (IRV):** el algoritmo de recuento Segunda Vuelta Instantánea no es compatible actualmente con el cifrado Varios concursos. Como la abstención de voto requiere el cifrado Varios concursos, no se aplica a las elecciones que usan IRV hoy. Consulte [Algoritmo de Segunda Vuelta Instantánea](../../../../../../docs/07-developers/07-velvet/05-instant-runoff.md) para ver los detalles sobre las limitaciones de IRV.

---

## Requisitos previos {#prerequisites}

Antes de poder usar la abstención de voto, deben configurarse los dos ajustes siguientes:

1. **Política de cifrado de concurso del proceso electoral:** configúrela en **Varios concursos** en la pestaña **Datos** del proceso electoral, dentro de **Configuración Avanzada**. Consulte [Datos del proceso electoral](./02-election-event/03-election_management_election-event_data.md).
2. **Política de abstención de voto de la elección:** configúrela en **Habilitado** en la pestaña **Datos** de la elección, dentro de **Configuración Avanzada**. Consulte [Datos de la elección](./03-election/03-election_management_election_data.md).

Si el proceso electoral usa el cifrado **Concurso único**, el ajuste de la política de abstención de voto queda oculto en el portal de administración y la opción no se ofrece a los votantes.

---

## Portal de administración {#admin-portal}

### Activar la política {#enabling-the-policy}

1. Abra el **proceso electoral** y confirme que **Política de cifrado de concurso** está en **Varios concursos**.
2. Abra la **elección** de destino y vaya a la pestaña **Datos**.
3. Despliegue **Configuración Avanzada**.
4. Configure **Política de abstención de voto** en **Habilitado** (el valor predeterminado es **Deshabilitado**).
5. Guarde la elección.

La política se configura por elección, no por pregunta. Todas las preguntas de esa elección comparten el mismo comportamiento de abstención de voto.

### Resultados del recuento {#tally-results}

Después de que termine un recuento, las papeletas de abstención aparecen en los resultados a nivel de elección:

- En la pestaña **Recuento**, la tabla **Elecciones** de resultados incluye una columna **Total de votos de renuncia** cuando al menos una elección incluida tiene habilitada la abstención de voto con cifrado Varios concursos.
- Los informes de elección generados incluyen una fila **Total de votos de renuncia** en el resumen de participación a nivel de elección.

Las papeletas de abstención **no** aparecen por separado en las tablas de resultados por pregunta. Las tablas de recuento por pregunta no muestran un recuento aparte de votos de abstención junto a los votos válidos, inválidos o en blanco.

---

## Portal de votación {#voting-portal}

Cuando la abstención de voto está habilitada para una elección, los votantes ven una opción adicional en la **pantalla de inicio** (antes de las preguntas de votación):

- Botón **Abstención de voto**: abre un cuadro de diálogo de confirmación que explica que el votante omitirá la selección de preguntas y pasará directamente a la revisión.
- Botón **Iniciar Votación**: el flujo normal por todas las preguntas.

### Flujo de abstención de voto {#decline-to-vote-flow}

1. El votante hace clic en **Abstención de voto** y confirma en el cuadro de diálogo.
2. La papeleta se marca como abstención en **todas las preguntas** de la elección.
3. El votante pasa directamente a la **pantalla de revisión**, sin pasar por las pantallas de votación de las preguntas.
4. En la pantalla de revisión, cada pregunta muestra **Abstención de voto** en lugar de las selecciones de candidatos.
5. El votante cifra y emite la papeleta como de costumbre (incluida la auditoría opcional, según la configuración).

### Flujo de votación normal {#normal-voting-flow}

Si el votante hace clic en **Iniciar Votación**, avanza por las preguntas de forma normal. No se admite elegir **Abstención de voto** después de iniciar el flujo normal: la abstención solo está disponible desde la pantalla de inicio.

El destino del botón **Atrás** en la primera pantalla de votación lo controla la **Política del botón Atrás de la pantalla de votación** a nivel de elección. Cuando la abstención de voto está habilitada, configure esa política en **Ir a la pantalla de inicio de la elección** para que la opción de abstención siga siendo accesible; si no, el botón **Atrás** vuelve a la pantalla de selección de elecciones (el valor predeterminado). Consulte [Datos de la elección](./03-election/03-election_management_election_data.md).

Cada vez que un votante llega a la pantalla de inicio, la papeleta vuelve a su estado inicial: se borran todas las selecciones anteriores, incluida cualquier selección previa de abstención.

---

## Resultados {#results}

La forma en que se cuentan las papeletas de abstención difiere entre el informe a nivel de elección y el informe a nivel de pregunta.

### Nivel de elección {#election-level}

A nivel de elección, una papeleta de abstención:

- **Cuenta para la participación total de votantes** (incluida en **Total de votantes** / votos emitidos para la elección).
- Se informa por separado como **Total de votos de renuncia**, con su propio número y su porcentaje sobre el censo.

Esto refleja que el votante participó en la elección al abstenerse formalmente, aunque no seleccionara ningún candidato.

### Nivel de pregunta {#contest-level}

A nivel de pregunta, una papeleta de abstención:

- **No** cuenta como **voto válido**.
- **No** cuenta como **voto inválido** (ni explícito ni implícito).
- **No** cuenta como **voto en blanco**.
- **No** contribuye a los totales de ningún candidato.

Por tanto, las tablas de participación y de resultados de candidatos por pregunta excluyen las papeletas de abstención de todas las categorías de voto. La abstención solo se registra a nivel de elección.

---

## Resumen {#summary}

| Aspecto | Comportamiento |
|--------|----------|
| **Política de cifrado** | Solo Varios concursos |
| **Algoritmo de recuento** | No compatible con IRV (IRV requiere la codificación de Concurso único) |
| **Ubicación de la configuración** | Elección **Datos** → **Configuración Avanzada** → **Política de abstención de voto** |
| **Punto de entrada del votante** | Botón **Abstención de voto** en la pantalla de inicio |
| **Alcance** | Toda la elección (todas las preguntas a la vez) |
| **Resultados a nivel de elección** | Cuenta como participación; se muestra como **Total de votos de renuncia** |
| **Resultados a nivel de pregunta** | No se cuenta como válido, inválido ni en blanco |
