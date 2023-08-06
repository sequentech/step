import React from 'react';
import logo from './logo.svg';
import './App.css';

import Keycloak from 'keycloak-js';

const KeycloakConfig = {
  url: 'http://127.0.0.1:8090/auth',
  realm: 'electoral-process',
  clientId: 'frontend'
};

let App = () => {
  const keycloak = new Keycloak(KeycloakConfig);

  try {
    //const authenticated = await keycloak.init();
    //console.log(`User is ${authenticated ? 'authenticated' : 'not authenticated'}`);
  } catch (error) {
    console.error('Failed to initialize adapter:', error);
  }

  return (
    <div className="App">
      <header className="App-header">
        <img src={logo} className="App-logo" alt="logo" />
        <p>
          Edit <code>src/App.tsx</code> and save to reload. Let's change it.
        </p>
        <a
          className="App-link"
          href="https://reactjs.org"
          target="_blank"
          rel="noopener noreferrer"
        >
          Learn React
        </a>
      </header>
    </div>
  );
};

export default App;
