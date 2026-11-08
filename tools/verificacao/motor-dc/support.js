/*
 * Motor DC aberto da VidaCor (DEC-2026-11-08-39).
 *
 * Renderiza os mockups `design/mockups/*.dc.html` para o verificador "a tela cabe na janela"
 * (tools/verificacao/verifica.sh, docs/05). Substitui o motor da ferramenta de design, que nunca
 * foi versionado e cuja licença é desconhecida. Cobre só o subconjunto do formato usado nos mockups:
 *   <x-dc> (modelo), <helmet> (vai para o <head>), <script type="text/x-dc"> com
 *   `class Component extends DCLogic` (state, setState, props, renderVals, componentDidMount,
 *   componentWillUnmount), <sc-if value>, <sc-for list as>, {{caminho}} em texto e atributos,
 *   eventos onClick/onChange/onKeyDown/onContextMenu/onMouseEnter e <dc-import name=... props>.
 * Fidelidade ao motor original: não verificada (o original não está disponível para comparação).
 *
 * Por que o código-fonte bruto: o parser HTML do navegador tira <sc-for> de dentro de <tbody>,
 * <tr> e <select> ("foster parenting"). Por isso o motor captura o texto original da página com
 * <plaintext> e monta a árvore com um parser próprio, sem passar pelo parser do navegador.
 * Sem dependências, sem rede além dos próprios arquivos .dc.html importados.
 */
(function () {
  'use strict';
  if (window.__motorDC) return;
  window.__motorDC = true;

  // Tudo o que vier depois deste <script> no arquivo vira texto bruto de um <plaintext> invisível.
  document.write('<plaintext id="__dc_fonte" style="display:none">');

  var SVG = 'http://www.w3.org/2000/svg';
  var VAZIOS = new Set('area base br col embed hr img input link meta param source track wbr'.split(' '));
  var BRUTOS = new Set(['script', 'style', 'textarea', 'title']);
  var BOOLEANOS = new Set(['disabled', 'checked', 'selected', 'readonly', 'hidden', 'open', 'required', 'multiple', 'autofocus']);
  var FECHA_IRMAO = { li: ['li'], option: ['option'], tr: ['tr'], td: ['td', 'th'], th: ['td', 'th'], p: ['p'] };

  // ---------- Parser do código-fonte ----------
  var decodificador = document.createElement('textarea');
  function decodifica(s) {
    if (s.indexOf('&') < 0) return s;
    decodificador.innerHTML = s;
    return decodificador.value;
  }

  function analisa(fonte) {
    var raiz = { tag: '#raiz', nome: '#raiz', attrs: [], filhos: [] };
    var pilha = [raiz];
    var i = 0, n = fonte.length;
    var reNome = /[a-zA-Z][\w:.-]*/y, reAttr = /[^\s"'>\/=]+/y, reEsp = /\s*/y;
    function topo() { return pilha[pilha.length - 1]; }
    function fecha(tag) {
      for (var k = pilha.length - 1; k > 0; k--) {
        if (pilha[k].tag === tag) { pilha.length = k; return; }
      }
    }
    while (i < n) {
      if (fonte.startsWith('<!--', i)) {
        var fim = fonte.indexOf('-->', i + 4);
        i = fim < 0 ? n : fim + 3;
        continue;
      }
      if (fonte[i] === '<' && fonte[i + 1] === '!') {
        var f2 = fonte.indexOf('>', i);
        i = f2 < 0 ? n : f2 + 1;
        continue;
      }
      if (fonte[i] === '<' && fonte[i + 1] === '/') {
        var m = /^<\/([a-zA-Z][\w:.-]*)\s*>/.exec(fonte.slice(i, i + 80));
        if (m) { fecha(m[1].toLowerCase()); i += m[0].length; continue; }
      }
      if (fonte[i] === '<' && /[a-zA-Z]/.test(fonte[i + 1] || '')) {
        reNome.lastIndex = i + 1;
        var nome = reNome.exec(fonte)[0];
        var no = { tag: nome.toLowerCase(), nome: nome, attrs: [], filhos: [] };
        var j = reNome.lastIndex, autoFecha = false;
        while (j < n) {
          reEsp.lastIndex = j; reEsp.exec(fonte); j = reEsp.lastIndex;
          if (fonte[j] === '>') { j++; break; }
          if (fonte[j] === '/' && fonte[j + 1] === '>') { autoFecha = true; j += 2; break; }
          if (fonte[j] === '/') { j++; continue; }
          reAttr.lastIndex = j;
          var ma = reAttr.exec(fonte);
          if (!ma) { j++; continue; }
          var an = ma[0], av = '';
          j = reAttr.lastIndex;
          var mi = /\s*=\s*/y; mi.lastIndex = j;
          if (mi.exec(fonte)) {
            j = mi.lastIndex;
            var q = fonte[j];
            if (q === '"' || q === "'") {
              var fq = fonte.indexOf(q, j + 1);
              if (fq < 0) fq = n;
              av = fonte.slice(j + 1, fq); j = fq + 1;
            } else {
              var mu = /[^\s>]*/y; mu.lastIndex = j; av = mu.exec(fonte)[0]; j = mu.lastIndex;
            }
          }
          no.attrs.push([an, av]);
        }
        i = j;
        var irmaos = FECHA_IRMAO[no.tag];
        if (irmaos && irmaos.indexOf(topo().tag) >= 0) pilha.pop();
        topo().filhos.push(no);
        if (autoFecha || VAZIOS.has(no.tag)) continue;
        if (BRUTOS.has(no.tag)) {
          var reFim = new RegExp('</' + no.tag + '\\s*>', 'ig');
          reFim.lastIndex = i;
          var mf = reFim.exec(fonte);
          var fimBruto = mf ? mf.index : n;
          no.filhos.push({ texto: fonte.slice(i, fimBruto), bruto: no.tag === 'script' || no.tag === 'style' });
          i = mf ? reFim.lastIndex : n;
          continue;
        }
        pilha.push(no);
        continue;
      }
      var prox = fonte.indexOf('<', i + 1);
      if (prox < 0) prox = n;
      topo().filhos.push({ texto: fonte.slice(i, prox) });
      i = prox;
    }
    return raiz;
  }

  function attr(no, nome) {
    for (var k = 0; k < no.attrs.length; k++) if (no.attrs[k][0].toLowerCase() === nome) return no.attrs[k][1];
    return null;
  }
  function procura(no, teste, saida) {
    if (!no.filhos) return saida;
    no.filhos.forEach(function (f) { if (f.tag && teste(f)) saida.push(f); else procura(f, teste, saida); });
    return saida;
  }

  // ---------- Definições de componente (página e importados) ----------
  var definicoes = {};
  function DCLogic(props) { this.props = props || {}; this.state = {}; }
  DCLogic.prototype.setState = function (u, cb) {
    var parcial = typeof u === 'function' ? u(this.state, this.props) : u;
    if (parcial) this.state = Object.assign({}, this.state, parcial);
    agenda();
    if (typeof cb === 'function') Promise.resolve().then(cb.bind(this));
  };
  DCLogic.prototype.forceUpdate = function () { agenda(); };
  window.DCLogic = DCLogic;

  function define(nome, arvore) {
    var modelo = procura(arvore, function (f) { return f.tag === 'x-dc'; }, [])[0];
    var script = procura(arvore, function (f) { return f.tag === 'script' && attr(f, 'type') === 'text/x-dc'; }, [])[0];
    var cabeca = procura(arvore, function (f) { return f.tag === 'helmet' || ((f.tag === 'link' || f.tag === 'style') && !dentroDe(arvore, f, 'x-dc')); }, []);
    var Comp = null, props = {};
    if (script) {
      try {
        Comp = new Function('DCLogic', script.filhos.map(function (t) { return t.texto; }).join('') +
          '\n;return typeof Component !== "undefined" ? Component : null;')(DCLogic);
      } catch (e) { console.error('motor-dc: erro no script de ' + nome + ': ' + e.message); }
      try { props = JSON.parse(attr(script, 'data-props') || '{}'); } catch (e) { props = {}; }
    }
    cabeca.forEach(function (c) { (c.tag === 'helmet' ? c.filhos : [c]).forEach(poeNaCabeca); });
    definicoes[nome] = { modelo: modelo ? modelo.filhos : [], Comp: Comp, props: props };
    var imports = procura({ filhos: modelo ? modelo.filhos : [] }, function (f) { return f.tag === 'dc-import'; }, []);
    return Promise.all(imports.map(function (im) { return carrega(attr(im, 'name')); }));
  }
  function dentroDe(no, alvo, tag) {
    var achou = false;
    (function anda(x, dentro) {
      if (achou || !x.filhos) return;
      x.filhos.forEach(function (f) { if (f === alvo) achou = achou || dentro; else if (f.tag) anda(f, dentro || f.tag === tag); });
    })(no, false);
    return achou;
  }
  var naCabeca = new Set();
  function poeNaCabeca(no) {
    if (!no.tag || no.tag === 'script' || no.tag === 'title' || no.tag === 'meta') return;
    var chave = no.tag + '|' + (attr(no, 'href') || '') + '|' + (no.filhos[0] ? no.filhos[0].texto : '');
    if (naCabeca.has(chave)) return;
    naCabeca.add(chave);
    var el = document.createElement(no.tag);
    no.attrs.forEach(function (a) { el.setAttribute(a[0], decodifica(a[1])); });
    if (no.filhos[0]) el.textContent = no.filhos[0].texto;
    document.head.appendChild(el);
  }
  var carregando = {};
  function carrega(nome) {
    if (!nome) return Promise.resolve();
    if (!carregando[nome]) {
      carregando[nome] = fetch(new URL(nome + '.dc.html', location.href).href)
        .then(function (r) { if (!r.ok) throw new Error('HTTP ' + r.status); return r.text(); })
        .then(function (t) { return define(nome, analisa(t)); })
        .catch(function (e) { console.error('motor-dc: não carregou ' + nome + '.dc.html: ' + e.message); });
    }
    return carregando[nome];
  }

  // ---------- Avaliação de {{ }} ----------
  var reUnico = /^\{\{([^}]*)\}\}$/, reVarios = /\{\{([^}]*)\}\}/g;
  function resolve(expr, escopo) {
    expr = expr.trim();
    if (expr === 'true') return true;
    if (expr === 'false') return false;
    if (expr === 'null') return null;
    if (/^-?\d+(\.\d+)?$/.test(expr)) return +expr;
    if (/^(['"]).*\1$/.test(expr)) return expr.slice(1, -1);
    if (expr[0] === '!') return !resolve(expr.slice(1), escopo);
    var partes = expr.split('.'), e = escopo, v;
    while (e) {
      if (e.vars != null && Object.prototype.hasOwnProperty.call(e.vars, partes[0])) { v = e.vars[partes[0]]; break; }
      e = e.pai;
    }
    if (!e) return undefined;
    for (var k = 1; k < partes.length; k++) v = v == null ? undefined : v[partes[k]];
    return v;
  }
  function comoTexto(v) {
    if (v == null || typeof v === 'boolean') return '';
    if (Array.isArray(v)) return v.map(comoTexto).join('');
    return String(v);
  }
  function valorDe(bruto, escopo) {
    var m = reUnico.exec(bruto);
    if (m) return resolve(m[1], escopo);
    var partes = bruto.split(reVarios), txt = '';
    for (var k = 0; k < partes.length; k++) txt += k % 2 ? comoTexto(resolve(partes[k], escopo)) : decodifica(partes[k]);
    return txt;
  }
  function estiloDe(v) {
    if (v && typeof v === 'object') {
      return Object.keys(v).map(function (k) { return k.replace(/[A-Z]/g, function (c) { return '-' + c.toLowerCase(); }) + ':' + v[k]; }).join(';');
    }
    return comoTexto(v);
  }

  // ---------- Renderização ----------
  var instancias = new Map(), vistas = null;
  function desenhaComponente(chave, nome, props, destino) {
    var def = definicoes[nome];
    if (!def) return;
    vistas.add(chave);
    var reg = instancias.get(chave), nova = false;
    if (!reg) {
      var inst = null;
      if (def.Comp) {
        try { inst = new def.Comp(props); } catch (e) { console.error('motor-dc: erro no construtor de ' + nome + ': ' + e.message); }
      }
      reg = { inst: inst || new DCLogic(props) };
      instancias.set(chave, reg);
      nova = true;
    }
    reg.inst.props = props;
    var vals = {};
    if (typeof reg.inst.renderVals === 'function') {
      try { vals = reg.inst.renderVals() || {}; } catch (e) { console.error('motor-dc: erro em renderVals de ' + nome + ': ' + e.message); return; }
    }
    var escopo = { vars: vals, pai: { vars: reg.inst.state, pai: { vars: props } } };
    var ctx = { chave: chave, cont: 0 };
    desenhaNos(def.modelo, escopo, destino, ctx, false);
    if (nova && typeof reg.inst.componentDidMount === 'function') {
      Promise.resolve().then(function () { reg.inst.componentDidMount(); });
    }
  }

  function desenhaNos(nos, escopo, destino, ctx, svg) {
    for (var k = 0; k < nos.length; k++) desenhaNo(nos[k], escopo, destino, ctx, svg);
  }

  function desenhaNo(no, escopo, destino, ctx, svg) {
    if (!no.tag) {
      if (no.bruto) { destino.appendChild(document.createTextNode(no.texto)); return; }
      var partes = no.texto.split(reVarios), txt = '';
      for (var p = 0; p < partes.length; p++) txt += p % 2 ? comoTexto(resolve(partes[p], escopo)) : decodifica(partes[p]);
      if (txt) destino.appendChild(document.createTextNode(txt));
      return;
    }
    switch (no.tag) {
      case 'sc-if':
        if (resolve((reUnico.exec(attr(no, 'value') || '') || [0, 'false'])[1], escopo)) desenhaNos(no.filhos, escopo, destino, ctx, svg);
        return;
      case 'sc-for': {
        var lista = valorDe(attr(no, 'list') || '', escopo), como = attr(no, 'as') || 'item';
        if (!lista || typeof lista.forEach !== 'function') return;
        lista.forEach(function (item, idx) {
          var vars = { $index: idx }; vars[como] = item;
          desenhaNos(no.filhos, { vars: vars, pai: escopo }, destino, ctx, svg);
        });
        return;
      }
      case 'dc-import': {
        var nome = attr(no, 'name'), props = {};
        no.attrs.forEach(function (a) {
          if (a[0] === 'name' || a[0].indexOf('hint-') === 0) return;
          props[a[0]] = valorDe(a[1], escopo);
        });
        desenhaComponente(ctx.chave + '/' + nome + '#' + (ctx.cont++), nome, props, destino);
        return;
      }
      case 'helmet': return;
      case 'script': if (attr(no, 'type') === 'text/x-dc' || attr(no, 'src')) return; break;
    }
    var emSvg = svg || no.tag === 'svg';
    var el = emSvg ? document.createElementNS(SVG, no.nome) : document.createElement(no.tag);
    var valor, temValor = false;
    no.attrs.forEach(function (a) {
      var nome = a[0], baixo = nome.toLowerCase();
      if (baixo.indexOf('hint-') === 0 || baixo === 'key') return;
      var v = valorDe(a[1], escopo);
      if (/^on[A-Z]/.test(nome)) {
        if (typeof v === 'function') {
          var ev = baixo.slice(2);
          if (ev === 'change') ev = 'input';
          if (ev === 'doubleclick') ev = 'dblclick';
          el.addEventListener(ev, v);
        }
        return;
      }
      if (baixo === 'value' && !emSvg) { valor = v; temValor = true; }
      if (BOOLEANOS.has(baixo) && !emSvg) {
        var liga = reUnico.test(a[1]) ? !!v && v !== 'false' : true;
        if (liga) el.setAttribute(baixo, '');
        if (baixo === 'checked' || baixo === 'selected') el[baixo] = liga;
        return;
      }
      if (baixo === 'style') { el.setAttribute('style', estiloDe(v)); return; }
      if (v === false && !/^(aria|data)-/.test(baixo)) return;
      if (v == null) return;
      el.setAttribute(emSvg ? nome : baixo, typeof v === 'string' ? v : comoTexto(v) || String(v));
    });
    if (BRUTOS.has(no.tag) && no.tag !== 'textarea') {
      el.textContent = no.filhos.map(function (t) { return t.texto; }).join('');
    } else {
      desenhaNos(no.filhos, escopo, el, ctx, emSvg && no.tag !== 'foreignobject');
    }
    if (temValor) el.value = comoTexto(valor);
    destino.appendChild(el);
  }

  // ---------- Ciclo: agendar, desenhar, preservar foco ----------
  var hospedeiro = null, agendado = false;
  function agenda() {
    if (agendado || !hospedeiro) return;
    agendado = true;
    Promise.resolve().then(desenha);
  }
  function caminhoDe(el) {
    var c = [];
    while (el && el !== hospedeiro) { c.unshift(Array.prototype.indexOf.call(el.parentNode.childNodes, el)); el = el.parentNode; }
    return el === hospedeiro ? c : null;
  }
  function desenha() {
    agendado = false;
    var ativo = document.activeElement, foco = null;
    if (ativo && hospedeiro.contains(ativo)) {
      foco = { caminho: caminhoDe(ativo), tag: ativo.tagName, ini: ativo.selectionStart, fim: ativo.selectionEnd };
    }
    vistas = new Set();
    var frag = document.createDocumentFragment();
    desenhaComponente('raiz', '#pagina', definicoes['#pagina'].props, frag);
    hospedeiro.replaceChildren(frag);
    instancias.forEach(function (reg, chave) {
      if (vistas.has(chave)) return;
      if (typeof reg.inst.componentWillUnmount === 'function') {
        try { reg.inst.componentWillUnmount(); } catch (e) { console.error('motor-dc: ' + e.message); }
      }
      instancias.delete(chave);
    });
    if (foco && foco.caminho) {
      var el = hospedeiro;
      foco.caminho.forEach(function (ix) { el = el && el.childNodes[ix]; });
      if (el && el.tagName === foco.tag && el.focus) {
        el.focus();
        try { if (foco.ini != null) el.setSelectionRange(foco.ini, foco.fim); } catch (e) { /* campo sem seleção */ }
      }
    }
    document.documentElement.setAttribute('data-dc-pronto', '1');
  }

  function inicia() {
    var bruto = document.getElementById('__dc_fonte');
    if (!bruto) { console.error('motor-dc: código-fonte da página não capturado'); return; }
    var fonte = bruto.textContent;
    bruto.remove();
    hospedeiro = document.createElement('x-dc');
    hospedeiro.style.display = 'contents';
    document.body.appendChild(hospedeiro);
    define('#pagina', analisa(fonte)).then(function () { agendado = true; desenha(); });
  }
  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', inicia);
  else inicia();
})();
