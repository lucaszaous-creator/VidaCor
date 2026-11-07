import html as H
OUT='/tmp/claude-0/-home-user/4fd689ae-5e0d-5bbd-bb63-c60bbbb26a18/scratchpad/vidacor-canvas/project/Menus-Contexto.dc.html'
I={ # stroke icon paths
'open':'M14 3h7v7M21 3l-9 9M5 5h5M5 5v14h14v-5',
'check':'M5 12l5 5 9-10','id':'M3 5h18v14H3zM7 15c.8-1.5 2-2 3-2s2.2.5 3 2M10 11a2 2 0 1 0 0-4 2 2 0 0 0 0 4z',
'tv':'M3 5h18v12H3zM8 21h8','move':'M5 12h14M13 6l6 6-6 6','star':'M12 3l2.7 5.6 6.1.9-4.4 4.3 1 6.1-5.4-2.9-5.4 2.9 1-6.1L3.2 9.5l6.1-.9z',
'chat':'M21 12a8 8 0 0 1-11.6 7.1L4 20l1-4.6A8 8 0 1 1 21 12z','cash':'M3 7h18v12H3zM12 10v6M9.5 12.5h5',
'cal':'M4 5h16v16H4zM4 10h16M9 3v4M15 3v4','x':'M6 6l12 12M18 6L6 18','print':'M6 9V3h12v6M6 18H4v-7h16v7h-2M6 14h12v7H6z',
'copy':'M8 8h12v12H8zM4 16V4h12','lock':'M5 11h14v10H5zM8 11V8a4 4 0 0 1 8 0v3','user':'M12 12a4 4 0 1 0 0-8 4 4 0 0 0 0 8zM4 21a8 8 0 0 1 16 0',
'clock':'M12 7v5l3 2M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18z','block':'M5 5l14 14M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18z',
'doc':'M14 3H6v18h12V7zM14 3v4h4','img':'M3 5h18v14H3zM3 16l5-5 4 4 3-3 6 6','eye':'M2 12s4-7 10-7 10 7 10 7-4 7-10 7S2 12 2 12zM12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6z',
'ruler':'M3 17L17 3l4 4L7 21zM7 13l2 2M10 10l2 2M13 7l2 2','pen':'M4 20h4L19 9l-4-4L4 16zM14 6l4 4','key':'M14 8a4 4 0 1 1-1.2 2.8L4 19v2h3v-2h2v-2h2l1.8-1.8',
'undo':'M9 14L4 9l5-5M4 9h11a5 5 0 0 1 0 10h-3','send':'M22 2L11 13M22 2l-7 20-4-9-9-4z','layers':'M12 3l9 5-9 5-9-5zM3 13l9 5 9-5',
'merge':'M6 3v6a6 6 0 0 0 6 6h6M18 9l3 6-3 6','search':'M11 18a7 7 0 1 0 0-14 7 7 0 0 0 0 14zM21 21l-4.3-4.3','mic':'M9 3h6v11H9zM5 11a7 7 0 0 0 14 0M12 18v3',
'flag':'M5 21V4h12l-2 4 2 4H5','bolt':'M13 2L4 14h7l-1 8 9-12h-7z','grid':'M4 4h7v7H4zM13 4h7v7h-7zM4 13h7v7H4zM13 13h7v7h-7z','info':'M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18zM12 11v6M12 7h.01',
}
def ico(k,c='currentColor'):
    return f'<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="{c}" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="{I[k]}"></path></svg>' if k else '<span style="width:16px"></span>'
def menu(items,top,left,w=290,dark=False,title=None,search=False,foot=None,label='Menu de contexto'):
    cls='cm dk' if dark else 'cm'
    out=[f'<div class="{cls}" role="menu" aria-label="{label}" style="top:{top}px;left:{left}px;width:{w}px">']
    if title: out.append(f'<div class="cm-t">{title}</div>')
    if search: out.append(f'<div class="cm-s">{ico("search")}<span>Digite para filtrar…</span></div>')
    for it in items:
        if it=='-': out.append('<div class="cm-sep" role="separator"></div>'); continue
        if isinstance(it,str) and it.startswith('#'): out.append(f'<div class="cm-h">{it[1:]}</div>'); continue
        k=it.get('i'); lab=it['t']; sc=it.get('k',''); sub=it.get('sub'); st=it.get('s','')
        right=f'<span class="cm-k">{sc}</span>' if sc else ''
        if sub: right='<span class="cm-k">›</span>'
        why=f'<span class="cm-why">{it["why"]}</span>' if it.get('why') else ''
        dis=' aria-disabled="true"' if st=='off' else ''
        role='menuitemcheckbox' if 'chk' in it else 'menuitem'
        chk=''
        if 'chk' in it: chk=f' aria-checked="{"true" if it["chk"] else "false"}"'; k='check' if it['chk'] else None
        out.append(f'<div class="cm-i {st}" role="{role}"{dis}{chk}>{ico(k)}<span class="cm-l">{lab}{why}</span>{right}</div>')
    if foot: out.append(f'<div class="cm-f">{ico("lock")}<span>{foot}</span></div>')
    out.append('</div>'); return ''.join(out)
def cursor(top,left): return f'<svg class="cur" style="top:{top}px;left:{left}px" width="22" height="22" viewBox="0 0 24 24" aria-hidden="true"><path d="M4 2l15 11-7 1 4 7-3 1-4-7-5 5z" fill="#0F172A" stroke="#fff" stroke-width="1.5"></path></svg><span class="clk" style="top:{top-6}px;left:{left-6}px"></span>'
def scene(n,title,desc,stage_h,stage,notes):
    nt=''.join(f'<li>{x}</li>' for x in notes)
    return f'''<section class="card" aria-labelledby="sc{n}" style="padding:22px">
  <div class="row" style="align-items:flex-start;gap:16px;flex-wrap:wrap">
    <span class="tile t-navy" style="font-weight:700">{n}</span>
    <div style="flex:1 1 400px;min-width:0"><h2 id="sc{n}" style="font-size:19px;font-weight:650">{title}</h2><p class="t14 mute" style="margin:4px 0 0">{desc}</p></div>
  </div>
  <div class="split" style="margin-top:16px">
    <div class="main stage" style="height:{stage_h}px">{stage}</div>
    <ul class="aside notes">{nt}</ul>
  </div>
</section>'''
def kc(n,x,m,sel=False,extra=''): return f'<div class="kc{" sel" if sel else ""}" {extra}><b>{n}</b><span class="x">{x}</span><span class="m">{m}</span></div>'
S=[]
# 1 Esteira
st=f'''<div class="mini-cols">
<div><div class="colh">Check-in</div>{kc("Antônio C. Ribeiro","Eco + Doppler","P012 · etapa 5 de 6",True)}{kc("Juliana S. Rocha","Morfológica","Falta pedido")}{kc("Rafael M. Lima","Raio X","Falta pagamento")}</div>
<div><div class="colh">Aguardando a sala</div>{kc("Lúcia F. Prado","TC crânio","Sala 6 · 4 min")}{kc("Tiago S. Braga","Raio X","Sala 5 · 12 min")}</div>
<div style="opacity:.45"><div class="colh">Em exame</div>{kc("Sônia M. Castro","Eco","Sala 1")}</div></div>
{cursor(92,210)}
{menu([{"i":"open","t":"Abrir ficha do paciente","k":"Enter"},{"i":"id","t":"Check-in","sub":1},{"i":"tv","t":"Chamar na TV","k":"C"},{"i":"move","t":"Mover para","sub":1},'-',{"i":"star","t":"Marcar como preferencial…","k":"P"},{"i":"chat","t":"Enviar pelo WhatsApp","sub":1},{"i":"cash","t":"Receber no caixa","k":"F4"},{"i":"cal","t":"Remarcar…","k":"R"},{"i":"print","t":"Imprimir etiqueta","k":"Ctrl+P"},'-',{"i":"flag","t":"Registrar desistência…","s":"danger"},{"i":"x","t":"Cancelar atendimento…","s":"off","why":"Só supervisor da recepção"}],98,222,title="Antônio Carlos Ribeiro · 15/06/1954",search=True,foot="Toda ação fica na auditoria")}
{menu(["#Etapas do check-in",{"i":"check","t":"Identidade conferida","s":"done"},{"i":"check","t":"Elegibilidade · Unimed","s":"done"},{"i":"check","t":"Guia autorizada","s":"done"},{"i":"check","t":"Pedido médico anexado","s":"done"},'-',{"i":"print","t":"Imprimir etiqueta e pulseira","k":"Ctrl+P"},{"i":"doc","t":"Assinar termo no tablet"},{"i":"img","t":"Fotografar documento"},{"i":"search","t":"Consultar elegibilidade de novo"}],170,518,w=270,label="Submenu Check-in")}'''
S.append(scene(1,"Esteira da recepção: clique direito no paciente","O menu traz tudo que a recepção faz com aquele paciente, sem abrir outra tela. O submenu Check-in mostra as etapas já feitas e as que faltam.",690,st,[
"Cabeçalho do menu com <b>nome + nascimento</b> (dois identificadores), para não agir no paciente errado.","Campo de filtro: começar a digitar filtra os itens (\"eti\" → Imprimir etiqueta).","Atalho de teclado ao lado de cada ação; o mesmo atalho funciona sem abrir o menu.","Ações destrutivas sempre no fim, em vermelho, e pedem motivo.","Sem permissão: o item aparece apagado com o motivo (\"Só supervisor da recepção\"), para a pessoa saber a quem pedir."]))
# 2 Agenda event + remarcar submenu
grid='''<div class="mini-ag"><div class="hd"></div><div class="hd">Eco · Dr. Paulo</div><div class="hd">USG obstétrica</div><div class="hd">USG e Doppler</div>
<div class="h">08:30</div><div class="e now">Sônia M. Castro<small>Em exame</small></div><div class="e">Patrícia G. Lopes<small>Obstétrica</small></div><div></div>
<div class="h">09:00</div><div class="e sel">Cláudia B. Nunes<small>Eco + Doppler</small></div><div class="f">+ livre</div><div class="e">Marcelo D. Faria<small>USG abdome</small></div>
<div class="h">09:30</div><div class="f">+ livre</div><div class="e">Fernanda A. Prado<small>Translucência</small></div><div class="f">+ livre</div>
<div class="h">10:00</div><div class="e">Hélio T. Mendes<small>Eco</small></div><div></div><div></div></div>'''
st=grid+cursor(150,190)+menu([{"i":"open","t":"Abrir agendamento","k":"Enter"},{"i":"check","t":"Confirmar presença"},{"i":"chat","t":"Reenviar confirmação (WhatsApp)"},'-',{"i":"cal","t":"Remarcar","sub":1},{"i":"user","t":"Trocar médico","sub":1},{"i":"move","t":"Mover para outra sala","sub":1},{"i":"copy","t":"Recortar para mover","k":"Ctrl+X"},'-',{"i":"doc","t":"Preparo do exame","sub":1},{"i":"cash","t":"Orçamento (particular)"},{"i":"clock","t":"Marcar atraso do paciente"},'-',{"i":"block","t":"Marcar falta (não compareceu)","s":"danger"},{"i":"x","t":"Cancelar agendamento…","k":"Del","s":"danger"}],156,202,title="Cláudia B. Nunes · 09:00 · Eco + Doppler")+menu(["#Sugestões para Cláudia",{"i":"bolt","t":"Hoje 09:30 · mesma sala","k":"1"},{"i":"bolt","t":"Hoje 10:30 · mesma sala","k":"2"},{"i":"bolt","t":"Seg 09/11 08:00 · Dr. Paulo","k":"3"},'-',{"i":"cal","t":"Escolher outro dia…"},{"i":"chat","t":"Perguntar ao paciente pelo WhatsApp"}],222,494,w=290,label="Submenu Remarcar")
S.append(scene(2,"Agenda: clique direito num agendamento","Remarcar não abre formulário: o submenu já sugere os próximos horários livres compatíveis com o exame, o médico e o preparo.",790,st,[
"Sugestões calculadas no servidor (sala, médico, duração, preparo e convênio); números 1–3 escolhem direto.","\"Perguntar ao paciente\" manda as opções por WhatsApp e reserva por 15 min.","Recortar e colar (Ctrl+X / Ctrl+V) move o agendamento para outro horário; arrastar com o mouse faz o mesmo.","Cancelar pede motivo e oferece avisar o paciente; falta conta para o indicador de no-show."]))
# 3 free slot
st=grid+cursor(196,300)+menu([{"i":"cal","t":"Agendar aqui","k":"N"},{"i":"bolt","t":"Encaixar da lista de espera","sub":1},{"i":"copy","t":"Colar agendamento","k":"Ctrl+V","s":"off","why":"Nada recortado"},'-',{"i":"flag","t":"Reservar para urgência"},{"i":"block","t":"Bloquear horário…","sub":1},{"chk":True,"t":"Disponível no agendamento online"},'-',{"i":"info","t":"Por que este horário está livre?"}],202,312,title="USG obstétrica · 09:30 livre")+menu(["#Combina com este horário",{"i":"user","t":"Carla N. Souza · obstétrica","k":"1"},{"i":"user","t":"Aline B. Ramos · morfológica (60 min)","s":"off","why":"Precisa de 60 min; só há 30"},{"i":"user","t":"Mônica S. Reis · obstétrica","k":"2"},'-',{"i":"chat","t":"Oferecer a todos pelo WhatsApp"}],250,604,w=300,label="Submenu Encaixar")
S.append(scene(3,"Agenda: clique direito num horário livre","O horário vazio também tem menu: agendar, encaixar quem está na lista de espera, bloquear ou reservar.",600,st,[
"A lista de espera é filtrada pelo que cabe no horário (duração, sala, preparo); quem não cabe aparece apagado com o motivo.","\"Oferecer a todos\" manda a vaga; o primeiro que confirmar leva (garantido no banco, sem dupla reserva).","Bloquear pede motivo (manutenção do aparelho, ausência do médico) e avisa quem já estava agendado no período.","\"Disponível no agendamento online\" é um item de marcar (liga/desliga)."]))
# 4 Fluxo de laudos
rows=''.join(f'<div class="lr{" sel" if s else ""}"><b>{n}</b><span>{e}</span><span>{m}</span><span class="pill {c}">{t}</span></div>' for n,e,m,t,c,s in [
("Sônia M. Castro","Ecocardiograma","Dr. Paulo","Em laudo","pv",0),("Eduardo M. Vaz","Teste ergométrico","Dr. Paulo","Vence 14:00","pr",1),("Vera L. Antunes","TC abdome","Telerradiologia","Aguardando","pg",0),("Maria H. Duarte","Ecocardiograma","Dr. Paulo","Assinado","pgr",0)])
st=f'<div class="mini-list">{rows}</div>'+cursor(98,240)+menu([{"i":"doc","t":"Abrir no laudo","k":"Enter"},{"i":"img","t":"Abrir imagens","k":"I"},{"i":"layers","t":"Comparar com o exame anterior"},'-',{"i":"user","t":"Atribuir a","sub":1},{"i":"flag","t":"Prioridade","sub":1},{"i":"eye","t":"Pedir segunda leitura"},{"chk":False,"t":"Conversar com o paciente antes de liberar"},'-',{"i":"send","t":"Enviar ao médico solicitante","s":"off","why":"Laudo ainda não assinado"},{"i":"print","t":"Imprimir laudo e imagens"},{"i":"send","t":"Compartilhar link seguro com o paciente"}],104,252,title="Eduardo M. Vaz · ergométrico · vence 14:00")+menu(["#Médicos disponíveis",{"i":"user","t":"Dr. Paulo Medeiros · 7 na fila","chk":True},{"i":"user","t":"Dr. Marcos Freire · 3 na fila"},{"i":"user","t":"Telerradiologia · prazo 4 h"},'-',{"i":"bolt","t":"Distribuir automaticamente"}],198,546,w=280,label="Submenu Atribuir")+menu([{"i":"doc","t":"Abrir laudo assinado","k":"Enter"},{"i":"pen","t":"Retificar (cria versão 2)…"},{"i":"undo","t":"Desfazer assinatura","s":"off","why":"Laudo assinado não se altera: use Retificar"},{"i":"eye","t":"Ver quem acessou"}],480,600,w=300,title="Maria H. Duarte · laudo assinado",label="Menu de laudo assinado")
S.append(scene(4,"Fluxo de laudos: clique direito num exame","O menu muda conforme o estado do laudo. Em laudo pendente: atribuir, priorizar, pedir segunda leitura. Em laudo assinado: só abrir, retificar com nova versão e ver acessos.",720,st,[
"Itens dependem do <b>estado</b>: \"Enviar ao solicitante\" só depois de assinado.","Registro clínico não se apaga: \"Desfazer assinatura\" não existe na prática; o menu explica o caminho certo (Retificar).","\"Conversar antes de liberar\" (resultado delicado) é um item de marcar e bloqueia a publicação no portal.","Atribuir mostra a fila de cada médico para dividir a carga."]))
# 5 Visualizador (dark)
st='<div class="viewer"><img src="/_blob/61aa8c1eda64e280ef9de45e3cb63a07" alt="Tomografia de crânio (exemplo)"><span class="ov tl">VERA L. ANTUNES · 58 A<br>TC CRÂNIO · Série 2 · 14/32</span><span class="ov tr">WL 40 / WW 90</span></div>'+cursor(150,250)+menu(["#Imagem 14 de 32",{"i":"grid","t":"Janela","sub":1},{"i":"ruler","t":"Medir","sub":1},{"i":"pen","t":"Anotar seta ou texto","k":"A"},{"i":"key","t":"Marcar como imagem-chave","k":"K"},'-',{"i":"layers","t":"Sincronizar séries"},{"i":"layers","t":"Comparar com o exame anterior"},{"i":"undo","t":"Girar · inverter · espelhar","sub":1},'-',{"i":"copy","t":"Copiar imagem para o laudo"},{"i":"send","t":"Exportar anonimizada…"},{"i":"info","t":"Informações DICOM","k":"Ctrl+I"}],156,262,dark=True,w=280,label="Menu do visualizador")+menu(["#Predefinições",{"i":"check","t":"Cérebro (40/90)","s":"done"},{"i":"grid","t":"Partes moles (40/400)","k":"1"},{"i":"grid","t":"Osso (500/2000)","k":"2"},{"i":"grid","t":"Pulmão (−600/1500)","k":"3"},'-',{"i":"pen","t":"Salvar a atual como predefinição"}],190,548,dark=True,w=260,label="Submenu Janela")
S.append(scene(5,"Visualizador de imagens: clique direito na imagem","No visualizador o menu é escuro (não ofusca a leitura) e traz as ferramentas de medida, janela e marcação.",720,st,[
"Predefinições de janela por número (1, 2, 3) sem tirar o olho da imagem.","\"Imagem-chave\" (K) leva a imagem para o laudo e para o portal do paciente.","Exportar sai <b>anonimizado</b> por padrão; exportar com identificação exige permissão e motivo.","Copiar para o laudo insere a imagem com legenda e referência da série."]))
# 6 Caixa
rows=''.join(f'<div class="lr{" sel" if s else ""}"><span>{h}</span><b>{n}</b><span>{f}</span><span class="num">{v}</span></div>' for h,n,f,v,s in [("08:52","Rafael M. Lima · Raio X","PIX","R$ 180,00",1),("08:31","Carla N. Souza · Obstétrica","Cartão 2x","R$ 320,00",0),("08:10","Sangria para o cofre","Dinheiro","− R$ 400,00",0)])
st=f'<div class="mini-list">{rows}</div>'+cursor(48,260)+menu([{"i":"doc","t":"Ver recibo","k":"Enter"},{"i":"chat","t":"Reenviar comprovante (WhatsApp)"},{"i":"doc","t":"Ver NFS-e nº 2026/004812"},{"i":"copy","t":"Copiar código da transação PIX"},{"i":"search","t":"Ver na conciliação bancária"},{"i":"pen","t":"Adicionar observação"},'-',{"i":"undo","t":"Estornar…","s":"danger","why":"Pede 2º fator e aprovação do gerente"}],54,272,title="Rafael M. Lima · R$ 180,00 · PIX",foot="Estorno: quem pede não aprova")
S.append(scene(6,"Caixa: clique direito num recebimento","Tudo sobre um pagamento em um lugar: recibo, nota, comprovante, conciliação e estorno.",470,st,[
"Estorno tem duas travas: <b>reautenticação</b> (2º fator) e <b>aprovação de outra pessoa</b> (separação de funções).","O estorno é idempotente: clicar duas vezes não estorna duas vezes.","\"Ver na conciliação\" abre a linha do extrato correspondente."]))
# 7 Paciente anywhere
st='<div class="mini-list"><div class="lr sel"><b>Juliana Sampaio Rocha</b><span>12/04/1993 · Bradesco Saúde</span><span>Gestante 22 sem</span></div><div class="lr"><b>Juliana S. Rocha</b><span>12/04/1993 · sem convênio</span><span class="pill pr">Possível duplicado</span></div></div>'+cursor(48,250)+menu([{"i":"open","t":"Abrir ficha","k":"Enter"},{"i":"cal","t":"Novo agendamento","k":"N"},{"i":"copy","t":"Copiar nome e nascimento"},{"i":"clock","t":"Ver histórico de exames"},{"i":"send","t":"Enviar acesso ao portal"},{"i":"print","t":"Imprimir etiqueta"},'-',{"i":"merge","t":"Unir cadastros duplicados…","why":"Revisão por 2 pessoas"},{"i":"eye","t":"Ver quem acessou esta ficha","s":"off","why":"Só encarregado LGPD"}],54,262,title="Juliana Sampaio Rocha · 12/04/1993")
S.append(scene(7,"Qualquer nome de paciente: menu do paciente","Onde o nome do paciente aparecer (busca, esteira, agenda, fila de laudos), o clique direito abre o mesmo menu do paciente.",490,st,[
"Mesmo menu em todo o sistema: aprender uma vez serve para todas as telas.","Unir duplicados mostra os dois cadastros lado a lado e precisa da revisão de duas pessoas; nada é apagado.","\"Ver quem acessou\" só para o encarregado LGPD; o próprio paciente vê o resumo no portal."]))
# 8 Editor
st='<div class="mini-ed"><div class="lbl">Avaliação</div><p>Hipertensão arterial controlada. Disfunção diastólica <mark>leve</mark>, compatível com a idade e a hipertensão.</p><div class="lbl" style="margin-top:14px">Plano</div><p>Manter losartana. Doppler de carótidas em 12 meses.</p></div>'+cursor(70,330)+menu([{"i":"bolt","t":"Inserir frase","sub":1},{"i":"ruler","t":"Inserir medida do aparelho","sub":1},{"i":"flag","t":"Inserir CID-10…","k":"Ctrl+D"},{"i":"mic","t":"Ditar a partir daqui","k":"F9"},'-',"#Sugestões de escrita",{"i":"pen","t":"leve → discreta"},{"i":"pen","t":"leve → grau I"},'-',{"i":"copy","t":"Copiar"},{"i":"copy","t":"Colar sem formatação","k":"Ctrl+Shift+V"}],76,342,w=270,title="Texto da evolução")+menu(["#Medidas do eco de hoje",{"i":"ruler","t":"FE (Teichholz) 66%"},{"i":"ruler","t":"DDVE 48 mm"},{"i":"ruler","t":"AE 36 mm"},{"i":"ruler","t":"E/A 0,7"},'-',{"i":"layers","t":"Inserir tabela completa"}],136,618,w=240,label="Submenu medidas")
S.append(scene(8,"Editor da evolução e do laudo: clique direito no texto","No texto clínico o menu ajuda a escrever: frases prontas do médico, medidas que vieram do aparelho, CID e ditado.",560,st,[
"Medidas inseridas pelo menu vêm do aparelho (DICOM SR), sem redigitação.","Sugestões de escrita são só sugestões: nada muda sem o médico escolher.","Colar sem formatação evita trazer estilos estranhos de outros sistemas."]))
rules='''<section class="card" aria-labelledby="rg-h" style="padding:24px">
<h2 id="rg-h" style="font-size:19px;font-weight:650">Como o clique direito funciona em todo o VidaCor</h2>
<div class="grid g3" style="margin-top:16px">
<div><h3 class="rh">Abrir</h3><ul class="notes"><li>Clique direito, tecla <b>Menu</b> ou <b>Shift+F10</b> no item focado.</li><li>No tablet: toque longo.</li><li>Também há o botão <b>⋯</b> em cada item, para quem não usa clique direito.</li></ul></div>
<div><h3 class="rh">Navegar</h3><ul class="notes"><li>Setas sobem e descem; → abre submenu; ← volta; Esc fecha.</li><li>Começar a digitar filtra os itens.</li><li>Atalho ao lado de cada ação; funciona também com o menu fechado.</li></ul></div>
<div><h3 class="rh">Organizar</h3><ul class="notes"><li>Ordem: abrir → ações do dia a dia → comunicação → impressão → destrutivas no fim, em vermelho.</li><li>No máximo 12 itens por nível e 1 nível de submenu.</li><li>Cabeçalho com quem ou o quê (nome + nascimento no caso de paciente).</li></ul></div>
<div><h3 class="rh">Permissão</h3><ul class="notes"><li>Ação que o perfil <b>nunca</b> terá: não aparece.</li><li>Ação que existe mas está indisponível agora: aparece apagada <b>com o motivo</b>.</li><li>O servidor confere a permissão de novo ao executar (G1).</li></ul></div>
<div><h3 class="rh">Segurança</h3><ul class="notes"><li>Destrutivas pedem motivo e confirmação com o nome do paciente.</li><li>Estorno, assinatura, exportação e permissões pedem 2º fator.</li><li>Toda ação vai para a auditoria; repetir não duplica (G3).</li></ul></div>
<div><h3 class="rh">Seleção múltipla</h3><ul class="notes"><li>Ctrl/Shift + clique seleciona vários; o menu mostra \"3 pacientes selecionados\".</li><li>Só aparecem ações seguras em lote (enviar preparo, imprimir etiquetas, confirmar presença).</li><li>Nada clínico em lote (assinar, liberar laudo).</li></ul></div>
</div></section>'''
css='''
body{margin:0;background:#F2F3F6}
.stage{position:relative;background:#F6F8FB;border-radius:16px;overflow:hidden;min-width:0}
.notes{margin:0;padding-left:18px;display:flex;flex-direction:column;gap:10px;font-size:14px;line-height:1.5;color:#334155}
.rh{font-size:15px;font-weight:600;margin-bottom:8px}
.cm{position:absolute;background:#fff;border-radius:12px;box-shadow:0 12px 32px rgba(15,23,42,.16),0 2px 6px rgba(15,23,42,.08);border:1px solid #E3E7ED;padding:6px;z-index:5;font-size:14px}
.cm-t{font-size:12px;font-weight:600;color:#5B6474;padding:8px 10px 6px;border-bottom:1px solid #F1F3F6;margin-bottom:4px;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
.cm-s{display:flex;align-items:center;gap:8px;margin:2px 4px 6px;padding:0 10px;height:34px;border-radius:8px;background:#F6F8FB;color:#8A93A3;font-size:13px}
.cm-h{font-size:11px;font-weight:700;letter-spacing:.08em;text-transform:uppercase;color:#5B6474;padding:8px 10px 4px}
.cm-i{display:flex;align-items:center;gap:10px;min-height:36px;padding:6px 10px;border-radius:8px;color:#0F172A}
.cm-i:first-of-type.hov,.cm-i.hov{background:#EEF3FA}
.cm-l{flex:1;display:flex;flex-direction:column;min-width:0}
.cm-why{font-size:11.5px;color:#5B6474;margin-top:1px}
.cm-k{font-size:12px;color:#5B6474;white-space:nowrap}
.cm-i.off{color:#8A93A3}.cm-i.off svg{stroke:#B6BDC8}
.cm-i.danger{color:#B4233C}.cm-i.danger svg{stroke:#B4233C}
.cm-i.done{color:#1F7A4D}.cm-i.done svg{stroke:#1F7A4D}
.cm-sep{height:1px;background:#EEF0F3;margin:5px 4px}
.cm-f{display:flex;align-items:center;gap:8px;font-size:11.5px;color:#5B6474;padding:8px 10px 4px;border-top:1px solid #F1F3F6;margin-top:4px}
.cm.dk{background:#141B2B;border-color:#26304A;color:#E6ECF7}
.cm.dk .cm-i{color:#E6ECF7}.cm.dk .cm-h,.cm.dk .cm-k,.cm.dk .cm-t{color:#9AA8C2}.cm.dk .cm-sep{background:#26304A}.cm.dk .cm-i.done{color:#7CD3A6}.cm.dk .cm-i.done svg{stroke:#7CD3A6}
.cur{position:absolute;z-index:6}
.clk{position:absolute;width:12px;height:12px;border-radius:50%;background:rgba(12,114,176,.35);box-shadow:0 0 0 6px rgba(12,114,176,.15);z-index:6}
.mini-cols{display:grid;grid-template-columns:repeat(3,200px);gap:14px;padding:20px}
.colh{font-size:13px;font-weight:600;margin:0 2px 8px}
.kc{background:#fff;border:1px solid #E8EAEF;border-radius:12px;padding:10px 12px;margin-bottom:8px;display:flex;flex-direction:column;gap:2px}
.kc.sel{border-color:#0A1A4A;box-shadow:0 0 0 1px #0A1A4A}
.kc b{font-size:14px}.kc .x{font-size:12.5px;color:#5B6474}.kc .m{font-size:12px;color:#5B6474}
.mini-ag{display:grid;grid-template-columns:56px repeat(3,170px);grid-auto-rows:46px;gap:4px;padding:16px}
.mini-ag .hd{font-size:13px;font-weight:600;display:flex;align-items:center}
.mini-ag .h{font-size:12px;color:#5B6474;text-align:right;padding:4px 8px 0 0}
.mini-ag .e{background:#fff;border:1px solid #E3E7ED;border-radius:9px;padding:5px 9px;font-size:13px;font-weight:600;display:flex;flex-direction:column;justify-content:center}
.mini-ag .e small{font-size:11.5px;font-weight:400;color:#5B6474}
.mini-ag .e.now{background:#FAF8FE;border-color:#B9A9E6}.mini-ag .e.sel{border-color:#0A1A4A;box-shadow:0 0 0 1px #0A1A4A}
.mini-ag .f{border:1.5px dashed #BFD9EE;background:#F7FBFE;border-radius:9px;color:#0C72B0;font-size:12px;font-weight:600;display:flex;align-items:center;justify-content:center}
.mini-list{padding:16px;display:flex;flex-direction:column;gap:6px;width:640px}
.lr{display:grid;grid-template-columns:1.4fr 1.2fr 1fr auto;gap:12px;align-items:center;background:#fff;border:1px solid #E8EAEF;border-radius:10px;padding:10px 14px;font-size:13.5px}
.lr.sel{border-color:#0A1A4A;box-shadow:0 0 0 1px #0A1A4A}.lr span{color:#5B6474}
.pill{font-size:12px;font-weight:600;padding:3px 9px;border-radius:999px}
.pv{background:#EFEBFC;color:#5B45B5}.pr{background:#FDEEF1;color:#B4233C}.pg{background:#F1F3F6;color:#334155}.pgr{background:#E5F5EC;color:#1F7A4D}
.viewer{position:absolute;inset:0;background:#05070B;display:flex;align-items:center;justify-content:flex-start;padding-left:40px}
.viewer img{height:100%;max-height:440px}
.ov{position:absolute;font:600 12px/1.5 Inter,sans-serif;color:#C8D3E3}.ov.tl{left:16px;top:12px}.ov.tr{right:16px;top:12px}
.mini-ed{background:#fff;margin:16px;border-radius:14px;padding:20px 24px;width:560px;font-size:15px;line-height:1.6}
.mini-ed p{margin:6px 0 0}.mini-ed mark{background:#DCEBF7;border-radius:3px;padding:0 2px}
'''
page=f'''<!doctype html>
<html lang="pt-BR">
<head>
<meta charset="utf-8">
<title>Menus de contexto — VidaCor Imagem</title>
<script src="./support.js"></script>
<link rel="stylesheet" href="/_blob/b7e42ddd1a1313743941687fc5dcf532">
</head>
<body>
<x-dc>
<helmet>
<link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&amp;display=swap" rel="stylesheet">
<style>{css}</style>
</helmet>
<div style="font-family: Inter, sans-serif; padding: 32px; display: flex; flex-direction: column; gap: 20px; max-width: 1360px; margin: 0 auto">
  <div class="page-h" style="padding: 0">
    <div class="grow"><div class="lbl">Interações</div><h1 style="font-size: 32px; font-weight: 650; margin-top: 6px">Clique direito no VidaCor</h1>
    <p>Cada coisa da tela (paciente, horário, laudo, imagem, pagamento, texto) tem um menu com o que dá para fazer com ela ali mesmo. Abaixo, oito situações reais com os menus e submenus abertos.</p></div>
  </div>
{rules}
{"".join(S)}
  <p class="t12 mute">Mockup · dados fictícios · imagem de exemplo pública</p>
</div>
</x-dc>
<script type="text/x-dc" data-dc-script data-props='{{"$preview":{{"width":1440,"height":6700}}}}'>
class Component extends DCLogic {{
  renderVals() {{ return {{}}; }}
}}
</script>
</body>
</html>
'''
open(OUT,'w').write(page); print(len(page))
