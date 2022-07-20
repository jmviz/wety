type ActionMap<M extends { [index: string]: any }> = {
  [Key in keyof M]: M[Key] extends undefined
    ? {
        type: Key;
      }
    : {
        type: Key;
        payload: M[Key];
      }
};

export enum ActionTypes {
  EnableDarkMode = "ENABLE_DARK_MODE",
  DisableDarkMode = "DISABLE_DARK_MODE",
}

// theme settings
export interface themeSettingsType {
  darkModeEnabled: boolean;
}

type ThemeSettingsPayload = {
  [ActionTypes.EnableDarkMode]: undefined,
  [ActionTypes.DisableDarkMode]: undefined
}

export type ThemeSettingsActions = ActionMap<
  ThemeSettingsPayload
>[keyof ActionMap<ThemeSettingsPayload>];


export const themeSettingsReducer = (
  state: themeSettingsType,
  action: ThemeSettingsActions
) => {
  let stateCopy = Object.assign({}, state)
  switch (action.type) {
    case ActionTypes.EnableDarkMode:
      return {...stateCopy, darkModeEnabled: true};
    case ActionTypes.DisableDarkMode:
      return {...stateCopy, darkModeEnabled: false};
    default:
      return state;
  }
};
