import '@fontsource/inter/400.css';
import '@fontsource/inter/500.css';
import '@fontsource/inter/600.css';
import '@fontsource/inter/700.css';
import '@vidacor/design-system/estilos.css';
import './telas/agenda/agenda.css';
import { criarCliente } from '@vidacor/api-client';
import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { Aplicacao } from './Aplicacao';

const elemento = document.getElementById('raiz');
if (elemento === null) {
  throw new Error('Elemento #raiz não encontrado');
}

createRoot(elemento).render(
  <StrictMode>
    <Aplicacao
      dependencias={{
        api: criarCliente(),
        irPara: (url) => {
          window.location.assign(url);
        },
      }}
    />
  </StrictMode>,
);
