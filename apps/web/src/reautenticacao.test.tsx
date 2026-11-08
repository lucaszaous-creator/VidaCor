import { render } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { abrirJanelaDeReautenticacao, CANAL_DE_REAUTENTICACAO, MENSAGEM_REAUTENTICADO } from './reautenticacao';
import { TelaReautenticado } from './telas/TelaReautenticado';

describe('reautenticação em janela (DEC-2026-11-08-54)', () => {
  it('janela bloqueada pelo navegador: devolve "bloqueada" na hora', async () => {
    const janela = { open: vi.fn(() => null) } as unknown as Window;
    await expect(abrirJanelaDeReautenticacao('/v1/sessao/reautenticar?voltar=%2Freautenticado', janela)).resolves.toBe('bloqueada');
  });

  it('a volta da janela avisa a tela de origem, que segue sem perder o digitado', async () => {
    const abrir = vi.fn(() => ({}));
    const janela = { open: abrir } as unknown as Window;
    const resultado = abrirJanelaDeReautenticacao('/v1/sessao/reautenticar?voltar=%2Freautenticado', janela);
    const fechar = vi.spyOn(window, 'close').mockImplementation(() => undefined);
    render(<TelaReautenticado />);
    await expect(resultado).resolves.toBe('confirmada');
    expect(fechar).toHaveBeenCalled();
    expect(abrir).toHaveBeenCalledWith(expect.stringContaining('voltar=%2Freautenticado'), 'vidacor-reautenticacao', expect.any(String));
  });

  it('mensagem estranha no canal não conta como confirmação', async () => {
    const janela = { open: vi.fn(() => ({})) } as unknown as Window;
    let resolvida = false;
    void abrirJanelaDeReautenticacao('/x', janela).then(() => {
      resolvida = true;
    });
    const canal = new BroadcastChannel(CANAL_DE_REAUTENTICACAO);
    canal.postMessage({ tipo: 'outra-coisa' });
    canal.postMessage(MENSAGEM_REAUTENTICADO);
    await new Promise((r) => setTimeout(r, 50));
    canal.close();
    expect(resolvida).toBe(false);
  });
});
