import random
P='/tmp/claude-0/-home-user/4fd689ae-5e0d-5bbd-bb63-c60bbbb26a18/scratchpad/vidacor-canvas/project/'
CSS='/_blob/b7e42ddd1a1313743941687fc5dcf532'
LOGO='/_blob/69e27b6caf70295083c2b311e1218937'
def barcode(w,h,seed):
    r=random.Random(seed); x=0; d=[]
    while x<w-2:
        bw=r.choice([1,1,2,3]); 
        if r.random()<0.55: d.append(f'M{x} 0h{bw}v{h}h-{bw}z')
        x+=bw+r.choice([1,1,2])
    return f'<svg width="{w}" height="{h}" viewBox="0 0 {w} {h}" role="img" aria-label="Código de barras"><path fill="#111" d="{"".join(d)}"></path></svg>'
def qr(size,seed):
    r=random.Random(seed); n=25; c=size/n; out=[]
    def fin(x,y):
        out.append(f'M{x*c} {y*c}h{7*c}v{7*c}h-{7*c}z M{(x+1)*c} {(y+1)*c}v{5*c}h{5*c}v-{5*c}z M{(x+2)*c} {(y+2)*c}h{3*c}v{3*c}h-{3*c}z')
    for (x,y) in [(0,0),(18,0),(0,18)]: fin(x,y)
    for i in range(n):
        for j in range(n):
            if (i<8 and j<8) or (i>16 and j<8) or (i<8 and j>16): continue
            if r.random()<0.45: out.append(f'M{i*c:.2f} {j*c:.2f}h{c:.2f}v{c:.2f}h-{c:.2f}z')
    return f'<svg width="{size}" height="{size}" viewBox="0 0 {size} {size}" role="img" aria-label="QR code da etiqueta"><path fill="#111" fill-rule="evenodd" d="{" ".join(out)}"></path></svg>'
head=lambda t:f'''<!doctype html>
<html lang="pt-BR">
<head>
<meta charset="utf-8">
<title>{t}</title>
<script src="./support.js"></script>
<link rel="stylesheet" href="{CSS}">
</head>
<body>
<x-dc>
<helmet>
<link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700;800&amp;display=swap" rel="stylesheet">
<style>STYLE</style>
</helmet>
'''
foot=lambda h:f'''</x-dc>
<script type="text/x-dc" data-dc-script data-props='{{"$preview":{{"width":1440,"height":{h}}}}}'>
class Component extends DCLogic {{
  renderVals() {{ return {{}}; }}
}}
</script>
</body>
</html>
'''
# ===================== Painel de etiquetas e rastreio =====================
rows=[
("ETQ-2026-018734","Pulseira","Antônio Carlos Ribeiro","15/06/1954","Eco + Doppler · ACC 2026-118790","Guichê 2 · Ana · 08:24","Bipada na Sala 1 · 08:41","ok","Em uso",1),
("ETQ-2026-018733","Exame","Antônio Carlos Ribeiro","15/06/1954","Eco + Doppler · ACC 2026-118790","Guichê 2 · Ana · 08:24","Colada no pedido","ok","Em uso",0),
("ETQ-2026-018731","Pulseira","Juliana Sampaio Rocha","12/04/1993","Morfológica · ACC 2026-118741","Guichê 1 · Carla · 08:19","Bipada na Sala 1 · 08:44","bad","Divergência",0),
("ETQ-2026-018729","Seringa","Vera L. Antunes","20/02/1968","Contraste iodado · lote IOP-2611","Sala 6 · Téc. Carla · 07:58","Bipada no tomógrafo · 08:02","ok","Usada",0),
("ETQ-2026-018722","Pulseira","Rafael Monteiro Lima","09/11/1988","Raio X de tórax · ACC 2026-118722","Guichê 2 · Ana · 08:10","Reimpressa · motivo: rasgou","warn","Invalidada",0),
("ETQ-2026-018723","Pulseira","Rafael Monteiro Lima","09/11/1988","Raio X de tórax · ACC 2026-118722","Guichê 2 · Ana · 08:12","Aguardando bipe na Sala 5","info","Impressa",0),
("ETQ-2026-018699","MAPA","Neusa A. Freitas","18/09/1957","MAPA 24 h · aparelho nº 07","Sala 7 · Enf. Lívia · 07:55","Devolução prevista 07/11 08:00","info","Em uso",0),
]
chip={"ok":"c-green","bad":"c-rose","warn":"c-amber","info":"c-blue"}
trs=''.join(f'''<tr class="{"sel" if s else ""}"><td class="nw"><b class="num">{i}</b></td><td>{t}</td><td class="nw"><b>{n}</b><div class="t12 mute">{d}</div></td><td>{e}</td><td class="t13 mute">{o}</td><td class="t13">{ev}</td><td><span class="chip {chip[k]}">{st}</span></td></tr>''' for i,t,n,d,e,o,ev,k,st,s in rows)
tl=[("ok","Impressa","Guichê 2 · impressora Zebra G2 · Ana Souza","08:24"),("ok","Colocada no paciente","Recepção conferiu nome + nascimento","08:25"),("ok","Bipada na Sala 1","Téc. Carla · paciente e exame conferem","08:41"),("ok","Vinculada ao exame","Imagens recebidas com o ACC 2026-118790","08:47"),("todo","Envelope (se impresso)","Ao liberar o laudo",""),("todo","Retirada pelo paciente","Assinatura no tablet","")]
tlh=''.join(f'<li class="tli"><span class="ck {k}">{"✓" if k=="ok" else ""}</span><div style="flex:1;min-width:0"><div class="t14 {"b" if k=="ok" else "mute"}">{a}</div><div class="t12 mute">{b}</div></div><span class="t12 mute num">{c}</span></li>' for k,a,b,c in tl)
style1='''body{margin:0;background:#F2F3F6}
.tb tr.sel td{background:#F3F6FB}
.tli{display:flex;gap:12px;align-items:flex-start;padding:10px 0;position:relative}
.ck{width:24px;height:24px;border-radius:50%;display:flex;align-items:center;justify-content:center;flex:none;font-size:12px;font-weight:700}
.ck.ok{background:#E5F5EC;color:#1F7A4D}.ck.todo{background:#fff;border:1.5px solid #CBD2DC}
.lbl-prev{border:1px solid #CBD2DC;border-radius:10px;background:#fff;padding:12px 14px;display:flex;gap:12px;align-items:center}
'''
p1=head('Etiquetas e rastreio — VidaCor').replace('STYLE',style1)+f'''<div class="app" style="font-family: Inter, sans-serif">
  <div class="side"><dc-import name="Sidebar" active="etiquetas" hint-size="248px,1400px"></dc-import></div>
  <main class="content">
    <div class="topbar">
      <h1 style="font-size: 24px; font-weight: 650">Etiquetas e rastreio</h1>
      <div class="grow"></div>
      <label class="search" style="max-width: 380px; flex: 1 1 300px; min-height: 44px"><svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" aria-hidden="true"><path d="M3 5v14M6 5v14M9 5v14M13 5v14M16 5v14M20 5v14"></path></svg><span class="sr">Bipar etiqueta</span><input type="search" placeholder="Bipe a etiqueta ou digite o código"></label>
      <a class="btn btn-sm" href="Etiquetas-Modelos.dc.html">Modelos</a>
      <button class="btn btn-p">Imprimir etiqueta</button>
    </div>

    <div class="row wrap t14 mute" style="gap: 24px; padding: 0 8px">
      <span><b style="color:#0F172A">86</b> impressas hoje</span>
      <span><b style="color:#0F172A">41</b> bipadas nas salas</span>
      <span><b style="color:#9A5B00">3</b> reimpressões com motivo</span>
      <span><b style="color:#C2334D">1</b> divergência aberta</span>
      <span><b style="color:#0F172A">3 de 4</b> impressoras online</span>
    </div>

    <section class="card" aria-labelledby="div-h" style="padding: 18px 22px; border: 1px solid #F6C9D2; box-shadow: none; background: #FFF7F8">
      <div class="row wrap" style="gap: 16px">
        <span class="tile t-rose"><svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" aria-hidden="true"><path d="M12 9v4M12 17h.01M10.3 3.9L1.8 18a2 2 0 0 0 1.7 3h17a2 2 0 0 0 1.7-3L13.7 3.9a2 2 0 0 0-3.4 0z"></path></svg></span>
        <div style="flex: 1 1 420px; min-width: 0">
          <h2 id="div-h" style="font-size: 16px; font-weight: 600">Etiqueta bipada na sala errada · exame bloqueado</h2>
          <p class="t14" style="margin: 4px 0 0">A pulseira de <b>Juliana Sampaio Rocha</b> (12/04/1993, morfológica, Sala 2) foi bipada na <b>Sala 1 · Ecocardiograma</b> às 08:44. O aparelho não recebeu a lista e nenhuma imagem foi aceita.</p>
        </div>
        <div class="row" style="gap: 8px"><button class="btn btn-sm">Ver detalhes</button><button class="btn btn-p btn-sm">Encaminhar à Sala 2</button></div>
      </div>
    </section>

    <div class="col" style="gap: 16px">
      <section class="card" aria-labelledby="ras-h" style="padding: 20px 8px 8px">
        <div class="row" style="padding: 0 14px 10px"><h2 id="ras-h" style="font-size: 18px; font-weight: 600">Etiquetas de hoje</h2><div class="grow"></div><div class="seg"><button class="on btn-sm" style="height: 36px">Todas</button><button style="height: 36px">Pulseiras</button><button style="height: 36px">Exame</button><button style="height: 36px">Seringa</button><button style="height: 36px">Envelope</button></div></div>
        <div class="scroll-x"><table class="tb"><thead><tr><th>Código</th><th>Tipo</th><th>Paciente</th><th>Para</th><th>Impressa</th><th>Último evento</th><th>Situação</th></tr></thead><tbody>{trs}</tbody></table></div>
      </section>

      <div class="grid g2">
        <section class="card" aria-labelledby="tl-h">
          <div class="lbl">Lastro da etiqueta</div>
          <h2 id="tl-h" style="font-size: 18px; font-weight: 650; margin-top: 6px" class="num">ETQ-2026-018734</h2>
          <div class="lbl-prev" style="margin-top: 12px">
            {qr(64,7)}
            <div style="min-width:0"><div class="t14 b">ANTÔNIO CARLOS RIBEIRO</div><div class="t13">Nasc. 15/06/1954 · P</div><div class="t12 mute num">ACC 2026-118790 · ETQ-018734</div></div>
          </div>
          <ol style="list-style: none; margin: 12px 0 0; padding: 0">{tlh}</ol>
          <div class="row wrap" style="gap: 8px; margin-top: 10px"><button class="btn btn-sm">Reimprimir…</button><button class="btn btn-sm">Invalidar…</button><a class="btn btn-g btn-sm" href="Paciente.dc.html">Abrir ficha</a></div>
          <p class="t12 mute" style="margin: 10px 0 0">Reimprimir pede motivo e invalida esta etiqueta: se a antiga for bipada depois, o sistema bloqueia e avisa.</p>
        </section>
        <section class="card" aria-labelledby="imp-h">
          <h2 id="imp-h" style="font-size: 16px; font-weight: 600; margin-bottom: 6px">Impressoras</h2>
          <div class="li"><span class="chip c-green"><span class="d"></span>Online</span><div style="flex:1"><div class="t t14">Guichê 1 · Zebra ZD421</div><div class="s">pulseiras e exame · ribbon 64%</div></div></div>
          <div class="li"><span class="chip c-green"><span class="d"></span>Online</span><div style="flex:1"><div class="t t14">Guichê 2 · Zebra ZD421</div><div class="s">pulseiras e exame · ribbon 81%</div></div></div>
          <div class="li"><span class="chip c-green"><span class="d"></span>Online</span><div style="flex:1"><div class="t t14">Sala 6 · Brother QL</div><div class="s">seringas</div></div></div>
          <div class="li"><span class="chip c-rose"><span class="d"></span>Offline</span><div style="flex:1"><div class="t t14">Recepção · impressora A4</div><div class="s">laudos e envelopes · 2 na fila</div></div></div>
        </section>
      </div>
    </div>
    <p class="t12 mute" style="margin: 0 8px">Mockup · dados fictícios · códigos ilustrativos</p>
  </main>
</div>
'''+foot(1500)
open(P+'Etiquetas.dc.html','w').write(p1)
# ===================== Folha de modelos =====================
MM=3.2 # px por mm
def box(wmm,hmm,inner,extra=''):
    return f'<div class="lab" style="width:{wmm*MM:.0f}px;height:{hmm*MM:.0f}px;{extra}">{inner}</div>'
def cap(t,s): return f'<div class="cap"><b>{t}</b><span>{s}</span></div>'
pulseira=box(250,25,f'''<div style="display:flex;align-items:center;height:100%;padding:0 10px 0 60px;gap:14px;background:repeating-linear-gradient(90deg,#fff 0 520px,#fff 520px)">
  <div style="width:14px;height:14px;border-radius:50%;border:2px solid #CBD2DC;position:absolute;left:18px"></div>
  {qr(62,11)}
  <div style="flex:1;min-width:0;line-height:1.15"><div style="font-size:17px;font-weight:800;letter-spacing:.01em">ANTÔNIO CARLOS RIBEIRO</div><div style="font-size:13px;font-weight:600">Nasc. 15/06/1954 · M · 72 a</div><div style="font-size:11px" class="num">ACC 2026-118790 · ETQ-018734</div></div>
  <div style="background:#111;color:#fff;font-weight:800;font-size:13px;border-radius:6px;padding:4px 8px">P</div>
  <div style="border:2px solid #111;font-weight:800;font-size:12px;border-radius:6px;padding:3px 8px">ALERGIA</div>
  {barcode(150,46,3)}
  <div style="width:120px"></div>
  <div style="width:14px;height:14px;border-radius:50%;border:2px solid #CBD2DC"></div>
</div>''','position:relative;border-radius:14px')
exame=box(60,30,f'''<div style="padding:8px 10px;display:flex;flex-direction:column;gap:2px;height:100%;line-height:1.15">
  <div style="font-size:13px;font-weight:800">ANTÔNIO CARLOS RIBEIRO</div>
  <div style="font-size:11px;font-weight:600">15/06/1954 · Unimed</div>
  <div style="font-size:11px">Eco + Doppler carótidas · 06/11 08:30</div>
  <div style="margin-top:auto;display:flex;align-items:flex-end;gap:6px">{barcode(120,26,5)}<span style="font-size:9.5px" class="num">ACC 2026-<br>118790</span></div>
</div>''')
seringa=box(45,20,f'''<div style="padding:6px 8px;line-height:1.15;height:100%;display:flex;gap:6px">
  <div style="flex:1;min-width:0"><div style="font-size:10.5px;font-weight:800">VERA L. ANTUNES</div><div style="font-size:9.5px">20/02/1968</div><div style="font-size:9.5px;font-weight:700">Iopamidol 300 · 80 mL</div><div style="font-size:9px" class="num">Lote IOP-2611 · val 03/27 · 07:58</div></div>
  {qr(40,13)}
</div>''')
envelope=box(110,60,f'''<div style="padding:12px 14px;height:100%;display:flex;flex-direction:column;gap:6px">
  <div style="display:flex;align-items:center;justify-content:space-between"><img src="{LOGO}" alt="VidaCor Imagem" style="height:38px"><span style="font-size:10px" class="num">ACC 2026-118790</span></div>
  <div style="display:flex;gap:12px;align-items:center;margin-top:2px">
    <div style="flex:1;min-width:0;line-height:1.25"><div style="font-size:15px;font-weight:800">Antônio Carlos Ribeiro</div><div style="font-size:12px">Nasc. 15/06/1954</div><div style="font-size:12px">Ecocardiograma + Doppler de carótidas</div><div style="font-size:12px">06/11/2026 · Dr. Paulo Medeiros</div></div>
    <div style="text-align:center">{qr(70,17)}<div style="font-size:8.5px;margin-top:2px">Resultado no portal</div></div>
  </div>
</div>''')
mapa=box(60,30,f'''<div style="padding:8px 10px;line-height:1.2;height:100%;display:flex;flex-direction:column;gap:2px">
  <div style="font-size:12px;font-weight:800">NEUSA A. FREITAS</div><div style="font-size:10.5px">18/09/1957</div>
  <div style="font-size:11px;font-weight:700">MAPA 24 h · aparelho nº 07</div>
  <div style="font-size:11px;margin-top:auto;border-top:1px dashed #999;padding-top:3px"><b>Devolver:</b> sáb 07/11 às 08:00</div>
</div>''')
cd=f'''<div class="lab" style="width:{118*MM:.0f}px;height:{118*MM:.0f}px;border-radius:50%;display:flex;align-items:center;justify-content:center;position:relative">
  <div style="position:absolute;width:{36*MM:.0f}px;height:{36*MM:.0f}px;border-radius:50%;border:1px dashed #9AA6B8;background:#F2F3F6"></div>
  <div style="position:absolute;top:44px;text-align:center"><img src="{LOGO}" alt="VidaCor Imagem" style="height:58px"></div>
  <div style="position:absolute;bottom:52px;text-align:center;line-height:1.3;width:70%"><div style="font-size:14px;font-weight:800">Maria Helena Duarte</div><div style="font-size:11px">Nasc. 12/03/1968</div><div style="font-size:11px">Ecocardiograma · 06/11/2026</div><div style="font-size:9.5px" class="num">ACC 2026-118701 · ETQ-018705</div></div>
  <div style="position:absolute;right:36px;top:{59*MM-30:.0f}px">{qr(60,19)}</div>
</div>'''
style2='''body{margin:0;background:#E9ECF1}
.lab{background:#fff;border:1px solid #BFC7D2;border-radius:8px;box-shadow:0 2px 6px rgba(15,23,42,.08);overflow:hidden;color:#111;font-family:Inter,Arial,sans-serif}
.cap{display:flex;flex-direction:column;gap:2px;margin-bottom:10px}.cap b{font-size:15px;font-weight:650;color:#0F172A}.cap span{font-size:13px;color:#5B6474}
.item{display:flex;flex-direction:column}
'''
p2=head('Modelos de etiqueta — VidaCor').replace('STYLE',style2)+f'''<div style="font-family: Inter, sans-serif; padding: 40px; display: flex; flex-direction: column; gap: 34px">
  <div><div class="lbl">Tamanho real aproximado (1 mm ≈ 3,2 px)</div><h1 style="font-size: 30px; font-weight: 650; margin-top: 6px">Modelos de etiqueta</h1><p class="t15 mute" style="margin: 6px 0 0; max-width: 900px">Toda etiqueta tem um código único (ETQ) e o número do exame (ACC). Mostra só o mínimo para identificar: nome completo e nascimento. Nunca diagnóstico ou resultado.</p></div>
  <div class="item">{cap("1 · Pulseira do paciente (250 × 25 mm)","Impressa no check-in. Nome, nascimento, P de preferencial, aviso de alergia sem dizer qual, QR e código de barras. Bipada na sala antes do exame.")}{pulseira}</div>
  <div style="display:flex;gap:40px;flex-wrap:wrap;align-items:flex-start">
    <div class="item">{cap("2 · Exame / pedido (60 × 30 mm)","Colada no pedido médico e na guia.")}{exame}</div>
    <div class="item">{cap("3 · Seringa de contraste (45 × 20 mm)","Impressa na sala; liga o lote ao paciente.")}{seringa}</div>
    <div class="item">{cap("4 · Aparelho MAPA/Holter (60 × 30 mm)","Colada no aparelho; data de devolução.")}{mapa}</div>
  </div>
  <div style="display:flex;gap:40px;flex-wrap:wrap;align-items:flex-start">
    <div class="item">{cap("5 · Envelope de exame (110 × 60 mm)","Com QR para o resultado no portal (o QR só abre após login do paciente).")}{envelope}</div>
  </div>
  <p class="t12 mute">Mockup · dados fictícios · códigos e QR ilustrativos · impressoras térmicas (ZPL) e gravador de mídia</p>
</div>
'''+foot(1500)
open(P+'Etiquetas-Modelos.dc.html','w').write(p2)
print('ok')
