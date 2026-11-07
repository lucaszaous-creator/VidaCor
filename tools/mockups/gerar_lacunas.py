P='/tmp/claude-0/-home-user/4fd689ae-5e0d-5bbd-bb63-c60bbbb26a18/scratchpad/vidacor-canvas/project/'
CSS='/_blob/079f870cfe8a03ec36101026af3bba84'
BASE='''body{margin:0;background:#F2F3F6}
.ck{width:24px;height:24px;border-radius:50%;display:flex;align-items:center;justify-content:center;flex:none;font-size:12px;font-weight:700}
.ck.ok{background:#E5F5EC;color:#1F7A4D}.ck.warn{background:#FFF3DF;color:#9A5B00}.ck.bad{background:#FDEEF1;color:#B4233C}.ck.todo{background:#fff;border:1.5px solid #CBD2DC;color:#8A93A3}.ck.cur{background:#0A1A4A;color:#fff}
.q{display:grid;grid-template-columns:minmax(0,1fr) auto;gap:16px;align-items:center;padding:14px 0;border-bottom:1px solid #F1F3F6}
.q:last-child{border-bottom:0}
.yn{display:inline-flex;gap:6px}.yn button{height:36px;min-width:64px;border-radius:999px;border:1px solid #D9DDE5;background:#fff;font:600 14px Inter,sans-serif;color:#334155;cursor:pointer}
.yn button.on{background:#0A1A4A;border-color:#0A1A4A;color:#fff}
.hint{font-size:12.5px;margin-top:4px}
.sec-t{font-size:13px;font-weight:700;letter-spacing:.06em;text-transform:uppercase;color:#5B6474;margin:22px 0 4px}
.li2{display:flex;gap:12px;align-items:flex-start;padding:10px 0;border-bottom:1px solid #F1F3F6}.li2:last-child{border-bottom:0}
.rowsel{background:#F3F6FB}
'''
def page(title,active,body,h,user=None,extra=''):
    u=f' userName="{user[0]}" userRole="{user[1]}" userInitials="{user[2]}"' if user else ''
    return f'''<!doctype html>
<html lang="pt-BR">
<head>
<meta charset="utf-8">
<title>{title}</title>
<script src="./support.js"></script>
<link rel="stylesheet" href="{CSS}">
</head>
<body>
<x-dc>
<helmet>
<link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&amp;display=swap" rel="stylesheet">
<style>{BASE}{extra}</style>
</helmet>
<div class="app" style="font-family: Inter, sans-serif">
  <div class="side"><dc-import name="Sidebar" active="{active}"{u} hint-size="248px,1400px"></dc-import></div>
  <main class="content">
{body}
    <p class="t12 mute" style="margin: 0 8px">Mockup · dados fictícios</p>
  </main>
</div>
</x-dc>
<script type="text/x-dc" data-dc-script data-props='{{"$preview":{{"width":1440,"height":{h}}}}}'>
class Component extends DCLogic {{
  renderVals() {{ return {{}}; }}
}}
</script>
</body>
</html>
'''
def yn(sel): return f'<span class="yn"><button class="{"on" if sel=="s" else ""}">Sim</button><button class="{"on" if sel=="n" else ""}">Não</button></span>'
def q(t,sel,hint='',cls=''):
    h=f'<div class="hint {cls}">{hint}</div>' if hint else ''
    return f'<div class="q"><div><div class="t15">{t}</div>{h}</div>{yn(sel)}</div>'
def ptop(nome,meta,chips=''):
    return f'''    <section class="card row wrap" style="gap: 16px; padding: 18px 22px">
      <span class="av av-lg" style="background: #FDEEF1; color: #C2334D">{"".join(w[0] for w in nome.split()[:2])}</span>
      <div style="flex: 1 1 300px; min-width: 0"><h1 style="font-size: 22px; font-weight: 650">{nome}</h1><div class="t15 mute" style="margin-top: 3px">{meta}</div></div>
      {chips}
    </section>'''
# ---------- 1 Triagem ----------
b=f'''    <div class="topbar"><a class="t14 b" href="Sala-Exame.dc.html">← Sala de exame</a><h1 style="font-size:22px;font-weight:650;margin-left:12px">Triagem de segurança</h1><div class="grow"></div><span class="t13 mute">Feita por Enf. Lívia · 08:52 · antes de entrar na sala</span></div>
{ptop("Lúcia Fernandes Prado","02/07/1959 · 67 anos · F · TC de crânio com contraste · Sala 6",'<span class="chip c-rose">Alergia registrada: iodo</span>')}
    <div class="split">
      <section class="card main" style="padding: 8px 28px 20px">
        <div class="sec-t">Contraste iodado</div>
        {q("Já teve reação a contraste iodado?","s","Registrado em 2023: urticária leve após TC. Exige protocolo de pré-medicação e médico presente.","")}
        {q("Asma ou outra alergia importante?","n")}
        {q("Usa metformina?","s","Orientar suspender por 48 h após o exame se a função renal estiver reduzida.")}
        <div class="q"><div><div class="t15">Função renal</div><div class="hint mute">Creatinina 1,4 mg/dL em 28/10 · anexada pela recepção</div></div><div style="text-align:right"><div class="big" style="font-size:26px">TFG 40</div><div class="t12 mute">mL/min/1,73 m² · CKD-EPI</div></div></div>
        <div class="sec-t">Gestação e amamentação</div>
        {q("Existe possibilidade de gravidez?","n","Não se aplica (67 anos) · pergunta registrada assim mesmo")}
        {q("Está amamentando?","n")}
        <div class="sec-t">Outros</div>
        {q("Jejum de 4 horas cumprido?","s","Última refeição às 04:30")}
        {q("Acesso venoso difícil ou extravasamento prévio?","n")}
        <div class="sec-t" style="color:#8A93A3">Ressonância (não se aplica a este exame)</div>
        <p class="t13 mute" style="margin:6px 0 0">Marcapasso, implantes, clipes, claustrofobia e outros itens aparecem automaticamente quando o exame for de ressonância.</p>
      </section>
      <aside class="aside col" style="gap:16px;flex:1 1 320px;min-width:0">
        <section class="card" style="border:1px solid #F3C98B;box-shadow:none;background:#FFFBF4">
          <div class="lbl" style="color:#9A5B00">Resultado da triagem</div>
          <div style="font-size:20px;font-weight:650;margin-top:6px">Precisa de aprovação médica</div>
          <ul class="t14" style="margin:12px 0 0;padding-left:18px;line-height:1.6">
            <li>Reação prévia a contraste iodado</li>
            <li>TFG 40: abaixo de 45</li>
            <li>Usa metformina</li>
          </ul>
          <button class="btn btn-p" style="width:100%;margin-top:14px">Pedir aprovação ao radiologista</button>
          <button class="btn btn-sm" style="width:100%;margin-top:8px">Sugerir exame sem contraste</button>
          <p class="t12 mute" style="margin:10px 0 0">O exame fica bloqueado na sala até um médico aprovar e registrar a conduta (pré-medicação, hidratação, contraste de baixa osmolaridade).</p>
        </section>
        <section class="card">
          <div class="t15 b">Termo de consentimento do contraste</div>
          <div class="li2"><span class="ck todo"></span><div><div class="t14">Assinar no tablet</div><div class="t12 mute">Versão 4 do termo · liberado após a aprovação médica</div></div></div>
        </section>
        <section class="card" style="background:#F6F8FB;box-shadow:none">
          <div class="t14 b">Se houver reação</div>
          <p class="t13 mute" style="margin:6px 0 10px">Protocolo de reação a contraste e checklist do carrinho de emergência da Sala 6 (conferido hoje às 07:30 por Enf. Lívia).</p>
          <button class="btn btn-sm">Registrar reação</button>
        </section>
      </aside>
    </div>'''
open(P+'Triagem.dc.html','w').write(page('Triagem de segurança — VidaCor','sala',b,1450))
# ---------- 2 Laudo estruturado TI-RADS ----------
nods=[("N1","Lobo direito, terço médio","1,8 × 1,2 × 1,1 cm","Sólido (2)","Hipoecoico (2)","Mais largo que alto (0)","Lisas (0)","Nenhum (0)","4","TR4 · moderadamente suspeito","PAAF (≥ 1,5 cm)"),("N2","Lobo esquerdo, polo inferior","0,6 × 0,5 × 0,4 cm","Espongiforme (0)","—","—","—","—","0","TR1 · benigno","Sem seguimento")]
rows=''.join(f'<tr class="{"rowsel" if i==0 else ""}"><td><b>{a}</b></td><td>{b}</td><td class="num nw">{c}</td><td class="t13">{" · ".join(x for x in (d,e,f,g,h_) if x!="—")}</td><td class="r num b">{pt}</td><td><span class="chip {"c-amber" if "TR4" in tr else "c-green"}">{tr}</span></td><td>{rec}</td></tr>' for i,(a,b,c,d,e,f,g,h_,pt,tr,rec) in enumerate(nods))
b=f'''    <div class="topbar"><a class="t14 b" href="Laudo.dc.html">← Laudar</a><h1 style="font-size:22px;font-weight:650;margin-left:12px">Laudo estruturado</h1><div class="grow"></div><div class="seg"><button class="on">TI-RADS</button><button>BI-RADS</button><button>O-RADS</button><button>Lung-RADS</button><button>Bosniak</button><button>Obstetrícia</button></div></div>
{ptop("Rita Carvalho Barbosa","14/08/1974 · 52 anos · F · USG de tireoide · Dr. Marcos Freire")}
    <section class="card" style="padding:20px 8px 8px">
      <div class="row" style="padding:0 14px 10px"><h2 style="font-size:18px;font-weight:600">Nódulos (ACR TI-RADS)</h2><div class="grow"></div><button class="btn btn-sm">+ Nódulo</button><button class="btn btn-sm">Medidas do aparelho</button></div>
      <div class="scroll-x"><table class="tb"><thead><tr><th>#</th><th>Localização</th><th>Medidas</th><th>Características (pontos)</th><th class="r">Pontos</th><th>Categoria</th><th>Conduta</th></tr></thead><tbody>{rows}</tbody></table></div>
    </section>
    <div class="split">
      <section class="card main" style="padding:22px 28px">
        <div class="lbl">Texto gerado para o laudo (editável)</div>
        <p class="t15" style="line-height:1.7;margin:12px 0 0">Tireoide de dimensões normais, contornos regulares e ecotextura homogênea. No terço médio do lobo direito, nódulo sólido, hipoecoico, mais largo que alto, de margens lisas, sem focos ecogênicos, medindo 1,8 × 1,2 × 1,1 cm (<b>ACR TI-RADS 4</b>). No polo inferior do lobo esquerdo, nódulo espongiforme de 0,6 cm (<b>TI-RADS 1</b>).</p>
        <p class="t15" style="line-height:1.7;margin:12px 0 0"><b>Conclusão:</b> nódulo TI-RADS 4 no lobo direito com indicação de punção aspirativa por agulha fina (≥ 1,5 cm), a critério clínico.</p>
      </section>
      <aside class="aside col" style="gap:16px;flex:1 1 320px;min-width:0">
        <section class="card"><div class="t15 b">Como a categoria foi calculada</div><p class="t13 mute" style="margin:8px 0 0;line-height:1.6">Soma dos pontos de composição, ecogenicidade, forma, margens e focos (ACR TI-RADS). Categoria e conduta pela tabela da versão vigente, guardada no laudo (lastro).</p></section>
        <section class="card"><div class="t15 b">Comparação</div><div class="li2"><span class="ck warn">!</span><div><div class="t14">N1 cresceu</div><div class="t12 mute">1,3 cm em 03/2025 → 1,8 cm hoje</div></div></div></section>
        <section class="card" style="background:#F6F8FB;box-shadow:none"><div class="t14 b">Outros modelos estruturados</div><p class="t13 mute" style="margin:6px 0 0">BI-RADS (mama), O-RADS (ovário), LI-RADS (fígado), Lung-RADS (pulmão), Bosniak (cisto renal), eco (ASE) e obstetrícia (biometria com percentis e curvas).</p></section>
      </aside>
    </div>'''
open(P+'Laudo-Estruturado.dc.html','w').write(page('Laudo estruturado — VidaCor','laudos',b,1250,("Dr. Marcos Freire","USG e Doppler","MF")))
# ---------- 3 Exame externo ----------
st=[("ok","Receber o exame","link do portal da outra clínica"),("ok","Conferir paciente",""),("cur","Escolher estudos",""),("todo","Importar","")]
sh=''.join(f'<li class="row" style="gap:10px;flex:1 1 180px"><span class="ck {k}">{"✓" if k=="ok" else i+1}</span><div><div class="t14 b">{a}</div><div class="t12 mute">{b_}</div></div></li>' for i,(k,a,b_) in enumerate(st))
stud=[(1,"10/03/2025","TC de crânio sem contraste","Hospital São Lucas","1 série · 32 imagens","Laudo em PDF"),(1,"10/03/2025","TC de seios da face","Hospital São Lucas","2 séries · 88 imagens","—"),(0,"02/01/2024","Raio X de tórax (fotos enviadas pela paciente)","WhatsApp","2 fotos + laudo PDF","Só como anexo")]
sr=''.join(f'<label class="li2" style="align-items:center"><input type="checkbox" {"checked" if c else ""} style="width:20px;height:20px;accent-color:#0A1A4A"><div style="flex:1"><div class="t15 b">{e}</div><div class="t13 mute">{d} · {i_} · {n}</div></div><span class="t13 mute">{l}</span></label>' for c,d,e,i_,n,l in stud)
b=f'''    <div class="topbar"><h1 style="font-size:22px;font-weight:650">Importar exame externo</h1><div class="grow"></div><span class="t13 mute">Recepção · Ana Souza</span></div>
    <ol class="card row wrap" style="list-style:none;margin:0;gap:12px;padding:16px 22px">{sh}</ol>
    <div class="split">
      <section class="card main" style="padding:22px 28px">
        <h2 style="font-size:18px;font-weight:600">Conferência do paciente</h2>
        <div class="grid g2" style="margin-top:14px">
          <div style="background:#F6F8FB;border-radius:14px;padding:16px"><div class="lbl">No exame recebido</div><div class="t15 b" style="margin-top:6px">PRADO^LUCIA^F</div><div class="t14 mute">Nasc. 02/07/1959 · ID externo HSL-552190</div></div>
          <div style="background:#F6F8FB;border-radius:14px;padding:16px"><div class="lbl">Cadastro VidaCor</div><div class="t15 b" style="margin-top:6px">Lúcia Fernandes Prado</div><div class="t14 mute">Nasc. 02/07/1959 · prontuário 2026-004417</div></div>
        </div>
        <div class="li2" style="margin-top:8px"><span class="ck ok">✓</span><div><div class="t14">Nascimento confere · nome compatível (abreviado na origem)</div><div class="t12 mute">Confirmado por Ana Souza com o documento da paciente</div></div></div>
        <div class="sec-t">De onde pode vir</div><div class="row wrap t14" style="gap:10px;margin-top:6px"><span class="chip c-blue">Link ou QR do portal de outra clínica</span><span class="chip">Arquivos enviados pelo paciente (portal ou WhatsApp)</span><span class="chip">Rede parceira (DICOMweb)</span></div><h2 style="font-size:18px;font-weight:600;margin-top:22px">Estudos encontrados</h2>
        <div style="margin-top:6px">{sr}</div>
      </section>
      <aside class="aside col" style="gap:16px;flex:1 1 320px;min-width:0">
        <section class="card"><div class="t15 b">Como entra no VidaCor</div><ul class="t13" style="margin:10px 0 0;padding-left:18px;line-height:1.7"><li>Marcado como <b>exame externo</b>, com a instituição de origem</li><li>Somente leitura: não vira laudo da VidaCor</li><li>Disponível como <b>exame anterior</b> para comparação</li><li>Hash de cada imagem e do PDF; lastro de quem importou</li><li>Identificação corrigida nas imagens (perfil IHE PIR), com o original guardado</li></ul></section>
        <section class="card"><div class="t15 b">Vai ser comparado com</div><div class="li2"><span class="ck cur">→</span><div><div class="t14">TC de crânio com contraste · hoje · Sala 6</div><div class="t12 mute">O médico verá a TC externa lado a lado</div></div></div></section>
        <button class="btn btn-p">Importar 2 estudos</button>
        <p class="t12 mute" style="margin:0">O link expira na origem; a cópia fica no PACS da VidaCor, com a origem registrada.</p>
      </aside>
    </div>'''
open(P+'Exame-Externo.dc.html','w').write(page('Importar exame externo — VidaCor','pacientes',b,1150))
# ---------- 4 Autorização ----------
fila=[("Unimed","Antônio C. Ribeiro","Eco + Doppler carótidas","Autorizada · senha UN-88231-X","ok",1),("SulAmérica","Lúcia F. Prado","TC crânio c/ contraste","Em análise · 6 h","cur",0),("Bradesco Saúde","Juliana S. Rocha","USG morfológica","Pendente: pedir relatório","warn",0),("Amil","Paulo R. Dias","Eco transtorácico","Negada · fora da cobertura","bad",0),("Petrobras","Marta L. Siqueira","Doppler venoso MMII","Enviada pelo portal (manual)","todo",0)]
fr=''.join(f'<tr class="{"rowsel" if s else ""}"><td class="nw"><b>{o}</b></td><td class="nw">{p}</td><td>{e}</td><td><span class="row" style="gap:8px"><span class="ck {k}">{"✓" if k=="ok" else ("!" if k in ("warn","bad") else "")}</span>{st_}</span></td></tr>' for o,p,e,st_,k,s in fila)
tl=[("ok","Solicitação montada","Do agendamento: carteirinha, TUSS 40901114 e 40901300, CID I10, pedido anexado","05/11 16:05"),("ok","Enviada à Unimed","Webservice TISS · solicitação SP/SADT · protocolo 1180-77231","05/11 16:05"),("ok","Em análise","Operadora recebeu","05/11 16:06"),("ok","Autorizada","Senha UN-88231-X · válida até 20/11 · 1 de cada procedimento","05/11 17:40"),("ok","Guia preenchida no agendamento","Sem digitação: dados vieram da resposta","05/11 17:40")]
tlh=''.join(f'<div class="li2"><span class="ck {k}">✓</span><div style="flex:1"><div class="t14 b">{a}</div><div class="t12 mute">{b_}</div></div><span class="t12 mute num">{c}</span></div>' for k,a,b_,c in tl)
b=f'''    <div class="topbar"><h1 style="font-size:22px;font-weight:650">Autorizações</h1><div class="grow"></div><button class="btn btn-p">Nova solicitação</button></div>
    <div class="row wrap t14 mute" style="gap:24px;padding:0 8px"><span><b style="color:#0F172A">18</b> em andamento</span><span><b style="color:#1F7A4D">41</b> autorizadas na semana</span><span><b style="color:#9A5B00">3</b> pendentes de documento</span><span><b style="color:#B4233C">2</b> negadas</span><span>tempo médio <b style="color:#0F172A">1 h 35</b></span></div>
    <div class="col" style="gap:16px">
      <section class="card" style="padding:20px 8px 8px">
        <div class="row" style="padding:0 14px 10px"><h2 style="font-size:18px;font-weight:600">Fila de autorizações</h2><div class="grow"></div><div class="seg"><button class="on btn-sm" style="height:36px">Todas</button><button style="height:36px">Pendentes</button><button style="height:36px">Negadas</button></div></div>
        <div class="scroll-x"><table class="tb"><thead><tr><th>Operadora</th><th>Paciente</th><th>Exame</th><th>Situação</th></tr></thead><tbody>{fr}</tbody></table></div>
        <div style="margin:14px;padding:14px 16px;border-radius:12px;background:#F6F8FB" class="t13 mute">Operadoras com integração (webservice TISS) recebem e respondem automaticamente. Para as demais, o sistema monta o pedido, a equipe envia pelo portal da operadora e registra a resposta: a solicitação fica rastreada do mesmo jeito.</div>
      </section>
      <div class="grid g2">
        <section class="card">
          <div class="lbl">Unimed · Antônio Carlos Ribeiro · 15/06/1954</div>
          <h2 style="font-size:18px;font-weight:650;margin-top:6px">Autorizada</h2>
          <div style="margin-top:8px">{tlh}</div>
          <div class="row wrap" style="gap:8px;margin-top:12px"><a class="btn btn-sm" href="Agendamento-Guia.dc.html">Abrir agendamento</a><a class="btn btn-sm" href="Lastro.dc.html">Ver lastro</a></div>
        </section>
        <section class="card" style="background:#F6F8FB;box-shadow:none"><div class="t14 b">Negada ou pendente</div><p class="t13 mute" style="margin:6px 0 0">Negativa traz o motivo da operadora e as saídas: recurso, particular com orçamento ou remarcação. Pendência de documento avisa o paciente pelo WhatsApp com o que falta.</p></section>
      </aside>
    </div>'''
open(P+'Autorizacao.dc.html','w').write(page('Autorizações — VidaCor','faturamento',b,1200,("Carla Mendes","Faturamento","CM")))
# ---------- 5 Auditoria pré-faturamento ----------
regras=[("bad","Senha vencida antes da data do exame","Todas","2","Bloqueia o envio","Pedir prorrogação"),("bad","Procedimento executado diferente do autorizado","Todas","1","Bloqueia o envio","Corrigir ou pedir nova senha"),("bad","Laudo não assinado","Todas","1","Bloqueia o envio","Avisar o médico"),("warn","CID-10 obrigatório ausente","SulAmérica","4","Corrigir antes","Preencher CID"),("warn","Assinatura do beneficiário ausente na guia","Bradesco Saúde","3","Corrigir antes","Coletar no próximo contato"),("warn","Valor diferente da tabela vigente","Unimed","2","Corrigir antes","Aplicar tabela 2026"),("warn","Prazo de envio vence em 7 dias","Amil","2","Atenção","Enviar no próximo lote"),("ok","Carteirinha válida, elegibilidade e senha","Todas","198","OK","—")]
rr=''.join(f'<tr><td><span class="row" style="gap:10px"><span class="ck {k}">{"✓" if k=="ok" else "!"}</span>{r}</span></td><td class="nw">{o}</td><td class="r num b">{n}</td><td><span class="chip {"c-rose" if k=="bad" else ("c-amber" if k=="warn" else "c-green")}">{ef}</span></td><td>{a}</td></tr>' for k,r,o,n,ef,a in regras)
b=f'''    <div class="topbar"><h1 style="font-size:22px;font-weight:650">Auditoria antes do faturamento</h1><div class="grow"></div><div class="seg"><button class="on">Novembro · 1ª quinzena</button><button>Por operadora</button></div><button class="btn btn-p">Gerar lotes das guias aprovadas</button></div>
    <section class="grid g4">
      <div class="card"><div class="lbl">Guias conferidas</div><div class="big" style="margin-top:6px">214</div></div>
      <div class="card"><div class="lbl">Prontas para enviar</div><div class="big" style="margin-top:6px;color:#1F7A4D">198</div></div>
      <div class="card"><div class="lbl">Corrigir antes</div><div class="big" style="margin-top:6px;color:#9A5B00">12</div></div>
      <div class="card"><div class="lbl">Bloqueadas</div><div class="big" style="margin-top:6px;color:#B4233C">4</div><div class="t13 mute" style="margin-top:6px">glosa evitada estimada R$ 3.870</div></div>
    </section>
    <div class="col" style="gap:16px">
      <section class="card" style="padding:20px 8px 8px">
        <div class="row" style="padding:0 14px 10px"><h2 style="font-size:18px;font-weight:600">O que a auditoria encontrou</h2></div>
        <div class="scroll-x"><table class="tb"><thead><tr><th>Regra</th><th>Operadora</th><th class="r">Guias</th><th>Efeito</th><th>O que fazer</th></tr></thead><tbody>{rr}</tbody></table></div>
      </section>
      <div class="grid g2">
        <section class="card"><div class="t15 b">Regras por operadora</div>
          <div class="li2"><div style="flex:1"><div class="t14 b">Unimed Costa do Sol</div><div class="t12 mute">24 regras · versão 2026.11 · revisada por Carla</div></div></div>
          <div class="li2"><div style="flex:1"><div class="t14 b">SulAmérica</div><div class="t12 mute">19 regras · exige CID em SP/SADT</div></div></div>
          <div class="li2"><div style="flex:1"><div class="t14 b">Bradesco Saúde</div><div class="t12 mute">17 regras · assinatura do beneficiário</div></div></div>
          <div class="li2"><div style="flex:1"><div class="t14 b">Amil · Petrobras</div><div class="t12 mute">14 · 21 regras</div></div></div>
          <button class="btn btn-sm" style="margin-top:8px">Editar regras (versão nova)</button>
        </section>
        <section class="card" style="background:#F6F8FB;box-shadow:none"><div class="t14 b">De onde vêm as regras</div><p class="t13 mute" style="margin:6px 0 0">Do manual de cada operadora, do padrão TISS e das <b>glosas que já aconteceram</b>: cada glosa recebida sugere uma regra nova para não repetir.</p></section>
      </div>
    </div>'''
open(P+'Auditoria-Faturamento.dc.html','w').write(page('Auditoria antes do faturamento — VidaCor','faturamento',b,1150,("Carla Mendes","Faturamento","CM")))
print('ok')
