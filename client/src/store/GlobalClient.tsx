import React, { createContext, useReducer, Dispatch } from "react";
import {
  themeSettingsType,
  themeSettingsReducer,
  ThemeSettingsActions,
} from "./reducers";

type InitialStateType = {
  themeSettings: themeSettingsType;
};

const initialState: InitialStateType = {
  themeSettings: {
    darkModeEnabled: true,
  }
}

const GlobalClientContext = createContext<{
  state: InitialStateType;
  dispatch: Dispatch<ThemeSettingsActions>;
}>({
  state: initialState,
  dispatch: () => null
});

const mainReducer = (
  { themeSettings }: InitialStateType,
  action: ThemeSettingsActions
) => ({
  themeSettings: themeSettingsReducer(themeSettings, action),
});

interface Props {
  children: React.ReactNode;
}

const GlobalClient: React.FC<Props> = ({ children }) => {
  const [state, dispatch] = useReducer(mainReducer, initialState);

  return (
    <GlobalClientContext.Provider value={{ state, dispatch }}>
      {children}
    </GlobalClientContext.Provider>
  );
};

export { GlobalClient, GlobalClientContext };
