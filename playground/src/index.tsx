import './index.css';
import React from 'react';
import ReactDOM from 'react-dom';
import Playground from './pages/playground';

const mountApp = () => {
  ReactDOM.render(
    <Playground />,
    document.getElementById('app'),
  );
};
mountApp();
