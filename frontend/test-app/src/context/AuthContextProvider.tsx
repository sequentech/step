import { createContext, useState } from "react";

/**
 * AuthContextValues defines the structure for the default values of the {@link AuthContext}.
 */
interface AuthContextValues {
  isAuthenticated: boolean;
}

/**
 * defaultAuthContextValues defines the default values for the {@link AuthContext}
 */
const defaultAuthContextValues: AuthContextValues = {
  isAuthenticated: false,
};

/**
 * AuthContext is the context exposed by the {@link AuthContextProvider}.
 */
export const AuthContext = createContext<AuthContextValues>(
  defaultAuthContextValues
);

/**
 * The props that must be passed to create the {@link AuthContextProvider}.
 */
interface AuthContextProviderProps {
  /**
   * The elements wrapped by the auth context.
   */
  children: JSX.Element;
}

/**
 * AuthContextProvider is responsible for managing the authentication state of the current user.
 *
 * @param props
 */
const AuthContextProvider = (props: AuthContextProviderProps) => {
  // This is just here to check that we setup the provider correctly.
  console.log("rendering AuthContextProvider");

  // Creating the local state to keep track of the authentication
  const [isAuthenticated, setAuthenticated] = useState<boolean>(false);

  return (
    // Creating the provider and passing the state into it. Whenever the state changes the components using this context will be re-rendered.
    <AuthContext.Provider value={{ isAuthenticated }}>
      {props.children}
    </AuthContext.Provider>
  );
};

export default AuthContextProvider;
