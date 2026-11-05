---
id: acclaimed_contests
title: Preguntas resueltas por aclamación
sidebar_position: 11
description: "Una pregunta resuelta por aclamación se decide antes de la votación: sus candidatos configurados resultan elegidos sin votación."
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

Una **pregunta resuelta por aclamación** se decide antes de la votación: sus candidatos configurados resultan elegidos sin votación. La pregunta sigue visible durante toda la experiencia del votante y de resultados, pero nunca forma parte de una papeleta codificada.

La función sigue una regla:

> **Una pregunta resuelta por aclamación se muestra, pero nunca se codifica.**

Es una configuración por pregunta. Una elección puede contener preguntas resueltas por aclamación y preguntas con votación normal.

## Configuración {#configuration}

1. Abra la pregunta de destino en el portal de administración.
2. Vaya a la pestaña **Datos** y expanda **Diseño de la papeleta**.
3. Active **Resuelto por aclamación**.
4. Compruebe que los candidatos configurados son las personas que deben resultar elegidas.
5. Guarde la pregunta y genere una nueva publicación de papeletas.

Configure la aclamación antes de publicar las papeletas. La configuración determina qué preguntas participan en la codificación de papeletas, así que cambiarla después de la publicación puede invalidar los ID de papeleta existentes y hacer que las papeletas sean incompatibles con su definición publicada.

Consulte [Datos de la pregunta](./04-contest/01-election_management_contest_data.md) para ver los demás ajustes de la pregunta.

### Notificación de aclamación {#acclamation-notice}

Por defecto, el portal de votación y el verificador de papeletas usan la notificación de aclamación estándar traducida. Una pregunta puede sustituirla por idioma en su configuración de presentación:

```json
{
  "i18n": {
    "en": {
      "acclamation_description": "This contest was decided by acclamation."
    },
    "fr": {
      "acclamation_description": "Ce vote a été acquis par acclamation."
    }
  }
}
```

Se usa el idioma actual cuando está disponible, seguido del idioma por defecto configurado y, después, de la notificación traducida estándar de la aplicación.

## Comportamiento del portal de votación {#voting-portal-behavior}

Las pantallas de papeleta y de revisión muestran la pregunta resuelta por aclamación en el orden normal de preguntas, incluidas su notificación y los candidatos disponibles. Las opciones de candidato están desactivadas y no se pueden seleccionar.

Los siguientes ajustes no se aplican a una pregunta resuelta por aclamación:

- los requisitos mínimo y máximo de selección;
- las políticas de subvoto, voto excesivo, voto en blanco y voto inválido;
- las opciones explícitas de voto en blanco y de voto inválido; y
- los votos por escrito.

### Elección mixta {#mixed-election}

Cuando una elección contiene preguntas resueltas por aclamación y preguntas con votación normal, el votante completa solo las preguntas con votación normal. Esas selecciones producen la papeleta cifrada habitual, el ID de papeleta, la opción de auditoría, el recibo y el flujo de seguimiento. La pregunta resuelta por aclamación no aporta nada a esa papeleta.

### Elección totalmente aclamada {#fully-acclaimed-election}

Cuando todas las preguntas de la papeleta del votante están resueltas por aclamación, no hay nada que emitir. El votante puede revisar lo que se ha decidido, pero:

- no se cifra ni se envía ninguna papeleta;
- no se crea ningún ID de papeleta, código QR, enlace de seguimiento, recibo ni opción de auditoría; y
- la pantalla de confirmación indica explícitamente que no se emitió ninguna papeleta.

## Comportamiento del verificador de papeletas {#ballot-verifier-behavior}

Una pregunta resuelta por aclamación no tiene ninguna selección decodificada, porque no está codificada. En una elección mixta, el verificador de papeletas combina las selecciones decodificadas con las preguntas resueltas por aclamación de la configuración de papeleta incrustada para mostrarlas. Después muestra la misma notificación de aclamación y los mismos candidatos desactivados que la pantalla de revisión del portal de votación.

Esta combinación es solo de presentación. No añade la pregunta a los datos de papeleta decodificados y no cambia el hash de las papeletas, las comprobaciones de firma, las comprobaciones de recodificación ni la comparación de ID de papeleta. En una elección totalmente aclamada no hay ninguna papeleta auditable que abrir en el verificador.

Consulte [Audite su voto](../../../../../../docs/03-voters/01-tutorials/03-voter_audit_ballot.md) para ver el procedimiento completo de auditoría del votante, y la [Especificación de codificación de papeletas](../../../../../../docs/05-reference/07-ballot_encoding.md) para las reglas de codificación.

## Recuento y resultados {#tally-and-results}

Como ninguna papeleta contiene la pregunta resuelta por aclamación, el recuento crea su resultado a partir de la configuración publicada de la pregunta, en lugar de ejecutar el algoritmo de escrutinio configurado. El resultado contiene:

- cero en censo, participación, votos, votos en blanco, votos inválidos, totales de candidatos y porcentajes;
- todos los candidatos elegibles configurados marcados como ganadores, en el orden configurado, con las posiciones ganadoras `1..N`; y
- ningún proceso de escrutinio ni datos de participación por canal.

Los marcadores explícitos de voto en blanco y de voto inválido, los candidatos desactivados y los huecos vacíos de voto por escrito no se tratan como candidatos elegidos. El portal de administración, el portal público de resultados y el informe en PDF muestran una notificación de aclamación en lugar de las cifras normales de participación.

Las hojas de recuento se rechazan en las preguntas resueltas por aclamación, porque los votos en papel, postales u otros votos externos no se pueden añadir a una pregunta que se decidió sin votación.

Consulte [Generación de resultados](../../../../../../docs/05-reference/04-tally_deep_dive/06-tally_results.md) para ver la forma canónica del resultado del recuento.

## Resumen {#summary}

| Aspecto | Comportamiento |
|--------|----------|
| **Configuración** | Pregunta **Datos** → **Diseño de la papeleta** → **Resuelto por aclamación** |
| **Cuándo configurar** | Antes de la publicación de papeletas; no lo cambie después de publicar |
| **Interacción del votante** | Se muestran la pregunta y los candidatos, pero todas las opciones están desactivadas |
| **Codificación de la papeleta** | Sin posición en texto claro, sin texto cifrado y sin pregunta decodificada |
| **Elección mixta** | Las demás preguntas se emiten y se auditan con normalidad |
| **Elección totalmente aclamada** | Sin papeleta, ID de papeleta, recibo, seguimiento ni auditoría |
| **Verificador** | Añade la pregunta desde la configuración de la papeleta, solo para mostrarla |
| **Recuento** | Resultado sintético en cero; todos los candidatos elegibles son ganadores |
| **Participación** | Se suprime y se sustituye por una notificación de aclamación |
| **Hojas de recuento** | Rechazadas |
