import * as React from 'react';
import { useContext } from 'react';
import { GlobalClientContext } from 'store/GlobalClient';

// style
import './Layout.scss';

type LayoutProps = {
  children: React.ReactNode;
};

export const Layout = ({children }: LayoutProps) => {
  const {state} = useContext(GlobalClientContext);

  let theme;

  if (state.themeSettings.darkModeEnabled) {
    theme = 'dark';
  } else {
    theme = 'light';
  }

  return(
    <div className={"layout-container"} data-theme={theme}>
      <div className="content-container">
        {children}
      </div>
    </div>
  )
}