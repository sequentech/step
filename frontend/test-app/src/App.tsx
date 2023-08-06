import { useContext } from 'react';
import logo from './logo.svg';
import './App.css';
import { AuthContext } from './context/AuthContextProvider';

let App = () => {
  const authContext = useContext(AuthContext);

  return (
    <div className="App">
      <header className="App-header">
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
      </header>
    </div>
  );
};

export default App;
