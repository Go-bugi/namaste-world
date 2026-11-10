import '@renderer/assets/main.css';

import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import App from './App';

// 프로덕션에서는 새로고침 단축키(Ctrl/Cmd+R)를 막는다.
if (import.meta.env.PROD) {
  window.addEventListener('keydown', (event) => {
    if (event.code === 'KeyR' && (event.ctrlKey || event.metaKey)) {
      event.preventDefault();
    }
  });
}

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
