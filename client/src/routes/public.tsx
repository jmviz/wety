import { Navigate, Outlet, useRoutes} from 'react-router-dom';
import { Home } from 'pages/Home';
import { Layout } from 'components/Layout/Layout';

const App = () => {
  return (
    <Layout>
      <Outlet />
    </Layout>
  );
};

export const Routes = () => {
  const publicRoutes = [
    {
      path: '/',
      element: <App />,
      children: [
        { path: '/', element: <Home /> },
        { path: '*', element: <Navigate to="." /> },
      ],
    }
  ];

  const element = useRoutes(publicRoutes);

  return (
    <>
      {element}
    </>
  );
};