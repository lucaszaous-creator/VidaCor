import type { Key, ReactNode } from 'react';
import { Tab, TabList, TabPanel, Tabs } from 'react-aria-components';

export interface Aba {
  id: string;
  rotulo: string;
  conteudo: ReactNode;
}

export interface PropsDasAbas {
  rotulo: string;
  abas: Aba[];
  selecionada?: string;
  aoSelecionar?: (id: string) => void;
}

/** Abas em pílula do vc10 (`seg`, ativa em marinho), com setas e Home/End do React Aria. */
export function Abas({ rotulo, abas, selecionada, aoSelecionar }: PropsDasAbas) {
  return (
    <Tabs
      selectedKey={selecionada}
      onSelectionChange={(chave: Key) => {
        aoSelecionar?.(String(chave));
      }}
    >
      <TabList aria-label={rotulo} className="seg">
        {abas.map((a) => (
          <Tab key={a.id} id={a.id}>
            {a.rotulo}
          </Tab>
        ))}
      </TabList>
      {abas.map((a) => (
        <TabPanel key={a.id} id={a.id}>
          {a.conteudo}
        </TabPanel>
      ))}
    </Tabs>
  );
}
