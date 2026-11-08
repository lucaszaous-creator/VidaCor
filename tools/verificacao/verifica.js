// Verificador "a tela cabe na janela" (docs/05, DEC-2026-11-08-40). Sai com código 1 se alguma tela falhar.
// Uso: node verifica.js [Tela.dc.html ...]   (sem argumentos: todas as telas de design/mockups)
// VC_MOCKUPS=<pasta> troca a pasta das telas (usado para provar que o verificador reprova).
//
// Critério:
// - Tela de sistema ($preview 1440x900 no próprio .dc.html): em 1440x900 e 1280x800 a página tem a altura
//   e a largura exatas da janela.
// - Tela com tamanho próprio (apps, tablet, telão, A4, componentes; outro $preview): só no próprio tamanho,
//   sem passar da altura nem da largura.
// - Em qualquer caso: nenhum painel cortando conteúdo ("cortado"), nenhuma rolagem lateral ("ladoalado")
//   e nenhum erro de script. Exceções só em excecoes.tsv, com motivo e decisão.
// - Recurso ausente (404 das imagens /_blob, que não estão no repositório) não reprova: é limite conhecido.
const fs=require('fs');const path=require('path');
const {renderiza,linha,motor}=require('./renderiza');
const D=__dirname, R=path.resolve(D,'../..'), MOCK=process.env.VC_MOCKUPS?path.resolve(process.env.VC_MOCKUPS):path.join(R,'design/mockups'), CSS=path.join(R,'design/vc10.css');
process.env.CSSID=process.env.CSSID||'3428af26abb0af564c467191e42d3475';

function tamanhoDe(f){
  const t=fs.readFileSync(path.join(MOCK,f),'utf8');
  const m=/data-props='([^']*)'/.exec(t);
  try{const p=JSON.parse(m[1]).$preview;if(p&&p.width&&p.height) return {w:p.width,h:p.height};}catch(e){/* sem $preview */}
  return {w:1440,h:900};
}
function excecoes(){
  const a=path.join(D,'excecoes.tsv');if(!fs.existsSync(a)) return [];
  return fs.readFileSync(a,'utf8').split('\n').filter(l=>l.trim()&&!l.startsWith('#')).map(l=>{
    const [arquivo,tipo,inicio,motivo,decisao]=l.split('\t');
    if(!arquivo||!tipo||!inicio||!motivo||!decisao){console.error('excecoes.tsv: linha sem as 5 colunas: '+l);process.exit(2);}
    return {arquivo,tipo,inicio,motivo,decisao,usada:false};});
}

(async()=>{
  let files=process.argv.slice(2).map(f=>path.basename(f));
  if(!files.length) files=fs.readdirSync(MOCK).filter(f=>f.endsWith('.dc.html')).sort();
  const faltando=files.filter(f=>!fs.existsSync(path.join(MOCK,f)));
  if(faltando.length){console.error('não encontrado em '+path.relative(R,MOCK)+': '+faltando.join(', '));process.exit(2);}
  const exc=excecoes();
  console.error('motor: '+motor);

  // Grupos de renderização: telas de sistema em 1440x900 e 1280x800; as demais no próprio tamanho.
  const grupos=new Map();const poe=(W,H,pasta,f,exata)=>{const k=W+'x'+H+'|'+pasta+'|'+exata;
    if(!grupos.has(k)) grupos.set(k,{W,H,pasta,exata,files:[]});grupos.get(k).files.push(f);};
  for(const f of files){const t=tamanhoDe(f);
    if(t.w===1440&&t.h===900){poe(1440,900,'1440',f,true);poe(1280,800,'1280',f,true);}
    else poe(t.w,t.h,'proprio',f,false);}

  const falhas=[];
  for(const g of grupos.values()){
    console.log(`# ${g.W}x${g.H} (${g.exata?'tela de sistema':'tamanho próprio'})`);
    await renderiza({dir:MOCK,css:CSS,W:g.W,H:g.H,out:path.join(D,'shots',g.pasta),files:g.files,aoMedir:r=>{
      console.log(linha(r));
      const prob=[];const m=r.m;
      if(g.exata?m.sh!==g.H:m.sh>g.H) prob.push(`altura da página ${m.sh} (janela ${g.H})`);
      if(g.exata?m.sw!==g.W:m.sw>g.W) prob.push(`largura da página ${m.sw} (janela ${g.W})`);
      for(const tipo of ['cortado','ladoalado']) for(const item of m[tipo]){
        const e=exc.find(x=>x.arquivo===r.f&&x.tipo===tipo&&(item===x.inicio||item.startsWith(x.inicio+' ')));
        if(e) e.usada=true; else prob.push(`${tipo}: ${item}`);}
      for(const e of r.errs) if(!/status of 404/.test(e)) prob.push('erro: '+e);
      if(prob.length) falhas.push(`${r.f} em ${g.W}x${g.H}: ${prob.join('; ')}`);
    }});
  }
  const sobras=exc.filter(e=>!e.usada&&files.includes(e.arquivo));
  for(const e of sobras) console.log(`aviso: exceção sem uso em excecoes.tsv (${e.arquivo}, ${e.tipo}, ${e.inicio}); revisar e registrar a retirada`);
  if(falhas.length){console.log(`\nREPROVADO: ${falhas.length} problema(s)`);falhas.forEach(x=>console.log('- '+x));process.exit(1);}
  console.log(`\nAPROVADO: ${files.length} tela(s)`);
})().catch(e=>{console.error(e);process.exit(2);});
