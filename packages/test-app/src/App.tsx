import { useContext } from 'react';
import logo from './logo.svg';
import './App.css';
import { AuthContext } from './context/AuthContextProvider';
import {WarnBox, Header, Footer, initializeLanguages} from '@sequentech/ui-essentials';

initializeLanguages({})

let App = () => {
  const authContext = useContext(AuthContext);

  return (
    <div className="App">
        <img src={logo} className="App-logo" alt="logo" />
        {
        !authContext.isAuthenticated
          ? <p>Not Authenticated</p>
          : <p>User Authenticated</p>
        }
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
        <Header />
        <WarnBox variant="error" onClose={() => undefined}>
            <b>Question / Contest Title 2:</b> You have chosen more than the allowed selecitons on
            this contest/office.
        </WarnBox>
    </div>
  );
};

export default App;
