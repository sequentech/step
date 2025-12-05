// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>

// SPDX-License-Identifier: AGPL-3.0-only


import { Component } from "react";

class KeycloakManLovesJsx extends Component {
  public render() {
    return (
      <div className="pf-c-card">
        <div className="pf-c-card__body">
          <div className="pf-c-empty-state pf-m-sm">
            <div className="pf-c-empty-state__content">
              <h4 className="pf-c-title pf-m-lg">
                Keycloak Man Loves JSX, React, and PatternFly
              </h4>
              <div className="pf-c-empty-state__body">
                <div className="pf-l-grid pf-m-gutter">
                  <div className="pf-l-grid__item pf-m-12-col">
                    <img src="public/keycloak-man-95x95.jpg" />
                  </div>
                  <div className="pf-l-grid__item pf-m-12-col">
                    <img src="public/heart-95x95.png" />
                  </div>
                  <div className="pf-l-grid__item pf-m-12-col">
                    <img src="public/jsx-95x95.png" />
                    <img src="public/react-95x95.png" />
                    <img src="public/patternfly-95x95.png" />
                  </div>
                </div>
              </div>
              <h4 className="pf-c-title pf-m-lg">
                But you can use whatever you want as long as you wrap it in a
                React Component.
              </h4>
            </div>
          </div>
        </div>
      </div>
    );
  }
}

export default KeycloakManLovesJsx;