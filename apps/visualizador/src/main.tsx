import '@fontsource/inter/400.css';
import '@fontsource/inter/500.css';
import '@fontsource/inter/600.css';
import '@fontsource/inter/700.css';
import '@vidacor/design-system/estilos.css';
import './visualizador.css';
import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { Aplicacao } from './Aplicacao';
import { ponteTauri } from './ponte';

const elemento = document.getElementById('raiz');
if (elemento === null) {
  throw new Error('Elemento #raiz não encontrado');
}

createRoot(elemento).render(
  <StrictMode>
    <Aplicacao ponte={ponteTauri} />
  </StrictMode>,
);
