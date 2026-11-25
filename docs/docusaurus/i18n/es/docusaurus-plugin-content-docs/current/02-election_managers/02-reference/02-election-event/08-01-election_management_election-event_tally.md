---
id: election_management_election_event_tally
title: Ceremonia de recuento
description: "Esta ceremonia garantiza que solo un quórum de fideicomisarios pueda reconstruir la clave privada para el descifrado, lo que preserva la seguridad durante todo el ciclo de vida de la elección."
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->


## Introducción {#introduction}
La sección de Recuento incluye los procedimientos esenciales para consolidar los resultados una vez finalizada la votación. Los miembros de la junta electoral siguen las instrucciones para iniciar la ceremonia de recuento, verificar los fragmentos de clave, ejecutar el recuento y revisar los resultados. Si sigue estas instrucciones, la fase posterior a la elección se gestiona con integridad y precisión, y se asegura de que los resultados se verifiquen y publiquen correctamente, y de que el proceso siga siendo transparente y confiable.

## Ceremonia de claves de cierre {#closing-key-ceremony}
Durante la ceremonia de claves de cierre de un proceso electoral, los fideicomisarios usan sus fragmentos de clave criptográfica para descifrar el recuento final:

- **Recuperar los fragmentos de clave guardados:** Cada fideicomisario recupera de forma segura el fragmento de la clave privada que guardó en su copia de seguridad.
- **Verificar la integridad y el funcionamiento:** Los fideicomisarios realizan pruebas o auditorías criptográficas para confirmar que cada fragmento no ha sido alterado y que funciona.
- **Descifrar los votos cifrados:** Los fideicomisarios combinan los fragmentos (en número igual o superior al umbral) para descifrar los datos de votación cifrados acumulados, lo que garantiza resultados precisos y transparentes.
- **Almacenamiento seguro de las claves:** Después del descifrado, los fragmentos de clave se vuelven a guardar de forma segura para mantener la confidencialidad y la autenticidad del resultado de la elección.

Esta ceremonia garantiza que solo un quórum de fideicomisarios pueda reconstruir la clave privada para el descifrado, lo que preserva la seguridad durante todo el ciclo de vida de la elección.

## Requisitos previos para el recuento {#prerequisites-for-tally}
Antes de iniciar el proceso de recuento:

1. **Ceremonia de claves de apertura completada:** Se han generado y verificado todos los fragmentos de clave.
2. **Período de votación finalizado (opcional):** Puede que se hayan emitido votos o no. Iniciar un recuento sin votos es técnicamente posible, pero produce resultados vacíos.

> Nota: Según la política de ceremonias de llaves/recuento del proceso electoral, si la ceremonia de claves está configurada como "Automática", la ceremonia de recuento también será automática y no se requerirá ninguna acción de los fideicomisarios.

## Iniciar el proceso de recuento {#starting-the-tally-process}
1. Vaya al **Proceso electoral** correspondiente en el portal de administración.
2. Vaya a la pestaña **Recuento**.
3. Haga clic en **Iniciar la Ceremonia de Recuento**.
4. Seleccione una o más elecciones para incluir en este recuento.
5. Confirme para continuar. El sistema avisará a los fideicomisarios para que verifiquen sus fragmentos de clave.
6. Los fideicomisarios verifican sus fragmentos cuando se les indique. Cuando se verifique el número de fragmentos que indica el umbral, el recuento comienza automáticamente.
7. Espere a que el sistema finalice el recuento. Cuando termine, aparece una notificación de éxito.

## Verificación de la clave del fideicomisario para el recuento {#trustee-key-verification-for-tally}
1. Inicie sesión en el portal de administración y abra el proceso electoral correspondiente.
2. Vaya a la pestaña **Recuento**.
3. Haga clic en la invitación de acción de clave o en el icono de llave junto a la ceremonia de recuento.
4. Seleccione la elección para el recuento.
5. Arrastre y suelte su fragmento de clave cifrado para verificarlo, o súbalo.
   Después de una carga válida, el área de carga queda bloqueada. Si vuelve a subir la misma clave desde una pestaña del navegador desactualizada, se acepta como ya restaurada. Si sube una clave distinta, se rechaza.
6. Repita el proceso para cada fideicomisario hasta alcanzar el umbral.
7. Cuando se hayan verificado todos los fragmentos necesarios, haga clic en **Iniciar Recuento** para comenzar el descifrado y el recuento.

## Revisar los resultados {#reviewing-results}
Una vez finalizado el recuento, los resultados están disponibles en la pestaña **Recuento**:

- **Ver resultados**: Haga clic en el botón de ver resultados de cada elección.
- **Columnas**: Active o desactive las columnas según sea necesario para facilitar la lectura.
- **Progreso del Recuento de Elecciones**: Supervise el estado (por ejemplo, "En curso" o "Éxito").
- **Logs**: Acceda a los registros detallados para la auditoría y la resolución de problemas.
- **Información General**: Muestra la fecha del recuento, la participación total de votantes, etc.
- **Resultados & Participación**: Desglose detallado por áreas, preguntas y elecciones.

### Información General {#general-information}
- **Fecha y hora del recuento**: Momento en que se finalizó el recuento.
- **Participación de votantes**: Resumen del número de votantes elegibles que participaron.

### Desglose de resultados y participación {#results--participation-breakdown}
- **Por elección**: Resultados de cada elección dentro del proceso electoral.
- **Por pregunta**: Resultados de cada pregunta en todas las elecciones.
- **Por área**: Resultados desglosados por divisiones geográficas u organizativas.
- **Resumen de participación**:
  - **Votantes Elegibles**: Número total de personas con derecho a votar.
  - **Total de votantes**: Número de personas que emitieron un voto.
  - **Total de votos válidos**: Número de papeletas válidas, incluidas las papeletas en blanco.
  - **Total de votos inválidos**: Número de papeletas inválidas, desglosado en:
    - **Votos explícitamente inválidos**: Votos nulos, papeletas estropeadas y acciones de protesta.
    - **Votos implícitamente inválidos**: Votos inválidos por la configuración (por ejemplo, varias selecciones cuando solo se permite una).
  - **Votos en blanco**: Papeletas en blanco explícitas e implícitas. Las papeletas en blanco se
    incluyen en el total de votos válidos.

### Resultados de candidatos {#candidate-results}
Para cada pregunta, muestre:

- **Nombre de la opción / candidato**
- **Número de Votos**: Número de votos válidos de ese candidato u opción.
- **Porcentaje de votos**: Proporción respecto a los votos válidos no en blanco, cuando el resultado es una fila de candidato.
- **Clasificación / Posición**: Puesto del candidato según el número de votos.

> Nota: Según la Política de Votación Ponderada del proceso electoral, si la política está configurada como "Votación Ponderada por Áreas", el recuento se calculará con los pesos asignados a cada área durante la última publicación anterior al recuento.

## Resumen {#summary}
Si sigue esta guía, los administradores garantizan que la ceremonia de recuento se lleve a cabo de forma segura, transparente y precisa. Los fideicomisarios verifican los fragmentos de clave, los votos cifrados se descifran correctamente y los resultados se revisan con todas las capacidades de auditoría. Así se mantienen la integridad y la confiabilidad, fundamentales para unas elecciones creíbles.
