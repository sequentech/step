---
id: election_management_election_event_dashboard
title: Panel de Control
description: "En la parte superior de la sección Visualización de datos del panel de control de un proceso electoral hay una barra de pasos (Step Crumb) que indica el estado del proceso electoral. Estos estados son secuenciales y se describen a continuación:"
---

<!--
-- SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->


# Descripción general del Panel de Control del proceso electoral

En la parte superior de la sección **Visualización de datos** del panel de control de un proceso electoral hay una **barra de pasos (Step Crumb)** que indica el estado del proceso electoral. Estos estados son **secuenciales** y se describen a continuación:

## Pasos del estado del proceso electoral {#election-status-steps}

- **Creado**  
  La etapa inicial, en la que se configura el proceso electoral. Los administradores configuran:
  - Diseños de papeleta
  - Votantes
  - Áreas (distritos o recintos)

- **Claves**  
  Tras la configuración, el administrador ejecuta una **ceremonia de claves**, que genera las claves criptográficas. Al completarla, el estado del proceso electoral cambia a **Claves**.

- **Publicar**  
  Una vez generadas las claves:
  - Los administradores **publican** los diseños de papeleta en el portal de votación.
  - Esto equivale a imprimir las papeletas en papel.
  - Los votantes que acceden al portal de votación ya pueden ver la elección.
  - Los administradores pueden publicar varias veces durante las ediciones.

- **Iniciado**  
  El **período de votación** ha comenzado.

- **Finalizado**  
  El **período de votación** ha **concluido**.

- **Resultados**  
  - Se ejecuta la **ceremonia de recuento**.
  - Las claves criptográficas generadas previamente se usan para **descifrar las papeletas cifradas**.
  - Los **resultados** quedan disponibles para los administradores.

---

## Métricas {#metrics}

- **Electores**  
  Número total de votantes importados en el sistema, incluidos los que aún no están habilitados.

- **Elecciones**  
  Número de elecciones individuales dentro del proceso electoral.

- **Áreas**  
  Número de áreas geográficas que participan en el proceso electoral.

- **Emails Enviados**  
  Número total de correos electrónicos enviados a los votantes.

- **SMS Enviados**  
  Número total de mensajes SMS enviados a los votantes.

---

## Gráficos {#charts}

- **Votos por día**  
  Representación gráfica de la **actividad diaria de votación**, por papeleta emitida.

- **Votantes por canales**  
  Un gráfico circular que muestra la distribución de los votantes (que han votado en al menos una papeleta) por canal de votación:
  - En línea
  - Papel
  - Teléfono
  - Postal

---

Este Panel de Control ofrece a los administradores una **visión clara** del progreso del proceso electoral y de sus estadísticas clave, lo que garantiza una **gestión y supervisión eficientes** durante todo el ciclo de vida de la elección.


