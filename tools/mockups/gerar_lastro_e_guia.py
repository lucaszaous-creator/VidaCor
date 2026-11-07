P='/tmp/claude-0/-home-user/4fd689ae-5e0d-5bbd-bb63-c60bbbb26a18/scratchpad/vidacor-canvas/project/'
CSS='/_blob/3428af26abb0af564c467191e42d3475'
def page(title,style,body,h):
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
<style>{style}</style>
</helmet>
{body}
</x-dc>
<script type="text/x-dc" data-dc-script data-props='{{"$preview":{{"width":1440,"height":{h}}}}}'>
class Component extends DCLogic {{
  renderVals() {{ return {{}}; }}
}}
</script>
</body>
</html>
'''
# ================= Agendamento com guia =================
steps=[("ok","Paciente","Antônio Carlos Ribeiro · 15/06/1954"),("ok","Exame e horário","Eco + Doppler · 12/11 08:30 · Sala 1"),("cur","Convênio e guia",""),("todo","Preparo e confirmação","")]
sh=''.join(f'<li class="sp {k}"><span class="ck {k}">{"✓" if k=="ok" else i+1}</span><div style="min-width:0"><div class="t14 b">{a}</div><div class="t12 mute clamp1">{b}</div></div></li>' for i,(k,a,b) in enumerate(steps))
val=[("ok","Carteirinha confere com o paciente","0 064 123456789012 3 · válida até 12/2027"),("ok","Elegível hoje","consultado na Unimed às 10:14"),("ok","Procedimentos autorizados cobrem o agendado","TUSS 40901114 e 40901300 · 1 de cada"),("ok","Senha válida na data do exame","válida até 20/11 · exame em 12/11"),("ok","Guia ainda não usada","nenhum outro atendimento com esta guia"),("warn","Pedido médico com mais de 30 dias?","emitido em 28/10 · dentro do prazo da Unimed (60 dias)")]
vh=''.join(f'<li class="vl"><span class="ck {"ok" if k=="ok" else "warn"}">{"✓" if k=="ok" else "!"}</span><div><div class="t14">{a}</div><div class="t12 mute">{b}</div></div></li>' for k,a,b in val)
st1='''body{margin:0;background:#F2F3F6}
.sp{display:flex;gap:12px;align-items:center;padding:12px 14px;border-radius:12px;min-width:0}
.sp.cur{background:#EEF3FA}
.ck{width:26px;height:26px;border-radius:50%;display:flex;align-items:center;justify-content:center;flex:none;font-size:13px;font-weight:700}
.ck.ok{background:#E5F5EC;color:#1F7A4D}.ck.todo{background:#fff;border:1.5px solid #CBD2DC;color:#8A93A3}.ck.cur{background:#0A1A4A;color:#fff}.ck.warn{background:#FFF3DF;color:#9A5B00}
.vl{display:flex;gap:12px;align-items:flex-start;padding:9px 0;border-bottom:1px solid #F1F3F6}.vl:last-child{border-bottom:0}
.fs{display:grid;grid-template-columns:repeat(auto-fit,minmax(min(100%,220px),1fr));gap:14px}
.att{display:flex;gap:10px;align-items:center;border:1px solid #E3E7ED;border-radius:12px;padding:10px 12px;background:#fff}
.thumb{width:44px;height:56px;border-radius:6px;background:linear-gradient(180deg,#F1F3F6,#E3E7ED);display:flex;align-items:center;justify-content:center;font-size:10px;color:#5B6474;flex:none}
'''
b1=f'''<div class="app" style="font-family: Inter, sans-serif">
  <div class="side"><dc-import name="Sidebar" active="agenda" hint-size="248px,1400px"></dc-import></div>
  <main class="content">
    <div class="topbar">
      <a class="t14 b" href="Agenda.dc.html">← Agenda</a>
      <h1 style="font-size: 24px; font-weight: 650; margin-left: 12px">Novo agendamento</h1>
      <div class="grow"></div>
      <span class="t13 mute">Rascunho salvo · reserva do horário por 10 min</span>
    </div>
    <ol class="card" style="list-style: none; margin: 0; display: grid; grid-template-columns: repeat(4, minmax(0, 1fr)); gap: 8px; padding: 10px" aria-label="Etapas do agendamento">{sh}</ol>

    <div class="split">
      <section class="card main" aria-labelledby="g-h" style="padding: 24px 28px">
        <div class="row wrap" style="gap: 12px">
          <div><h2 id="g-h" style="font-size: 20px; font-weight: 650">Convênio e guia de autorização</h2>
          <p class="t14 mute" style="margin: 4px 0 0">A VidaCor só atende convênio com a guia já autorizada. Sem guia, o horário fica só como reserva provisória.</p></div>
          <div class="grow"></div>
          <div class="seg" role="radiogroup" aria-label="Forma de pagamento"><button class="on" role="radio" aria-checked="true">Convênio</button><button role="radio" aria-checked="false">Particular</button></div>
        </div>

        <h3 class="lbl" style="margin: 22px 0 10px">Beneficiário</h3>
        <div class="fs">
          <label class="field">Operadora<select class="in"><option>Unimed Costa do Sol</option></select></label>
          <label class="field">Plano<input class="in" value="Unimed Pleno · Apartamento"></label>
          <label class="field">Carteirinha<input class="in num" value="0 064 123456789012 3"></label>
          <label class="field">Validade da carteirinha<input class="in" value="12/2027"></label>
        </div>

        <h3 class="lbl" style="margin: 24px 0 10px">Guia de autorização</h3>
        <div class="fs">
          <label class="field">Nº da guia na operadora<input class="in num" value="4471230"></label>
          <label class="field">Senha de autorização<input class="in num" value="UN-88231-X"></label>
          <label class="field">Data da autorização<input class="in" value="30/10/2026"></label>
          <label class="field">Senha válida até<input class="in" value="20/11/2026"></label>
          <label class="field">Caráter do atendimento<select class="in"><option>Eletivo</option></select></label>
          <label class="field">Tipo de guia TISS<select class="in"><option>SP/SADT</option></select></label>
        </div>

        <h3 class="lbl" style="margin: 24px 0 10px">Procedimentos autorizados</h3>
        <div class="scroll-x"><table class="tb"><thead><tr><th>Código TUSS</th><th>Procedimento</th><th class="r">Autorizado</th><th class="r">Neste agendamento</th><th class="r">Saldo</th></tr></thead><tbody>
          <tr><td class="num nw">40901114</td><td>Ecocardiograma transtorácico</td><td class="r num">1</td><td class="r num">1</td><td class="r num">0</td></tr>
          <tr><td class="num nw">40901300</td><td>Doppler colorido de carótidas</td><td class="r num">1</td><td class="r num">1</td><td class="r num">0</td></tr>
        </tbody></table></div>

        <h3 class="lbl" style="margin: 24px 0 10px">Solicitante</h3>
        <div class="fs">
          <label class="field">Médico solicitante<input class="in" value="Dr. Hugo Lemos"></label>
          <label class="field">CRM / UF<input class="in" value="[000000] / RJ"></label>
          <label class="field">CBO<input class="in" value="225120 · Cardiologista"></label>
          <label class="field">Data do pedido<input class="in" value="28/10/2026"></label>
          <label class="field">CID-10 (se a operadora exigir)<input class="in" value="I10 · Hipertensão essencial"></label>
          <label class="field">Indicação clínica<input class="in" value="Controle de HAS; avaliar função"></label>
        </div>

        <h3 class="lbl" style="margin: 24px 0 10px">Anexos</h3>
        <div class="row wrap" style="gap: 10px">
          <div class="att"><span class="thumb">PDF</span><div><div class="t14 b">Pedido médico</div><div class="t12 mute">foto · 28/10 · SHA-256 3f9a…c21</div></div></div>
          <div class="att"><span class="thumb">PDF</span><div><div class="t14 b">Autorização Unimed</div><div class="t12 mute">PDF do portal · 30/10 · SHA-256 a71d…09e</div></div></div>
          <button class="btn btn-sm">+ Fotografar ou anexar</button>
        </div>
      </section>

      <aside class="aside col" style="gap: 16px; flex: 0 1 380px; min-width: 340px">
        <section class="card" aria-labelledby="v-h">
          <h2 id="v-h" style="font-size: 16px; font-weight: 600; margin-bottom: 4px">Conferência automática</h2>
          <ul style="list-style: none; margin: 0; padding: 0">{vh}</ul>
        </section>
        <section class="card" aria-labelledby="l-h" style="background: #F6F8FB; box-shadow: none">
          <h2 id="l-h" style="font-size: 15px; font-weight: 600">Lastro desta guia</h2>
          <div class="t13" style="margin-top: 8px; line-height: 1.6">Digitada por <b>Ana Souza</b> · Guichê 2 · 10:16<br>Elegibilidade: protocolo <span class="num">ELG-77120</span><br>Vai ligar: agendamento → check-in → exame (ACC) → guia TISS → lote → pagamento</div>
          <a class="t14 b" href="Lastro.dc.html" style="display: inline-block; margin-top: 8px">Ver no explorador de lastro</a>
        </section>
        <section class="card" aria-label="Sem guia" style="border: 1px dashed #F3C98B; box-shadow: none">
          <div class="t14 b">Ainda sem guia?</div>
          <p class="t13 mute" style="margin: 6px 0 10px">O horário vira <b>reserva provisória por 48 h</b>. O paciente recebe pelo WhatsApp o pedido para enviar a autorização. Sem guia até o prazo, a vaga é liberada e oferecida à lista de espera.</p>
          <button class="btn btn-sm">Reservar sem guia (48 h)</button>
        </section>
      </aside>
    </div>

    <div class="row wrap" style="gap: 12px; background: #fff; border-radius: 18px; padding: 14px 20px; box-shadow: 0 1px 2px rgba(15,23,42,.05)">
      <span class="t14 mute" style="flex: 1 1 300px">A guia fica reservada para este agendamento: outro atendimento não consegue usá-la.</span>
      <button class="btn">Voltar</button>
      <button class="btn btn-p">Continuar para preparo e confirmação</button>
    </div>
  </main>
</div>'''
open(P+'Agendamento-Guia.dc.html','w').write(page('Novo agendamento · guia — VidaCor',st1,b1,1600))

# ================= Explorador de lastro =================
phases=[
("Agendamento",[("AGD-2026-031877","Agendamento","Criado pelo WhatsApp (robô) · confirmado por Ana Souza · 05/11 16:02","ok"),("CONF-88412","Confirmação","WhatsApp · modelo “confirmação D-1” v3 · lida 05/11 18:10","ok")]),
("Autorização",[("GUIA 4471230","Guia da operadora","Senha UN-88231-X · válida até 20/11 · 2 procedimentos TUSS","ok"),("ELG-77120","Elegibilidade","Unimed · elegível · 06/11 08:21","ok"),("DOC 3f9a…c21","Pedido médico","Foto · Dr. Hugo Lemos CRM [000000]/RJ · 28/10","ok")]),
("Chegada",[("ATD-2026-118790","Atendimento","Check-in 08:21 · Guichê 2 · Ana Souza","ok"),("ETQ-2026-018734","Pulseira","Impressa 08:24 · bipada na Sala 1 às 08:41","ok"),("TERMO v4","Termo de consentimento","Assinado no tablet · 08:23 · hash 9b2e…71f","ok"),("PLACA QRT4B21","Estacionamento","Entrada 08:12 · cortesia validada pelo exame","ok")]),
("Exame",[("ACC 2026-118790","Pedido de exame (accession)","Worklist enviada ao GE Vivid E95 (VIVID_S1)","ok"),("MPPS 1.2.840…5521","Execução no aparelho","Início 08:41 · fim 09:02 · Téc. Carla","ok"),("STUDY 1.2.840…88.1","Estudo DICOM","38 imagens · 4 vídeos · 18 medidas (SR)","ok"),("DESINF T-07","Transdutor","Desinfecção nível alto · ciclo 07:55 · lote DES-221","ok")]),
("Laudo",[("LAU-2026-044120 v1","Laudo","Rascunho com medidas do aparelho · Dr. Paulo","ok"),("ASS ICP 5F:2A…","Assinatura digital","Certificado ICP-Brasil · carimbo do tempo 10:42","ok"),("PORTAL","Publicação","Liberado no portal 10:43 · aviso WhatsApp lido 11:02","ok")]),
("Faturamento",[("GUIA TISS 900812","Guia SP/SADT","Gerada do atendimento · XML TISS 4.01","ok"),("LOTE 2026-11-03","Lote","Enviado 07/11 · protocolo ANS 7781203","cur"),("DEMONSTRATIVO","Retorno da operadora","aguardando","todo")]),
("Financeiro",[("PAGTO","Recebimento","aguardando o demonstrativo","todo"),("REPASSE","Repasse ao médico","regra 40% · competência 11/2026","todo")]),
]
def node(i,t,d,k,sel=False):
    return f'<button class="nd {k}{" sel" if sel else ""}"><span class="ck {k}">{"✓" if k=="ok" else ("…" if k=="cur" else "")}</span><span class="nd-b"><span class="num nd-i">{i}</span><b>{t}</b><span class="t12 mute">{d}</span></span></button>'
ph=''.join(f'<section class="ph"><div class="ph-h">{n}</div><div class="ph-n">{"".join(node(*x, sel=(x[0]=="STUDY 1.2.840…88.1")) for x in items)}</div></section>' for n,items in phases)
meta=[("Identificador","StudyInstanceUID 1.2.840.113619.2.55.3.604688.88.1"),("Criado","06/11/2026 08:41:12 · pelo aparelho VIVID_S1 (10.0.20.11)"),("Recebido pelo VidaCor","08:41:13 → 09:02:40 · 42 objetos DICOM"),("Integridade","SHA-256 de cada objeto guardado · 0 divergências"),("Conciliação","Paciente e ACC conferidos automaticamente (nome + nascimento + accession)"),("Armazenamento","PACS local · réplica no servidor de contingência · backup externo 07/11 02:10"),("Retenção","20 anos (prontuário) · até 11/2046"),("Acessos","Dr. Paulo (laudo) · Téc. Carla (execução) · paciente (portal, 2 imagens-chave)"),("Ligado a","ACC 2026-118790 · MPPS 1.2.840…5521 · LAU-2026-044120 v1 · ETQ-2026-018734")]
mh=''.join(f'<div class="kv"><span>{a}</span><b>{b}</b></div>' for a,b in meta)
st2='''body{margin:0;background:#F2F3F6}
.ph{display:grid;grid-template-columns:150px minmax(0,1fr);gap:14px;padding:14px 0;border-bottom:1px solid #F1F3F6}
.ph:last-child{border-bottom:0}
.ph-h{font-size:13px;font-weight:700;letter-spacing:.06em;text-transform:uppercase;color:#5B6474;padding-top:12px}
.ph-n{display:grid;grid-template-columns:repeat(auto-fill,minmax(250px,1fr));gap:10px}
.nd{display:flex;gap:10px;align-items:flex-start;text-align:left;background:#fff;border:1px solid #E8EAEF;border-radius:12px;padding:10px 12px;font:inherit;color:inherit;cursor:pointer;min-width:0}
.nd:hover{border-color:#C9D2DE}.nd.sel{border-color:#0A1A4A;box-shadow:0 0 0 1px #0A1A4A}
.nd.todo{background:#FAFBFC;border-style:dashed}
.nd-b{display:flex;flex-direction:column;gap:2px;min-width:0}
.nd-b b{font-size:14px;font-weight:600}
.nd-i{font-size:11.5px;color:#0C72B0;font-weight:600;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
.ck{width:22px;height:22px;border-radius:50%;display:flex;align-items:center;justify-content:center;flex:none;font-size:12px;font-weight:700;margin-top:2px}
.ck.ok{background:#E5F5EC;color:#1F7A4D}.ck.cur{background:#EFEBFC;color:#5B45B5}.ck.todo{background:#fff;border:1.5px solid #CBD2DC}
.kv{display:flex;flex-direction:column;gap:2px;padding:10px 0;border-bottom:1px solid #F1F3F6;font-size:14px}
.kv:last-child{border-bottom:0}.kv span{font-size:12px;color:#5B6474;font-weight:600;text-transform:uppercase;letter-spacing:.05em}.kv b{font-weight:500;word-break:break-word}
'''
b2=f'''<div class="app" style="font-family: Inter, sans-serif">
  <div class="side"><dc-import name="Sidebar" active="lastro" hint-size="248px,1400px"></dc-import></div>
  <main class="content">
    <div class="topbar">
      <h1 style="font-size: 24px; font-weight: 650">Lastro</h1>
      <div class="grow"></div>
      <label class="search" style="max-width: 520px; flex: 1 1 380px; min-height: 44px"><svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" aria-hidden="true"><path d="M11 18a7 7 0 1 0 0-14 7 7 0 0 0 0 14zM21 21l-4.3-4.3"></path></svg><span class="sr">Buscar por qualquer código</span><input type="search" value="ACC 2026-118790" placeholder="Qualquer código: ACC, guia, ETQ, StudyUID, NFS-e, PIX, lote, placa"></label>
      <button class="btn btn-sm">Exportar dossiê (PDF assinado)</button>
    </div>

    <section class="card row wrap" style="gap: 18px; padding: 18px 22px" aria-label="Resumo">
      <div style="flex: 1 1 320px"><div class="lbl">Atendimento ATD-2026-118790</div><div style="font-size: 20px; font-weight: 650; margin-top: 4px">Antônio Carlos Ribeiro · 15/06/1954</div><div class="t14 mute" style="margin-top: 2px">Eco + Doppler de carótidas · Unimed · 06/11/2026</div></div>
      <div class="row wrap t14" style="gap: 20px"><span><b>21</b> <span class="mute">registros ligados</span></span><span><b>5</b> <span class="mute">documentos com hash</span></span><span><b>3</b> <span class="mute">integrações</span></span><span><b style="color:#5B45B5">2</b> <span class="mute">etapas em andamento</span></span></div>
    </section>

    <div class="split">
      <section class="card main" aria-label="Cadeia do atendimento" style="padding: 8px 22px">{ph}</section>
      <aside class="aside" style="flex: 0 1 400px; min-width: 340px">
        <section class="card" aria-labelledby="m-h">
          <div class="lbl">Registro selecionado</div>
          <h2 id="m-h" style="font-size: 18px; font-weight: 650; margin-top: 6px">Estudo DICOM</h2>
          <div style="margin-top: 8px">{mh}</div>
          <div class="row wrap" style="gap: 8px; margin-top: 12px"><a class="btn btn-sm" href="Visualizador.dc.html">Abrir imagens</a><button class="btn btn-sm">Histórico de eventos</button></div>
        </section>
      </aside>
    </div>
    <p class="t12 mute" style="margin: 0 8px">Mockup · dados e identificadores fictícios</p>
  </main>
</div>'''
open(P+'Lastro.dc.html','w').write(page('Explorador de lastro — VidaCor',st2,b2,1650))
print('ok')
