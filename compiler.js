// ============================================================================
// AXCode — kompilator w przeglądarce
// Jeden plik. Cztery warstwy: lexer, parser, weryfikator, runtime.
// Autor: Axel G
// Instagram: https://www.instagram.com/axelgadomski
// ============================================================================

// ---------------------------------------------------------------------------
// WARSTWA 1 — LEXER
// Tekst źródłowy na tokeny. Każdy token niesie rodzaj, numer linii,
// poziom wcięcia i treść. Wcięcie jest znaczące, jak w języku AXCode.
// ---------------------------------------------------------------------------

const KEYWORDS = new Set([
  "app","version","language","supports","orientation","offline","entry","minOs",
  "theme","style","enum","data","store","global","state","permission","api","function",
  "component","screen","test","ui","accessibility","returns","single","list","local",
  "synced","secure","encrypted","queue","cache","follows","system","true","false",
  "if","else","for","in","return","set","add","remove","update","save","load","clear",
  "when","tapped","changed","appears","disappears","launches","valid","invalid","error",
  "online","offline","granted","denied","go","to","back","open","close","show","use",
  "try","catch","finally","and","or","is","not","none","where","order","by","ascending",
  "descending","empty","item","column","row","grid","card","scroll","spacer","text",
  "button","field","image","icon","checkbox","audioPlayer","videoPlayer","cameraPreview",
  "value","then","raise","confirm","ask","multipart","body","get","post","put","patch",
  "delete","base","auth","bearer","timeout","retry","purpose","required","precision",
  "duration","limited","approximate","label","placeholder","error","type","min","max",
  "length","email","password","number","disabled","style","color","alt","role","state",
  "padding","margin","gap","align","justify","width","height","overflow","columns",
  "direction","vertical","horizontal","fill","center","start","end","stretch","spaceBetween",
  "full","auto","half","visible","hidden","scroll","controls","subtitles","facing","user",
  "announce","vibrate","short","medium","long","round","planar",
]);

function lex(source) {
  const lines = source.split("\n");
  const tokens = [];

  for (let i = 0; i < lines.length; i++) {
    const raw = lines[i];
    const lineNo = i + 1;

    // pomijamy puste linie
    if (raw.trim().length === 0) {
      tokens.push({ kind: "blank", line: lineNo });
      continue;
    }

    // wcięcie
    const indentMatch = raw.match(/^[ \t]*/);
    const indent = indentMatch[0].replace(/\t/g, "  ").length;
    let rest = raw.slice(indentMatch[0].length);

    // komentarz
    if (rest.startsWith("//")) {
      tokens.push({ kind: "comment", line: lineNo, indent, text: rest.slice(2).trim() });
      continue;
    }

    tokens.push(lexLine(rest, lineNo, indent));
  }

  return tokens;
}

function lexLine(text, line, indent) {
  // deklaracja bloku: "screen Chat", "store outbox local encrypted", "api X"
  const blockMatch = text.match(/^([a-zA-Z]+)(?:\s+(.+))?$/);
  const head = text.split(/\s+/)[0];

  // linia jest deklaracją bloku, gdy pierwsze słowo jest słowem kluczowym-deklaracją
  const blockDeclarators = new Set([
    "app","version","language","supports","orientation","offline","entry","minOs",
    "theme","style","enum","data","store","global","permission","api","function",
    "component","screen","test","ui","accessibility",
  ]);

  if (blockDeclarators.has(head)) {
    return { kind: "decl", line, indent, keyword: head, rest: text.slice(head.length).trim(), raw: text };
  }

  // wywołanie: nazwa(argumenty)
  const callMatch = text.match(/^([a-zA-Z][a-zA-Z0-9_]*)\((.*)\)\s*$/);
  if (callMatch) {
    return { kind: "call", line, indent, name: callMatch[1], args: callMatch[2], raw: text };
  }

  // wszystko inne: pole / akcesorium / instrukcja
  return { kind: "stmt", line, indent, raw: text };
}



// ---------------------------------------------------------------------------
// WARSTWA 2 — PARSER
// Tokeny na drzewo składniowe. Rozpoznaje dziewięć jednostek programu
// i buduje drzewo interfejsu ze stosu wcięć.
// ---------------------------------------------------------------------------


const BLOCK_UNITS = new Set(["app","theme","style","enum","data","store","global","permission","api","function","component","screen","test"]);

function parse(source) {
  const tokens = lex(source).filter(t => t.kind !== "blank");
  const ast = {
    app: {},
    theme: null,
    styles: [],
    enums: [],
    data: [],
    stores: [],
    globals: [],
    permissions: [],
    apis: [],
    functions: [],
    components: [],
    screens: [],
    tests: [],
    diagnostics: [],
  };

  let i = 0;
  while (i < tokens.length) {
    const t = tokens[i];

    if (t.kind === "comment") { i++; continue; }

    // app-level pary klucz-wartość na poziomie 0
    if (t.kind === "decl" && t.indent === 0) {
      if (t.keyword === "ui" || t.keyword === "accessibility") {
        // "ui test" / "accessibility test"
        const sub = (t.rest || "").split(/\s+/)[0];
        const { body, next } = collectBody(tokens, i + 1, 0);
        ast.tests.push({ kind: `${t.keyword} test`, name: t.rest.slice(sub.length).trim(), body, line: t.line });
        i = next;
        continue;
      }

      if (t.keyword === "app" || t.keyword === "version" || t.keyword === "language" ||
          t.keyword === "supports" || t.keyword === "orientation" || t.keyword === "offline" ||
          t.keyword === "entry" || t.keyword === "minOs") {
        ast.app[t.keyword] = stripQuotes(t.rest);
        i++;
        continue;
      }

      const { body, next } = collectBody(tokens, i + 1, 0);

      if (t.keyword === "theme") {
        ast.theme = { name: stripQuotes(t.rest), body, line: t.line };
      } else if (t.keyword === "style") {
        ast.styles.push({ name: t.rest.trim(), body, line: t.line });
      } else if (t.keyword === "enum") {
        ast.enums.push({ name: t.rest.trim(), values: body.filter(b => b.kind === "stmt").map(b => b.raw.trim()), body, line: t.line });
      } else if (t.keyword === "data") {
        ast.data.push({ name: t.rest.trim(), fields: parseFields(body), body, line: t.line });
      } else if (t.keyword === "store") {
        const parts = t.rest.trim().split(/\s+/);
        ast.stores.push({ name: parts[0], modifiers: parts.slice(1), body, line: t.line });
      } else if (t.keyword === "global") {
        ast.globals.push(parseGlobal(t.rest, body));
      } else if (t.keyword === "permission") {
        ast.permissions.push({ name: t.rest.trim(), body, line: t.line });
      } else if (t.keyword === "api") {
        ast.apis.push(parseApi(t.rest.trim(), body));
      } else if (t.keyword === "function") {
        ast.functions.push(parseFunction(t.rest.trim(), body, t.line));
      } else if (t.keyword === "component") {
        ast.components.push(parseComponent(t.rest.trim(), body, t.line));
      } else if (t.keyword === "screen") {
        ast.screens.push(parseScreen(t.rest.trim(), body, t.line));
      } else if (t.keyword === "test") {
        ast.tests.push({ kind: "test", name: stripQuotes(t.rest), body, line: t.line });
      }

      i = next;
      continue;
    }

    ast.diagnostics.push({ line: t.line, message: `Nieoczekiwana linia poza blokiem: ${t.raw}` });
    i++;
  }

  return ast;
}

// Zbiera ciało bloku: wszystkie tokeny o wcięciu większym niż rodzic
function collectBody(tokens, start, parentIndent) {
  const body = [];
  let i = start;
  while (i < tokens.length) {
    const t = tokens[i];
    if (t.kind === "comment") { i++; continue; }
    if (t.indent <= parentIndent) break;
    body.push(t);
    i++;
  }
  return { body, next: i };
}

function parseFields(body) {
  return body.filter(b => b.kind === "stmt").map(b => {
    const parts = b.raw.trim().split(/\s+/);
    return { name: parts[0], type: parts[1] || "Text", optional: (parts[1] || "").endsWith("?") };
  });
}

function parseGlobal(rest, body) {
  const parts = (rest || "").trim().split(/\s+/);
  // "state nazwa Typ = wartość"
  const idx = parts.indexOf("state");
  const name = parts[idx + 1] || "nieznana";
  const type = parts[idx + 2] || "Text";
  const eq = rest.indexOf("=");
  return { name, type, initial: eq > -1 ? rest.slice(eq + 1).trim() : null, line: body[0] ? body[0].line : 0 };
}

function parseApi(name, body) {
  const endpoints = [];
  let config = {};
  for (let i = 0; i < body.length; i++) {
    const t = body[i];
    if (t.kind === "decl" && t.keyword === "function") {
      const { body: fnBody, next } = collectBody(body, i + 1, t.indent);
      endpoints.push(parseFunction(t.rest.trim(), fnBody, t.line));
      i = next - 1;
    } else if (t.kind === "stmt") {
      const parts = t.raw.trim().split(/\s+/);
      if (parts.length >= 2) config[parts[0]] = parts.slice(1).join(" ");
    }
  }
  return { name, config, endpoints, line: body[0] ? body[0].line : 0 };
}

function parseFunction(rest, body, line) {
  // "login(email Text, password Text) returns Json"  |  "sendMessage(body Text)"
  const m = rest.match(/^([A-Za-z0-9_]+)\s*\(([^)]*)\)\s*(?:returns\s+(\S+))?/);
  if (!m) return { name: rest, params: [], returns: null, body, line };
  const params = m[2].split(",").map(p => p.trim()).filter(Boolean).map(p => {
    const parts = p.split(/\s+/);
    return { name: parts[0], type: parts[1] || "Text" };
  });
  return { name: m[1], params, returns: m[3] || null, body, line };
}

function parseComponent(rest, body, line) {
  const m = rest.match(/^([A-Za-z0-9_]+)\s*\(([^)]*)\)/);
  const params = m ? m[2].split(",").map(p => p.trim()).filter(Boolean).map(p => {
    const parts = p.split(/\s+/);
    return { name: parts[0], type: parts[1] || "Text" };
  }) : [];
  return { name: m ? m[1] : rest, params, tree: buildTree(body), body, line };
}

function parseScreen(rest, body, line) {
  const parts = rest.split(/\s+/);
  const name = parts[0];
  const params = parts.slice(1).map(p => {
    const bits = p.split(/\s+/);
    return { name: bits[0], type: bits[1] || "Text" };
  });
  return { name, params, tree: buildTree(body), body, line };
}

// Buduje drzewo UI z tokenów, używając wcięć
function buildTree(tokens) {
  const root = { kind: "root", children: [] };
  const stack = [{ node: root, indent: -1 }];

  for (const t of tokens) {
    if (t.kind === "comment") continue;

    while (stack.length > 1 && t.indent <= stack[stack.length - 1].indent) stack.pop();
    const parent = stack[stack.length - 1].node;

    const node = tokenToNode(t);
    if (!node) continue;

    parent.children.push(node);
    if (node.children) stack.push({ node, indent: t.indent });
  }

  return root;
}

const CONTAINERS = new Set(["column","row","grid","card","scroll","spacer","list","item","empty"]);

function tokenToNode(t) {
  if (t.kind === "decl") {
    // "state draft Text = ''" wewnątrz ekranu
    if (t.keyword === "state") {
      const m = t.rest.match(/^([A-Za-z0-9_.]+)\s+(\S+)\s*(?:=\s*(.+))?$/);
      return {
        kind: "state",
        name: m ? m[1] : t.rest,
        type: m ? m[2] : "Text",
        initial: m && m[3] ? m[3].trim() : null,
        line: t.line,
        children: null,
      };
    }
    return { kind: "raw", text: t.raw.trim(), line: t.line, children: null };
  }

  if (t.kind === "call") {
    return { kind: "call", name: t.name, args: t.args, line: t.line, children: null };
  }

  const word = t.raw.trim().split(/\s+/)[0];

  if (CONTAINERS.has(word)) {
    return { kind: "container", name: word, line: t.line, children: [] };
  }
  if (word === "text") {
    const m = t.raw.trim().match(/^text\s+(.+)$/);
    return { kind: "text", value: m ? m[1].trim() : "", line: t.line, children: [] };
  }
  if (word === "button") {
    const m = t.raw.trim().match(/^button\s+(.+)$/);
    return { kind: "button", value: m ? m[1].trim() : "", line: t.line, children: [] };
  }
  if (word === "field") {
    const m = t.raw.trim().match(/^field\s+([A-Za-z0-9_.]+)$/);
    return { kind: "field", name: m ? m[1] : t.raw.trim(), line: t.line, children: [] };
  }
  if (word === "image") {
    const m = t.raw.trim().match(/^image\s+(.+)$/);
    return { kind: "image", ref: m ? m[1].trim() : "", line: t.line, children: [] };
  }
  if (word === "checkbox") {
    const m = t.raw.trim().match(/^checkbox\s+(.+)$/);
    return { kind: "checkbox", ref: m ? m[1].trim() : "", line: t.line, children: [] };
  }
  if (word === "audioPlayer" || word === "videoPlayer" || word === "cameraPreview") {
    return { kind: "media", name: word, line: t.line, children: [] };
  }
  if (word === "when") {
    return { kind: "when", event: t.raw.trim().slice(4).trim(), line: t.line, children: [] };
  }
  if (word === "if") {
    return { kind: "if", condition: t.raw.trim().slice(2).trim(), line: t.line, children: [] };
  }
  if (word === "else") {
    return { kind: "else", line: t.line, children: [] };
  }
  if (word === "for") {
    return { kind: "for", spec: t.raw.trim().slice(3).trim(), line: t.line, children: [] };
  }
  if (word === "list") {
    const m = t.raw.trim().match(/^list\s+(.+)$/);
    return { kind: "container", name: "list", source: m ? m[1].trim() : "", line: t.line, children: [] };
  }

  return { kind: "raw", text: t.raw.trim(), line: t.line, children: null };
}

function stripQuotes(s) {
  if (!s) return "";
  return s.trim().replace(/^"|"$/g, "");
}



// ---------------------------------------------------------------------------
// WARSTWA 3 — WERYFIKATOR SEMANTYCZNY
// Reguły, których kompilator AXCode wymaga na etapie kompilacji:
// dostępność, typy, magazyny, bezpieczeństwo, nawigacja, warstwa API.
// Kody błędów zachowują konwencję języka (AX1000–AX9999).
// ---------------------------------------------------------------------------
 Sprawdza to, co kompilator AXCode wymusza na etapie kompilacji:
// dostępność, nawigację, typy i klasę przechowywania sekretów.

function verify(ast) {
  const findings = [];

  const screenNames = new Set(ast.screens.map(s => s.name));
  const componentNames = new Set(ast.components.map(c => c.name));
  const functionNames = new Set(ast.functions.map(f => f.name));
  const dataNames = new Set(ast.data.map(d => d.name));
  const enumNames = new Set(ast.enums.map(e => e.name));
  const storeNames = new Set(ast.stores.map(s => s.name));
  const globalNames = new Set(ast.globals.map(g => g.name));
  const styleNames = new Set(ast.styles.map(s => s.name));

  const known = new Set([...screenNames, ...componentNames, ...functionNames, ...dataNames, ...enumNames, ...storeNames, ...globalNames, ...styleNames]);

  // --- AX7xxx: dostępność ---
  walkAll(ast, (node, screenName) => {
    if (node.kind === "image" && !node.children.some(c => c.kind === "raw" && c.text.startsWith("alt "))) {
      findings.push(ax(7001, "UI", `Obraz bez tekstu alternatywnego w ekranie ${screenName} (linia ${node.line}).`, "Dodaj: alt \"opis obrazu\"."));
    }
    if (node.kind === "button") {
      const hasLabel = node.children.some(c => c.kind === "raw" && c.text.startsWith("accessibility label"));
      const isIcon = /^icon\s+"/.test(node.value || "");
      if (isIcon && !hasLabel) {
        findings.push(ax(7002, "UI", `Przycisk ikonowy bez accessibility label w ekranie ${screenName} (linia ${node.line}).`, "Dodaj: accessibility label \"opis akcji\"."));
      }
    }
    if (node.kind === "field") {
      const hasLabel = node.children.some(c => c.kind === "raw" && c.text.startsWith("label "));
      if (!hasLabel) {
        findings.push(ax(7003, "UI", `Pole ${node.name} bez etykiety w ekranie ${screenName} (linia ${node.line}).`, "Dodaj: label \"Nazwa pola\"."));
      }
    }
  });

  // --- AX4xxx: stan i magazyny ---
  const globalDeclared = new Set(ast.globals.map(g => g.name));
  for (const s of ast.screens) {
    for (const t of s.body) {
      if (t.kind === "decl" && t.keyword === "state") {
        const m = t.rest.match(/^([A-Za-z0-9_.]+)\s+(\S+)\s*(?:=\s*(.+))?$/);
        if (!m) {
          findings.push(ax(4001, "Stan", `Nieprawidłowa deklaracja stanu w ekranie ${s.name} (linia ${t.line}): ${t.rest}`, "Poprawna forma: state nazwa Typ = wartość"));
        }
      }
    }
  }

  // zapisy do magazynów, które nie istnieją
  walkBody(ast, (t, where) => {
    if (t.kind !== "stmt" && t.kind !== "raw") return;
    const text = (t.raw || t.text || "").trim();
    const saveMatch = text.match(/^(save|load|clear)\s+([A-Za-z0-9_]+)$/);
    if (saveMatch && !storeNames.has(saveMatch[2])) {
      findings.push(ax(4002, "Magazyn", `Operacja ${saveMatch[1]} na magazynie "${saveMatch[2]}", którego nie ma w deklaracjach (linia ${t.line}).`, `Zadeklaruj: store ${saveMatch[2]} local`));
    }
  });

  // --- AX5xxx: API i sieć ---
  for (const api of ast.apis) {
    if (!api.config.timeout) {
      findings.push(ax(5001, "API", `API ${api.name} nie ma limitu czasu (timeout).`, "Dodaj: timeout 20000"));
    }
    if (!api.config.retry) {
      findings.push(ax(5002, "API", `API ${api.name} nie ma ponawiania (retry).`, "Dodaj: retry 3"));
    }
  }

  // --- AX8xxx: platforma / nawigacja ---
  walkBody(ast, (t, where) => {
    const text = (t.raw || t.text || "").trim();
    const goMatch = text.match(/^go to\s+([A-Za-z0-9_]+)/);
    if (goMatch && !screenNames.has(goMatch[1])) {
      findings.push(ax(8001, "Nawigacja", `Nawigacja do ekranu "${goMatch[1]}", który nie istnieje (linia ${t.line}).`, "Popraw nazwę ekranu albo dodaj brakujący ekran."));
    }
    const styleMatch = text.match(/^style\s+(\S+)$/);
    if (styleMatch && !styleNames.has(styleMatch[1])) {
      findings.push(ax(3001, "UI", `Odwołanie do stylu "${styleMatch[1]}", którego nie ma (linia ${t.line}).`, "Dodaj styl albo popraw nazwę."));
    }
  });

  // --- AX6xxx: bezpieczeństwo ---
  for (const store of ast.stores) {
    const isSecureClass = store.modifiers.includes("secure") || store.modifiers.includes("encrypted");
    const nameLooksSecret = /key|token|secret|session|credential/i.test(store.name);
    if (nameLooksSecret && !isSecureClass) {
      findings.push(ax(6001, "Bezpieczeństwo", `Magazyn "${store.name}" wygląda na przechowujący sekrety, ale nie ma klasy secure ani encrypted.`, `Zmień na: store ${store.name} secure`));
    }
  }

  // --- AX2xxx: typy ---
  for (const d of ast.data) {
    for (const f of d.fields) {
      const baseType = f.type.replace("?", "").replace(/^List</, "").replace(/>$/, "");
      const builtin = ["Text","Number","Boolean","Date","Duration","Json","File","Image","Video","Audio","Location"];
      if (!builtin.includes(baseType) && !known.has(baseType)) {
        findings.push(ax(2001, "Typy", `Pole ${d.name}.${f.name} używa nieznanego typu "${f.type}".`, "Użyj typu wbudowanego albo zadeklaruj data/enum o tej nazwie."));
      }
    }
  }

  return findings;
}

function walkAll(ast, visit) {
  for (const screen of ast.screens) {
    walkNode(screen.tree, node => visit(node, screen.name));
  }
  for (const comp of ast.components) {
    walkNode(comp.tree, node => visit(node, `component ${comp.name}`));
  }
}

function walkNode(node, visit) {
  visit(node);
  if (!node.children) return;
  for (const c of node.children) walkNode(c, visit);
}

function walkBody(ast, visit) {
  const scopes = [
    ...ast.functions.map(f => ({ body: f.body, where: f.name })),
    ...ast.screens.map(s => ({ body: s.body, where: s.name })),
    ...ast.screens.flatMap(s => (s.tree ? flatten(s.tree) : [])).map(n => ({ body: n.children || [], where: "UI" })),
  ];
  for (const scope of scopes) {
    for (const t of scope.body) visit(t, scope.where);
  }
}

function flatten(node) {
  const out = [node];
  if (node.children) for (const c of node.children) out.push(...flatten(c));
  return out;
}

function ax(code, category, message, fix) {
  return { code: `AX${code}`, category, message, fix };
}



// -------------------------------------------------------------------------
// WARSTWA 4 — RUNTIME RENDERUJĄCY
// Drzewo składniowe na drzewo dokumentu. Obsługuje kontenery, listy
// z pustym stanem, komponenty z parametrami, warunki, zdarzenia,
// nawigację i stan lokalny oraz globalny.
// ---------------------------------------------------------------------------
 Zamienia AST na drzewo DOM w przeglądarce.
// Bez frameworka: renderer tworzy elementy, wiąże zdarzenia i utrzymuje stan.

const ThemeDefaults = {
  primary: "#7C3AED",
  accent: "#06B6D4",
  danger: "#DC2626",
  success: "#16A34A",
  warning: "#D97706",
};

function createRuntime(ast, options) {
  const opts = options || {};
  const state = {
    // struktura programu
    app: ast.app || {},
    screens: new Map(),
    components: new Map(),
    functions: new Map(),
    data: new Map(),
    enums: new Map(),
    stores: new Map(),
    globals: new Map(),
    permissions: new Map(),
    apis: new Map(),
    styles: new Map(),
    tests: ast.tests || [],

    // stan wykonania
    global: new Map(),
    screenState: new Map(),
    currentScreen: ast.app && ast.app.entry ? ast.app.entry : null,
    enumValues: new Map(),
    permissionDecls: [],

    // warstwy
    persistence: options && options.persistence ? options.persistence : createPersistence(opts.namespace),
    apiObjects: new Map(),
    apiCalls: [],
    apiResults: [],
    apiErrors: [],
    storeWrites: [],
    diagnostics: [],
    announcements: [],
    navigationLog: [],
  };

  // rejestracja jednostek
  for (const d of ast.data) state.data.set(d.name, d);
  for (const e of ast.enums) {
    state.enums.set(e.name, e.values);
    state.enumValues.set(e.name, e.values);
    state.data.set(e.name, { enumValues: e.values });
  }
  for (const s of ast.styles) state.styles.set(s.name, parseStyle(s.body));
  for (const f of ast.functions) state.functions.set(f.name, f);
  for (const c of ast.components) state.components.set(c.name, c);
  for (const a of ast.apis) state.apis.set(a.name, a);
  for (const st of ast.stores) {
    state.stores.set(st.name, st);
    // magazyn przyjmuje dane z pamięci trwałej albo startuje pusty
    const restored = state.persistence.get(st.name);
    const initial = Array.isArray(restored) ? restored : [];
    state.global.set(st.name, initial);
  }
  for (const g of ast.globals) state.global.set(g.name, evalLiteral(g.initial));
  for (const p of ast.permissions) state.permissionDecls.push({ name: p.name, body: p.body });
  for (const sc of ast.screens) state.screens.set(sc.name, sc);

  // warstwa API budowana na podstawie deklaracji
  createApiLayer(state);
  state.permissionLayer = createPermissionLayer(state);
  state.builtins = { evalExpression, executeBlock, invokeFunction };

  return state;
}

function parseStyle(body) {
  const out = {};
  for (const t of body) {
    if (t.kind === "stmt") {
      const m = t.raw.trim().match(/^([a-zA-Z]+)\s+(.+)$/);
      if (m) out[m[1]] = m[2].trim();
    }
  }
  return out;
}

function evalLiteral(raw) {
  if (raw === null || raw === undefined) return null;
  const s = String(raw).trim();
  if (s === "true") return true;
  if (s === "false") return false;
  if (s === "none") return null;
  if (/^-?[0-9.]+$/.test(s)) return Number(s);
  if (s.startsWith('"') && s.endsWith('"')) return s.slice(1, -1);
  if (s.startsWith("[") && s.endsWith("]")) {
    return s.slice(1, -1).split(",").map(x => evalLiteral(x)).filter(x => x !== "");
  }
  if (s.startsWith("{") && s.endsWith("}")) {
    const obj = {};
    for (const pair of s.slice(1, -1).split(",")) {
      const [k, v] = pair.split(":").map(x => x && x.trim());
      if (k) obj[k] = evalLiteral(v);
    }
    return obj;
  }
  return s;
}

// ---------------------------------------------------------------------------
// Renderer: AST ekranu -> DOM
// ---------------------------------------------------------------------------

function renderApp(ast, mountId, options) {
  const runtime = createRuntime(ast, options);
  const mount = document.getElementById(mountId || "app");
  if (!mount) return runtime;

  const entryName = (ast.app && ast.app.entry) || (ast.screens[0] && ast.screens[0].name);
  const screen = runtime.screens.get(entryName);
  if (!screen) {
    mount.innerHTML = "<p>Nie znaleziono ekranu startowego.</p>";
    return runtime;
  }

  applyTheme(ast, runtime);
  applyAccessibilityPreferences(runtime);

  // zdarzenie "when app launches" na dowolnym ekranie
  for (const s of runtime.screens.values()) {
    const launch = findWhen(s.tree, "app launches");
    if (launch) {
      const scope = createScope(runtime, runtime.screenState.get(s.name) || new Map(), null, null, new Map());
      executeBlock(launch.children, scope);
    }
  }

  mount.innerHTML = "";
  mount.appendChild(renderScreen(screen, runtime));
  return runtime;
}

function applyAccessibilityPreferences(runtime) {
  if (typeof document === "undefined") return;
  const prefs = readAccessibilityPreferences();
  runtime.accessibilityPrefs = prefs;
  const root = document.documentElement;
  if (prefs.reduceMotion) root.setAttribute("data-ax-reduce-motion", "true");
  if (prefs.highContrast) root.setAttribute("data-ax-high-contrast", "true");
}

function applyTheme(ast, state) {
  const theme = ast.theme;
  const palette = { ...ThemeDefaults };
  if (theme) {
    for (const t of theme.body) {
      if (t.kind === "stmt") {
        const m = t.raw.trim().match(/^color\s+([a-zA-Z]+)\s+(?:light\s+|dark\s+)?(#[0-9A-Fa-f]{3,8})$/);
        if (m) palette[m[1]] = m[2];
      }
    }
  }
  const root = document.documentElement;
  for (const [k, v] of Object.entries(palette)) root.style.setProperty(`--ax-${k}`, v);
}

function renderScreen(screen, runtime, argText) {
  const wrapper = el("div", "ax-screen");
  wrapper.setAttribute("data-screen", screen.name);
  runtime.currentScreen = screen.name;

  // parametry ekranu przekazane w nawigacji
  const params = new Map();
  if (argText && screen.params && screen.params.length > 0) {
    screen.params.forEach((p, i) => params.set(p.name, evalLiteral(argText)));
  }

  // stan lokalny ekranu — zachowany między renderami
  let local = runtime.screenState.get(screen.name);
  if (!local) {
    local = new Map();
    for (const t of screen.body) {
      if (t.kind === "decl" && t.keyword === "state") {
        const m = t.rest.match(/^([A-Za-z0-9_.]+)\s+(\S+)\s*(?:=\s*(.+))?$/);
        if (m) local.set(m[1], evalLiteral(m[3]));
      }
    }
    runtime.screenState.set(screen.name, local);
  }

  const scope = createScope(runtime, local, null, null, params);

  // treść ekranu bez deklaracji stanu
  const treeWithoutState = { kind: "root", children: screen.tree.children.filter(n => n.kind !== "state") };
  const container = renderTree(treeWithoutState, runtime, screen, local, params);
  wrapper.appendChild(container);

  // zdarzenie "when screen appears"
  const appear = findWhen(screen.tree, "screen appears");
  if (appear) executeBlock(appear.children, scope);

  // zdarzenie "when screen disappears" na poprzednim ekranie pomijamy — brak historii

  return wrapper;
}

function renderTree(node, state, screen, local) {
  const frag = document.createElement("div");
  frag.className = "ax-tree";

  for (const child of node.children) {
    const rendered = renderNode(child, state, screen, local);
    if (rendered) frag.appendChild(rendered);
  }

  return frag;
}

function renderNode(node, state, screen, local) {
  if (!node) return null;

  switch (node.kind) {
    case "container":
      return renderContainer(node, state, screen, local);
    case "text":
      return renderText(node, state, screen, local);
    case "button":
      return renderButton(node, state, screen, local);
    case "field":
      return renderField(node, state, screen, local);
    case "image":
      return renderImage(node, state, screen, local);
    case "checkbox":
      return renderCheckbox(node, state, screen, local);
    case "media":
      return renderMedia(node, state, screen, local);
    case "call":
      return renderComponentCall(node, state, screen, local);
    case "if":
      return renderIf(node, state, screen, local);
    case "when":
      return null; // zdarzenia obsługiwane osobno
    case "state":
      return null;
    default:
      return null;
  }
}

const LAYOUT_TAGS = { column: "div", row: "div", card: "div", scroll: "div", grid: "div", spacer: "div" };

function renderContainer(node, state, screen, local) {
  const box = el("div", `ax-${node.name}`);
  applyAccessories(box, node, state, screen, local);

  if (node.name === "list") {
    const items = local.get(node.source) || evalPath(node.source, state, local) || [];
    if (!Array.isArray(items) || items.length === 0) {
      const emptyNode = node.children.find(c => c.kind === "container" && c.name === "empty");
      if (emptyNode) {
        for (const c of emptyNode.children) {
          const rendered = renderNode(c, state, screen, local);
          if (rendered) box.appendChild(rendered);
        }
      } else {
        box.appendChild(elText("Brak danych.", "ax-caption"));
      }
      return box;
    }
    const itemNode = node.children.find(c => c.kind === "container" && c.name === "item");
    if (itemNode) {
      for (const entry of items) {
        const itemLocal = new Map(local);
        itemLocal.set(itemVarName(itemNode), entry);
        for (const c of itemNode.children) {
          const rendered = renderNode(c, state, screen, itemLocal);
          if (rendered) box.appendChild(rendered);
        }
      }
    }
    return box;
  }

  for (const child of node.children) {
    // if / else łączymy w pary
    if (child.kind === "if") {
      const ifBranch = evaluateCondition(child.condition, state, screen, local);
      const elseNode = node.children[node.children.indexOf(child) + 1];
      const target = ifBranch ? child : (elseNode && elseNode.kind === "else" ? elseNode : null);
      if (target) {
        for (const c of target.children) {
          const rendered = renderNode(c, state, screen, local);
          if (rendered) box.appendChild(rendered);
        }
      }
      continue;
    }
    if (child.kind === "else") continue; // obsłużone przy if

    const rendered = renderNode(child, state, screen, local);
    if (rendered) box.appendChild(rendered);
  }

  return box;
}

function itemVarName(itemNode) {
  const t = itemNode.children.length ? itemNode : null;
  // "item message" — nazwa zmiennej jest w źródle kontenera
  return itemNode.source || "item";
}

function renderText(node, state, screen, local) {
  const value = interpolate(node.value, state, screen, local);
  const p = elText(value, "ax-text");
  applyAccessories(p, node, state, screen, local);
  return p;
}

function renderButton(node, state, screen, local) {
  const raw = node.value || "";
  const iconMatch = raw.match(/^icon\s+"([^"]+)"/);
  const labelMatch = raw.match(/"([^"]*)"/);

  const b = el("button", "ax-button");
  b.textContent = labelMatch ? labelMatch[1] : (iconMatch ? iconMatch[1] : "Przycisk");
  if (iconMatch) b.classList.add("ax-button-icon");

  applyAccessories(b, node, state, screen, local);

  const when = node.children.find(c => c.kind === "when" && c.event === "tapped");
  if (when) {
    b.addEventListener("click", () => {
      runStatements(when.children, state, screen, local);
    });
  }
  return b;
}

function renderField(node, state, screen, local) {
  const wrap = el("label", "ax-field");
  let labelText = node.name;
  let inputType = "text";
  let placeholder = "";

  for (const c of node.children) {
    if (c.kind === "raw") {
      const m = c.text.match(/^label\s+"(.*)"$/);
      if (m) labelText = m[1];
      const p = c.text.match(/^placeholder\s+"(.*)"$/);
      if (p) placeholder = p[1];
      const t = c.text.match(/^type\s+(\w+)$/);
      if (t) inputType = t[1] === "password" ? "password" : t[1] === "email" ? "email" : "text";
    }
  }

  const span = elText(labelText, "ax-label");
  const input = el("input", "ax-input");
  input.type = inputType;
  if (placeholder) input.placeholder = placeholder;
  input.value = local.get(node.name) || "";
  input.addEventListener("input", () => local.set(node.name, input.value));

  wrap.appendChild(span);
  wrap.appendChild(input);
  return wrap;
}

function renderImage(node, state, screen, local) {
  const alt = node.children.find(c => c.kind === "raw" && c.text.startsWith("alt "));
  const fig = el("div", "ax-image-block");
  const src = interpolate(node.ref, state, screen, local);
  const img = el("img", "ax-image");
  img.alt = alt ? alt.text.slice(4).trim().replace(/^"|"$/g, "") : "Obraz";
  img.src = src && src.startsWith("http") ? src : "";
  fig.appendChild(img);
  if (alt) fig.appendChild(elText(img.alt, "ax-caption"));
  return fig;
}

function renderCheckbox(node, state, screen, local) {
  const wrap = el("label", "ax-checkbox");
  const input = el("input", "ax-check-input");
  input.type = "checkbox";
  input.checked = Boolean(local.get(node.ref));
  input.addEventListener("change", () => local.set(node.ref, input.checked));
  const acc = node.children.find(c => c.kind === "raw" && c.text.startsWith("accessibility label"));
  wrap.appendChild(input);
  wrap.appendChild(elText(acc ? acc.text.slice(19).trim().replace(/^"|"$/g, "") : node.ref, "ax-label"));
  return wrap;
}

function renderMedia(node, state, screen, local) {
  const box = el("div", `ax-${node.name}`);
  box.appendChild(elText(`[${node.name}]`, "ax-caption"));
  return box;
}

function renderComponentCall(node, state, screen, local) {
  const comp = state.components.get(node.name);
  if (!comp) {
    const frag = el("div", "ax-call-missing");
    frag.appendChild(elText(`[${node.name}]`, "ax-caption"));
    return frag;
  }
  const compLocal = new Map(local);
  const args = node.args.split(",").map(a => a.trim()).filter(Boolean);
  comp.params.forEach((p, idx) => compLocal.set(p.name, evalPath(args[idx], state, compLocal)));
  const box = el("div", `ax-component ax-component-${node.name}`);
  for (const child of comp.tree.children) {
    const rendered = renderNode(child, state, screen, compLocal);
    if (rendered) box.appendChild(rendered);
  }
  return box;
}

function renderIf(node, state, screen, local) {
  const pass = evaluateCondition(node.condition, state, screen, local);
  if (!pass) return null;
  const box = el("div", "ax-if");
  for (const c of node.children) {
    const rendered = renderNode(c, state, screen, local);
    if (rendered) box.appendChild(rendered);
  }
  return box;
}

// ---------------------------------------------------------------------------
// Akcesoria: style, padding, gap, dostępność
// ---------------------------------------------------------------------------

function applyAccessories(element, node, state, screen, local) {
  if (!node.children) return;
  for (const c of node.children) {
    if (c.kind !== "raw") continue;
    const text = c.text.trim();

    const styleMatch = text.match(/^style\s+(\S+)$/);
    if (styleMatch) {
      element.classList.add(`ax-s-${styleMatch[1]}`);
      const style = state.styles.get(styleMatch[1]);
      if (style) {
        if (style.background) element.style.background = resolveColor(style.background);
        if (style.color) element.style.color = resolveColor(style.color);
        if (style["font"]) element.style.font = style["font"].split(",")[0];
        if (style["font size"]) element.style.fontSize = style["font size"] + "px";
        if (style["font mono"] !== undefined) element.style.fontFamily = "ui-monospace, monospace";
        if (style.radius) element.style.borderRadius = style.radius + "px";
        if (style.padding) element.style.padding = style.padding.split(/\s+/).join("px ") + "px";
      }
      continue;
    }

    const accMatch = text.match(/^accessibility label\s+"(.*)"$/);
    if (accMatch) {
      element.setAttribute("aria-label", accMatch[1]);
      element.setAttribute("title", accMatch[1]);
      continue;
    }

    const roleMatch = text.match(/^role\s+(\w+)$/);
    if (roleMatch) element.setAttribute("role", roleMatch[1]);
  }

  // właściwości layoutu z surowych linii
  const flat = node.children.filter(c => c.kind === "raw").map(c => c.text.trim());
  const gap = flat.find(t => t.startsWith("gap "));
  if (gap) element.style.gap = gap.split(/\s+/)[1] + "px";

  const pad = flat.find(t => t.startsWith("padding "));
  if (pad) element.style.padding = pad.split(/\s+/).slice(1).join("px ") + "px";

  const cols = flat.find(t => t.startsWith("columns "));
  if (cols) element.style.gridTemplateColumns = `repeat(${cols.split(/\s+/)[1]}, 1fr)`;

  const align = flat.find(t => t.startsWith("align "));
  if (align) element.style.alignItems = align.split(/\s+/)[1] === "center" ? "center" : align.split(/\s+/)[1];

  const maxLines = flat.find(t => t.startsWith("max lines "));
  if (maxLines) {
    const n = Number(maxLines.split(/\s+/)[2]);
    element.style.display = "-webkit-box";
    element.style.webkitLineClamp = n;
    element.style.webkitBoxOrient = "vertical";
    element.style.overflow = "hidden";
  }
}

function resolveColor(value) {
  if (!value) return "transparent";
  if (value.startsWith("#")) return value;
  const m = value.match(/^theme\.(\w+)$/);
  if (m) return `var(--ax-${m[1]})`;
  if (value === "white") return "#ffffff";
  if (value === "transparent") return "transparent";
  return value;
}

// ---------------------------------------------------------------------------
// Ewaluacja: warunki, ścieżki, interpolacja
// ---------------------------------------------------------------------------

function evaluateCondition(condition, state, screen, local) {
  if (!condition) return false;
  const c = condition.trim();

  let m = c.match(/^(.+?)\s+is\s+not\s+(.+)$/);
  if (m) return evalPath(m[1], state, local) !== evalPath(m[2], state, local);

  m = c.match(/^(.+?)\s+is\s+(.+)$/);
  if (m) {
    const left = evalPath(m[1], state, local);
    const right = evalPath(m[2], state, local);
    return left === right;
  }

  m = c.match(/^length\((.+?)\)\s+is\s+(\d+)$/);
  if (m) {
    const val = evalPath(m[1], state, local);
    return Array.isArray(val) ? val.length === Number(m[2]) : false;
  }

  m = c.match(/^length\((.+?)\)\s+>\s+(\d+)$/);
  if (m) {
    const val = evalPath(m[1], state, local);
    return Array.isArray(val) ? val.length > Number(m[2]) : false;
  }

  const val = evalPath(c, state, local);
  return Boolean(val);
}

function evalPath(raw, state, local) {
  if (raw === null || raw === undefined) return null;
  const s = String(raw).trim();
  if (s.length === 0) return null;

  if (s.startsWith('"') && s.endsWith('"')) return s.slice(1, -1);
  if (s === "true") return true;
  if (s === "false") return false;
  if (s === "none") return null;
  if (/^-?[0-9.]+$/.test(s)) return Number(s);

  // globalny stan
  if (state.global.has(s)) return state.global.get(s);

  // stan ekranu
  if (local && local.has(s)) return local.get(s);

  // ścieżka a.b.c
  const parts = s.split(".");
  if (parts.length > 1) {
    let base = local && local.has(parts[0]) ? local.get(parts[0])
      : state.global.has(parts[0]) ? state.global.get(parts[0])
      : null;
    for (let i = 1; i < parts.length && base != null; i++) base = base[parts[i]];
    if (base !== undefined) return base;
  }

  // enum: Nazwa.wartość
  if (parts.length === 2) {
    const enumDef = state.data.get(parts[0]);
    if (enumDef && enumDef.enumValues) return parts[1];
  }

  // wywołanie funkcji zarejestrowanej jako dane
  if (s.endsWith("()")) {
    const fnName = s.slice(0, -2);
    const fn = state.functions.get(fnName);
    if (fn) return fn.name;
  }

  return null;
}

function interpolate(template, state, screen, local) {
  if (!template) return "";
  let out = template;

  // "Tekst " + zmienna + " dalej"
  out = out.replace(/"([^"]*)"/g, (_, inner) => inner);

  const parts = ou
  t.split(/\s*\+\s*/);
  if (parts.length > 1) {
    return parts.map(p => {
      const v = evalPath(p.trim(), state, local);
      return v === null ? p.trim() : String(v);
    }).join("");
  }

  return out.replace(/^"|"$/g, "");
}

function runStatements(children, state, screen, local) {
  for (const c of children) {
    if (c.kind !== "raw") continue;
    const text = c.text;

    const setMatch = text.match(/^set\s+([A-Za-z0-9_.]+)\s*=\s*(.+)$/);
    if (setMatch) {
      const target = setMatch[1];
      const value = evalPath(setMatch[2], state, local);
      if (target.includes(".")) {
        const parts = target.split(".");
        if (state.global.has(parts[0])) {
          const obj = state.global.get(parts[0]);
          if (obj && typeof obj === "object") obj[parts[1]] = value;
          continue;
        }
      }
      if (state.global.has(target)) state.global.set(target, value);
      else if (local) local.set(target, value);
      continue;
    }

    const goMatch = text.match(/^go to\s+([A-Za-z0-9_]+)/);
    if (goMatch) {
      const target = state.screens.get(goMatch[1]);
      if (target) rerender(target, state);
      continue;
    }
  }
}

function rerender(screen, state) {
  const mount = document.querySelector(".ax-screen");
  if (!mount) return;
  const fresh = renderScreen(screen, state);
  mount.replaceWith(fresh);
}

function findWhen(tree, eventName) {
  for (const c of tree.children) {
    if (c.kind === "when" && c.event === eventName) return c;
    if (c.children) {
      const found = findWhen({ children: c.children }, eventName);
      if (found) return found;
    }
  }
  return null;
}

// ---------------------------------------------------------------------------
// Pomocnicze
// ---------------------------------------------------------------------------

function el(tag, className) {
  const node = document.createElement(tag);
  if (className) node.className = className;
  return node;
}

function elText(value, className) {
  const node = el("span", className);
  node.textContent = value;
  return node;
}





// ============================================================================
// WARSTWA 5 — EWALUATOR WYRAŻEŃ
// Pełna ewaluacja: literały, ścieżki, operatory, wywołania funkcji,
// konstruktory typów danych, interpolacja tekstu.
// ============================================================================

function splitTopLevel(text, separator) {
  const parts = [];
  let depth = 0;
  let current = "";
  let inString = false;
  for (let i = 0; i < text.length; i++) {
    const ch = text[i];
    if (ch === '"' && text[i - 1] !== "\\") inString = !inString;
    if (!inString) {
      if (ch === "(" || ch === "[" || ch === "{") depth++;
      if (ch === ")" || ch === "]" || ch === "}") depth--;
      if (ch === separator && depth === 0) { parts.push(current.trim()); current = ""; continue; }
    }
    current += ch;
  }
  if (current.trim().length > 0) parts.push(current.trim());
  return parts;
}

function evalExpression(expr, scope) {
  if (expr === null || expr === undefined) return null;
  let s = String(expr).trim();
  if (s.length === 0) return null;

  // literały
  if (s.startsWith('"') && s.endsWith('"') && s.length > 1) return s.slice(1, -1);
  if (s === "true") return true;
  if (s === "false") return false;
  if (s === "none") return null;
  if (/^-?[0-9]+(\.[0-9]+)?$/.test(s)) return Number(s);

  // operator łączenia tekstu
  const plusParts = splitTopLevel(s, "+");
  if (plusParts.length > 1) {
    let result = "";
    let numeric = true;
    let sum = 0;
    for (const part of plusParts) {
      const v = evalExpression(part, scope);
      if (typeof v === "number" && numeric) { sum += v; }
      else numeric = false;
      result += v === null || v === undefined ? "" : String(v);
    }
    return numeric ? sum : result;
  }

  // konstruktor modelu danych: Nazwa(pole = wartość, ...)
  const ctor = s.match(/^([A-Z][A-Za-z0-9_]*)\(([\s\S]*)\)$/);
  if (ctor) {
    const obj = { __type: ctor[1] };
    for (const pair of splitTopLevel(ctor[2], ",")) {
      const eq = pair.indexOf("=");
      if (eq === -1) continue;
      const key = pair.slice(0, eq).trim();
      obj[key] = evalExpression(pair.slice(eq + 1), scope);
    }
    return obj;
  }

  // wywołanie funkcji wbudowanej
  const call = s.match(/^([a-z][A-Za-z0-9_]*)\(([\s\S]*)\)$/);
  if (call) {
    const name = call[1];
    const args = splitTopLevel(call[2], ",").map(a => evalExpression(a, scope));
    return callBuiltin(name, args, scope);
  }

  // operatory porównania
  let m = s.match(/^(.+?)\s+is\s+not\s+(.+)$/);
  if (m) return evalExpression(m[1], scope) !== evalExpression(m[2], scope);
  m = s.match(/^(.+?)\s+is\s+(.+)$/);
  if (m) return evalExpression(m[1], scope) === evalExpression(m[2], scope);
  m = s.match(/^(.+?)\s*>=\s*(.+)$/);
  if (m) return Number(evalExpression(m[1], scope)) >= Number(evalExpression(m[2], scope));
  m = s.match(/^(.+?)\s*<=\s*(.+)$/);
  if (m) return Number(evalExpression(m[1], scope)) <= Number(evalExpression(m[2], scope));
  m = s.match(/^(.+?)\s*>\s*(.+)$/);
  if (m) return Number(evalExpression(m[1], scope)) > Number(evalExpression(m[2], scope));
  m = s.match(/^(.+?)\s*<\s*(.+)$/);
  if (m) return Number(evalExpression(m[1], scope)) < Number(evalExpression(m[2], scope));

  // ścieżka dostępu
  return resolvePath(s, scope);
}

function resolvePath(path, scope) {
  const parts = path.split(".");
  const base = parts[0];

  let value;
  if (scope.runtime.global.has(base)) value = scope.runtime.global.get(base);
  else if (scope.local.has(base)) value = scope.local.get(base);
  else if (scope.item !== null && scope.item !== undefined && scope.itemVar === base) value = scope.item;
  else if (scope.params && scope.params.has(base)) value = scope.params.get(base);
  else if (scope.enumValues && scope.enumValues.has(parts[0])) return parts[1];
  else return null;

  for (let i = 1; i < parts.length; i++) {
    if (value === null || value === undefined) return null;
    if (Array.isArray(value)) { value = value[Number(parts[i])]; continue; }
    value = value[parts[i]];
  }
  return value === undefined ? null : value;
}

function callBuiltin(name, args, scope) {
  switch (name) {
    case "length": return Array.isArray(args[0]) ? args[0].length : String(args[0] ?? "").length;
    case "round": return Math.round(Number(args[0]) * Math.pow(10, Number(args[1] ?? 0))) / Math.pow(10, Number(args[1] ?? 0));
    case "floor": return Math.floor(Number(args[0]));
    case "ceil": return Math.ceil(Number(args[0]));
    case "abs": return Math.abs(Number(args[0]));
    case "min": return Math.min(...args.map(Number));
    case "max": return Math.max(...args.map(Number));
    case "clamp": return Math.min(Math.max(Number(args[0]), Number(args[1])), Number(args[2]));
    case "substring": return String(args[0] ?? "").slice(Number(args[1]), args[2] === undefined ? undefined : Number(args[2]));
    case "contains": return String(args[0] ?? "").includes(String(args[1] ?? ""));
    case "startsWith": return String(args[0] ?? "").startsWith(String(args[1] ?? ""));
    case "toUpper": return String(args[0] ?? "").toUpperCase();
    case "toLower": return String(args[0] ?? "").toLowerCase();
    case "trim": return String(args[0] ?? "").trim();
    case "newId": return "id_" + Date.now().toString(36) + "_" + Math.random().toString(36).slice(2, 8);
    case "now": return new Date().toISOString();
    case "timestampMs": return Date.now();
    case "randomToken": return Math.random().toString(36).slice(2, 2 + Number(args[0] ?? 12));
    case "join": return Array.isArray(args[0]) ? args[0].join(String(args[1] ?? ", ")) : "";
    case "joinNumbers": return Array.isArray(args[0]) ? args[0].join(String(args[1] ?? ", ")) : "";
    case "splitText": return String(args[0] ?? "").split(String(args[1] ?? " "));
    case "isEmpty": return args[0] === null || args[0] === undefined || args[0] === "" || (Array.isArray(args[0]) && args[0].length === 0);
    case "not": return !args[0];
    default:
      if (scope.runtime.functions.has(name)) return "fn:" + name;
      if (scope.runtime.enums.has?.(name)) return args[0];
      return null;
  }
}

// ============================================================================
// WARSTWA 6 — INTERPRETER LOGIKI
// Wykonuje treść funkcji i zdarzeń: set, add, remove, update, save, load,
// if/else, for, try/catch/finally, return, go to.
// ============================================================================

function createScope(runtime, local, item, itemVar, params) {
  return {
    runtime,
    local: local || new Map(),
    item: item ?? null,
    itemVar: itemVar || null,
    params: params || new Map(),
    locals: new Map(),
  };
}

function executeBlock(tokens, scope) {
  let returned = null;
  for (let i = 0; i < tokens.length; i++) {
    const t = tokens[i];
    if (t.kind === "comment" || t.kind === "blank") continue;

    const indent = t.indent;
    const raw = (t.raw || "").trim();

    // if / else — zbieramy gałęzie na tym samym poziomie
    if (raw.startsWith("if ") || raw === "if") {
      const { body: thenBody, next } = collectSiblings(tokens, i + 1, indent);
      const elseStart = next;
      let elseBody = [];
      let end = next;
      if (elseStart < tokens.length && (tokens[elseStart].raw || "").trim().startsWith("else")) {
        const collected = collectSiblings(tokens, elseStart + 1, tokens[elseStart].indent);
        elseBody = collected.body;
        end = collected.next;
      }
      const condition = raw.slice(2).trim();
      const pass = Boolean(evalExpression(condition, scope));
      const branch = pass ? thenBody : elseBody;
      const inner = executeBlock(branch, scope);
      if (inner && inner.__return) return inner;
      i = end - 1;
      continue;
    }

    if (raw.startsWith("for ")) {
      const { body, next } = collectSiblings(tokens, i + 1, indent);
      const m = raw.match(/^for\s+([A-Za-z0-9_]+)\s+in\s+(.+)$/);
      if (m) {
        const varName = m[1];
        const list = evalExpression(m[2], scope);
        if (Array.isArray(list)) {
          for (const element of list) {
            const nested = scope;
            nested.item = element;
            nested.itemVar = varName;
            executeBlock(body, nested);
          }
        }
      }
      i = next - 1;
      continue;
    }

    if (raw === "try") {
      const { body: tryBody, next } = collectSiblings(tokens, i + 1, indent);
      let catchBody = [];
      let finallyBody = [];
      let end = next;
      while (end < tokens.length) {
        const cand = (tokens[end].raw || "").trim();
        if (cand.startsWith("catch") && tokens[end].indent === indent) {
          const collected = collectSiblings(tokens, end + 1, indent);
          catchBody = collected.body;
          end = collected.next;
          continue;
        }
        if (cand === "finally" && tokens[end].indent === indent) {
          const collected = collectSiblings(tokens, end + 1, indent);
          finallyBody = collected.body;
          end = collected.next;
          continue;
        }
        break;
      }
      try {
        const inner = executeBlock(tryBody, scope);
        if (inner && inner.__return) return inner;
      } catch (err) {
        const nested = scope;
        nested.locals.set("error", { message: err.message });
        executeBlock(catchBody, nested);
      } finally {
        executeBlock(finallyBody, scope);
      }
      i = end - 1;
      continue;
    }

    // set zmienna = wyrażenie
    const setM = raw.match(/^set\s+([A-Za-z0-9_.\[\]]+)\s*=\s*([\s\S]+)$/);
    if (setM) {
      assignPath(setM[1], evalExpression(setM[2], scope), scope);
      continue;
    }

    // add Element(...) to lista
    const addM = raw.match(/^add\s+([\s\S]+?)\s+to\s+([A-Za-z0-9_]+)$/);
    if (addM) {
      const value = evalExpression(addM[1], scope);
      const target = addM[2];
      const list = scope.runtime.global.get(target) ?? scope.local.get(target);
      if (Array.isArray(list)) list.push(value);
      else scope.runtime.global.set(target, [value]);
      continue;
    }

    // remove element from lista
    const remM = raw.match(/^remove\s+([\s\S]+?)\s+from\s+([A-Za-z0-9_]+)$/);
    if (remM) {
      const value = evalExpression(remM[1], scope);
      const target = remM[2];
      const list = scope.runtime.global.get(target) ?? scope.local.get(target);
      if (Array.isArray(list)) {
        const idx = list.indexOf(value);
        if (idx > -1) list.splice(idx, 1);
        else list.shift();
      }
      continue;
    }

    // update element.pole = wartość
    const updM = raw.match(/^update\s+([A-Za-z0-9_.\[\]]+)\s*=\s*([\s\S]+)$/);
    if (updM) {
      assignPath(updM[1], evalExpression(updM[2], scope), scope);
      continue;
    }

    // save / load / clear magazyn
    const storeM = raw.match(/^(save|load|clear)\s+([A-Za-z0-9_]+)$/);
    if (storeM) {
      handleStoreOperation(storeM[1], storeM[2], scope);
      continue;
    }

    // enqueue ... to outbox
    const enqM = raw.match(/^enqueue\s+([\s\S]+?)\s+to\s+([A-Za-z0-9_]+)$/);
    if (enqM) {
      const value = evalExpression(enqM[1], scope);
      const list = scope.runtime.global.get(enqM[2]);
      if (Array.isArray(list)) list.push(value);
      continue;
    }

    // return
    const retM = raw.match(/^return(?:\s+([\s\S]+))?$/);
    if (retM) {
      return { __return: retM[1] === undefined ? null : evalExpression(retM[1], scope) };
    }

    // go to Ekran
    const goM = raw.match(/^go to\s+([A-Za-z0-9_]+)(?:\((.*)\))?$/);
    if (goM) {
      const screen = scope.runtime.screens.get(goM[1]);
      if (screen) navigateTo(screen, scope.runtime, goM[2]);
      continue;
    }

    if (raw === "go back") {
      const first = scope.runtime.screens.values().next().value;
      if (first) navigateTo(first, scope.runtime);
      continue;
    }

    // wywołanie funkcji zadeklarowanej
    const fnM = raw.match(/^([a-zA-Z_][A-Za-z0-9_]*)\(([\s\S]*)\)$/);
    if (fnM && scope.runtime.functions.has(fnM[1])) {
      invokeFunction(fnM[1], fnM[2], scope);
      continue;
    }

    // announce / announceUrgent / vibrate
    const annM = raw.match(/^announce(?:Urgent)?\(([\s\S]*)\)$/);
    if (annM) {
      const text = evalExpression(annM[1], scope);
      announceToUser(text);
      continue;
    }
    const vibM = raw.match(/^vibrate\s+(short|medium|long)$/);
    if (vibM) {
      triggerVibration(vibM[1]);
      continue;
    }
  }
  return returned;
}

function assignPath(target, value, scope) {
  if (target.includes(".")) {
    const parts = target.split(".");
    const root = parts[0];
    let obj = scope.runtime.global.has(root) ? scope.runtime.global.get(root) : scope.local.get(root);
    if (!obj || typeof obj !== "object") return;
    for (let i = 1; i < parts.length - 1; i++) {
      const key = parts[i];
      const idx = key.match(/^([A-Za-z0-9_]+)\[(\d+)\]$/);
      if (idx) { obj = obj[idx[1]]?.[Number(idx[2])]; }
      else obj = obj[key];
      if (!obj) return;
    }
    const last = parts[parts.length - 1];
    const lastIdx = last.match(/^([A-Za-z0-9_]+)\[(\d+)\]$/);
    if (lastIdx) { if (Array.isArray(obj[lastIdx[1]])) obj[lastIdx[1]][Number(lastIdx[2])] = value; }
    else obj[last] = value;
    return;
  }
  if (scope.runtime.global.has(target)) scope.runtime.global.set(target, value);
  else if (scope.params && scope.params.has(target)) scope.params.set(target, value);
  else scope.local.set(target, value);
}

function handleStoreOperation(op, storeName, scope) {
  const store = scope.runtime.stores.get(storeName);
  if (!store) {
    scope.runtime.diagnostics.push("Operacja " + op + " na niezadeklarowanym magazynie: " + storeName);
    return;
  }
  if (op === "save") {
    const data = scope.runtime.global.get(storeName) ?? scope.local.get(storeName);
    scope.runtime.persistence.set(storeName, data === undefined ? null : JSON.parse(JSON.stringify(data)));
    scope.runtime.storeWrites.push({ store: storeName, at: Date.now(), size: Array.isArray(data) ? data.length : 1 });
  } else if (op === "load") {
    const persisted = scope.runtime.persistence.get(storeName);
    if (persisted !== undefined && Array.isArray(persisted)) scope.runtime.global.set(storeName, persisted);
  } else if (op === "clear") {
    scope.runtime.persistence.delete(storeName);
    scope.runtime.global.set(storeName, []);
  }
}

function invokeFunction(name, argText, scope) {
  const fn = scope.runtime.functions.get(name);
  if (!fn) return null;
  const args = splitTopLevel(argText || "", ",").map(a => evalExpression(a, scope));
  const params = new Map();
  fn.params.forEach((p, i) => params.set(p.name, args[i] === undefined ? null : args[i]));
  const nested = createScope(scope.runtime, scope.local, scope.item, scope.itemVar, params);
  const result = executeBlock(fn.body, nested);
  return result && result.__return !== undefined ? result.__return : null;
}

function announceToUser(text) {
  if (typeof document === "undefined") return;
  let region = document.getElementById("ax-announce");
  if (!region) {
    region = document.createElement("div");
    region.id = "ax-announce";
    region.setAttribute("role", "status");
    region.setAttribute("aria-live", "polite");
    region.style.cssText = "position:fixed;left:-9999px;width:1px;height:1px;overflow:hidden";
    document.body.appendChild(region);
  }
  region.textContent = String(text ?? "");
}

function triggerVibration(pattern) {
  if (typeof navigator === "undefined" || !navigator.vibrate) return;
  const map = { short: 40, medium: 120, long: 300 };
  navigator.vibrate(map[pattern] || 60);
}

function navigateTo(screen, runtime, argText) {
  if (typeof document === "undefined") return;
  runtime.currentScreen = screen.name;
  const mount = document.querySelector(".ax-screen");
  if (!mount) return;
  const fresh = renderScreen(screen, runtime, argText);
  mount.replaceWith(fresh);
}

function collectSiblings(tokens, start, parentIndent) {
  const body = [];
  let i = start;
  while (i < tokens.length) {
    const t = tokens[i];
    if (t.kind === "blank" || t.kind === "comment") { i++; continue; }
    if (t.indent <= parentIndent) break;
    body.push(t);
    i++;
  }
  return { body, next: i };
}

// ============================================================================
// WARSTWA 7 — WARSTWA API Z MOCOWANIEM NA ŻĄDANIA
// Każdy endpoint z deklaracji jest dostępny jako funkcja. Obsługa błędu,
// limitu czasu i ponawiania jest częścią wywołania.
// ============================================================================

function createApiLayer(runtime) {
  const layer = {};
  for (const api of runtime.apis.values()) {
    const apiObject = {};
    apiObject.__config = api.config;
    apiObject.__base = (api.config.base || "").replace(/^"|"$/g, "");

    for (const endpoint of api.endpoints) {
      apiObject[endpoint.name] = async (...args) => {
        const timeout = Number(api.config.timeout || 20000);
        const retries = Number(api.config.retry || 0);
        let lastError = null;

        for (let attempt = 0; attempt <= retries; attempt++) {
          const controller = typeof AbortController !== "undefined" ? new AbortController() : null;
          const timer = controller ? setTimeout(() => controller.abort(), timeout) : null;
          try {
            const method = detectHttpMethod(endpoint);
            const path = detectHttpPath(endpoint);
            const url = apiObject.__base + path;
            const call = { method, url, attempt, args };
            runtime.apiCalls.push(call);

            if (typeof fetch === "function" && /^https?:/.test(url)) {
              const headers = { "content-type": "application/json" };
              if ((api.config.auth || "").includes("bearer")) headers.authorization = "Bearer <token>";
              const response = await 
                fetch(url, {
                method,
                headers,
                signal: controller ? controller.signal : undefined,
              });
              if (timer) clearTimeout(timer);
              if (!response.ok) throw new Error("HTTP " + response.status);
              runtime.apiResults.push({ url, status: response.status, at: Date.now() });
              return await response.json();
            }

            if (timer) clearTimeout(timer);
            // Brak środowiska sieciowego: zwracamy dane próbne oznaczone jako nieprawdziwe.
            return { __simulated: true, note: "Wywołanie API zapisane, brak środowiska sieciowego.", call };
          } catch (err) {
            if (timer) clearTimeout(timer);
            lastError = err;
          }
        }
        runtime.apiErrors.push({ api: api.name, endpoint: endpoint.name, message: lastError ? lastError.message : "nieznany" });
        throw lastError || new Error("Wywołanie nie powiodło się");
      };
    }
    layer[api.name] = apiObject;
    runtime.apiObjects.set(api.name, apiObject);
  }
  return layer;
}

function detectHttpMethod(endpoint) {
  const methods = { get: "GET", post: "POST", put: "PUT", patch: "PATCH", delete: "DELETE" };
  for (const t of endpoint.body) {
    const first = (t.raw || "").trim().split(/\s+/)[0].toLowerCase();
    if (methods[first]) return methods[first];
  }
  return "GET";
}

function detectHttpPath(endpoint) {
  for (const t of endpoint.body) {
    const raw = (t.raw || "").trim();
    const m = raw.match(/^(get|post|put|patch|delete)\s+("([^"]*)"|\/.*)$/);
    if (m) {
      const part = m[2].trim();
      return part.startsWith('"') ? part.replace(/^"|"$/g, "") : part;
    }
  }
  return "/" + endpoint.name;
}

// ============================================================================
// WARSTWA 8 — UPRAWNIENIA I PREFERENCJE DOSTĘPNOŚCI
// ============================================================================

function createPermissionLayer(runtime) {
  return {
    async request(name) {
      const known = {
        camera: "camera",
        microphone: "microphone",
        photos: "geolocation",
        location: "geolocation",
        notifications: "notifications",
        vibration: "vibrate",
      };
      const browserName = known[name];
      if (!browserName) return "denied";
      if (typeof navigator !== "undefined" && navigator.permissions) {
        try {
          const status = await navigator.permissions.query({ name: browserName });
          runtime.permissions.set(name, status.state);
          return status.state;
        } catch (e) {
          runtime.permissions.set(name, "unknown");
          return "unknown";
        }
      }
      return "unknown";
    },
    declared() {
      return runtime.permissionDecls;
    },
  };
}

function readAccessibilityPreferences() {
  const prefs = { reduceMotion: false, highContrast: false, textScale: 1 };
  if (typeof window === "undefined" || !window.matchMedia) return prefs;
  prefs.reduceMotion = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  prefs.highContrast = window.matchMedia("(prefers-contrast: more)").matches;
  return prefs;
}

// ============================================================================
// WARSTWA 9 — PAMIĘĆ TRWAŁA SESJI
// Zapisuje stan magazynów między odświeżeniami strony w pamięci przeglądarki.
// ============================================================================

function createPersistence(namespace) {
  const key = "axcode:" + (namespace || "default");
  let memory = new Map();
  if (typeof localStorage !== "undefined") {
    try {
      const raw = localStorage.getItem(key);
      if (raw) memory = new Map(Object.entries(JSON.parse(raw)));
    } catch (e) { memory = new Map(); }
  }
  return {
    get(name) { return memory.get(name); },
    set(name, value) {
      memory.set(name, value);
      if (typeof localStorage === "undefined") return;
      try { localStorage.setItem(key, JSON.stringify(Object.fromEntries(memory))); } catch (e) { /* limit pamięci */ }
    },
    delete(name) {
      memory.delete(name);
      if (typeof localStorage === "undefined") return;
      try { localStorage.setItem(key, JSON.stringify(Object.fromEntries(memory))); } catch (e) { /* limit pamięci */ }
    },
    all() { return Object.fromEntries(memory); },
  };
}

// ============================================================================
// WARSTWA 10 — TESTY
// Wykonuje test, ui test i accessibility test zadeklarowane w pliku .ax.
// ============================================================================

function runTests(ast) {
  const runtime = createRuntime(ast);
  const results = [];

  for (const test of ast.tests) {
    if (test.kind === "test") {
      results.push(runUnitTest(test));
    } else if (test.kind === "ui test") {
      results.push({ kind: "ui", name: test.name, passed: null, note: "Test interfejsu wymaga środowiska przeglądarki." });
    } else {
      const findings = verify(ast).filter(f => f.code.startsWith("AX7"));
      results.push({
        kind: "accessibility",
        name: test.name,
        passed: findings.length === 0,
        findings,
      });
    }
  }
  return results;
}

function runUnitTest(test) {
  const runtime = createRuntime({ ...test, screens: [], components: [], functions: [], data: [], enums: [], stores: [], globals: [], apis: [], styles: [], tests: [] });
  const scope = createScope(runtime, new Map(), null, null, new Map());

  let given = null;
  let when = null;
  let expected = null;

  for (const t of test.body) {
    const raw = (t.raw || "").trim();
    const gm = raw.match(/^given\s+(?:value\s+|text\s+)?([\s\S]+)$/);
    if (gm) { given = gm[1]; continue; }
    const wm = raw.match(/^when\s+([\s\S]+)$/);
    if (wm) { when = wm[1]; continue; }
    const tm = raw.match(/^then\s+([\s\S]+)$/);
    if (tm) { expected = tm[1]; continue; }
  }

  const actual = when ? evalExpression(when, scope) : null;

  let passed = null;
  if (expected) {
    const em = expected.match(/^result\s+is\s+(.+)$/);
    if (em) passed = String(actual) === String(evalExpression(em[1], scope));
  }

  return { kind: "unit", name: test.name, given, when, actual, expected, passed };
}

// ---------------------------------------------------------------------------
// INTERFEJS PUBLICZNY
// ---------------------------------------------------------------------------
export {
  lex, parse, verify, renderApp, createRuntime,
  evalExpression, executeBlock, invokeFunction,
  createApiLayer, createPermissionLayer, createPersistence,
  runTests, readAccessibilityPreferences,
};

export const AXCODE_VERSION = "1.0.0";
export const AXCODE_AUTHOR = "Axel G";
export const AXCODE_INSTAGRAM = "https://www.instagram.com/axelgadomski";

// ---------------------------------------------------------------------------
// WBUDOWANE ŹRÓDŁO STRONY — działa też przy otwarciu pliku bez serwera
// ---------------------------------------------------------------------------
export const SITE_SOURCE = "app \"AXCode — strona o języku\"\nversion \"1.0.0\"\nlanguage pl\nsupports ios android web\norientation automatic\nentry Start\n\ntheme default\n  follows system\n\n  color primary #7C3AED\n  color accent #06B6D4\n  color danger #DC2626\n  color success #16A34A\n  color warning #D97706\n\n  color background light #FAFAFA\n  color background dark #09090B\n  color surface light #FFFFFF\n  color surface dark #18181B\n  color text light #09090B\n  color text dark #FAFAFA\n  color muted light #52525B\n  color muted dark #A1A1AA\n  color border light #E4E4E7\n  color border dark #27272A\n\n  radius small 10\n  radius medium 16\n  radius large 24\n  radius full 999\n\n  font body \"Inter\"\n  font mono \"JetBrains Mono\"\n\nstyle hero\n  font size 46\n  weight bold\n  line height 1.05\n  letter spacing -1\n\nstyle title\n  font size 30\n  weight bold\n  line height 1.15\n  letter spacing -0.4\n\nstyle heading\n  font size 20\n  weight semibold\n\nstyle subheading\n  font size 17\n  weight semibold\n\nstyle body\n  font size 16\n  weight regular\n  line height 1.5\n\nstyle caption\n  font size 13\n  weight regular\n  color theme.muted\n\nstyle codeBlock\n  background theme.surface\n  radius medium\n  padding 18\n  font mono\n  font size 13\n\nstyle buttonPrimary\n  background theme.primary\n  color white\n  radius medium\n  padding 16 22\n\nstyle buttonSecondary\n  background theme.surface\n  color theme.text\n  radius medium\n  padding 16 22\n\nstyle card\n  background theme.surface\n  radius large\n  padding 24\n\nstyle pill\n  background theme.surface\n  radius full\n  padding 8 14\n\nstyle badgeAccent\n  background theme.accent\n  color white\n  radius full\n  padding 4 10\n\ndata Feature\n  id Text\n  title Text\n  detail Text\n\ndata LegendRow\n  id Text\n  kind Text\n  purpose Text\n\ndata CodeSample\n  id Text\n  label Text\n  body Text\n\nenum UnitKind\n  structural\n  behavioural\n\nenum StageKind\n  done\n  planned\n\nstore content local\n  list Feature\n  key \"axcode.site.features.v1\"\n\nstore legends local\n  list LegendRow\n  key \"axcode.site.legends.v1\"\n\nstore samples local\n  list CodeSample\n  key \"axcode.site.samples.v1\"\n\nglobal state currentSection Text = \"start\"\nglobal state expandedSampleId Text = \"\"\n\nfunction loadContent()\n  set features = [\n    Feature(id = \"f1\", title = \"Jeden ekran to jeden blok\", detail = \"Ekran zawiera własny stan, layout, zdarzenia, nawigację i efekty. Nie trzeba skakać między plikiem widoku, stylów, stanu i logiki.\"),\n    Feature(id = \"f2\", title = \"Reaktywność bez glue code'u\", detail = \"Zmiana stanu automatycznie aktualizuje interfejs. Programista nie pisze ręcznej synchronizacji DOM, listenerów ani cyklu życia komponentu.\"),\n    Feature(id = \"f3\", title = \"Bezpieczeństwo w modelu danych\", detail = \"Klucze, tokeny i dane wrażliwe mają jawną klasę przechowywania. Nie można przypadkiem zapisać sekretu w niezabezpieczonym magazynie.\"),\n    Feature(id = \"f4\", title = \"Dostępność w semantyce\", detail = \"Kompilator wymaga tekstu alternatywnego dla obrazu, etykiety dla pola i etykiety dla przycisku ikonowego. Brak to błąd, nie ostrzeżenie.\"),\n    Feature(id = \"f5\", title = \"Jeden front-end, wiele backendów\", detail = \"Kompilator tworzy platformowo-neutralny graf aplikacji AXIR, a dopiero potem generuje kod dla iOS, Androida i sieci.\"),\n    Feature(id = \"f6\", title = \"Błędy przed uruchomieniem\", detail = \"Nieznany ekran, zły typ pola, brak etykiety dostępności albo sekret w złym magazynie są wykrywane w czasie kompilacji, nie w produkcji.\")\n  ]\n  save features\n\n  set legends = [\n    LegendRow(id = \"l1\", kind = \"structural\", purpose = \"app, screen, component, data, enum, store, theme, api, test\"),\n    LegendRow(id = \"l2\", kind = \"behavioural\", purpose = \"state, global state, when tapped, when screen appears, go to, save\"),\n    LegendRow(id = \"l3\", kind = \"behavioural\", purpose = \"list z item i empty, formularze z required i min length\"),\n    LegendRow(id = \"l4\", kind = \"structural\", purpose = \"style, akcesoria dostępności, permission, animate\")\n  ]\n  save legends\n\n  set samples = [\n    CodeSample(id = \"s1\", label = \"Ekran z listą i pustym stanem\", body = \"screen Chat\\n  title \\\"Rozmowa\\\"\\n\\n  state draft Text = \\\"\\\"\\n\\n  column\\n    list messages\\n      empty\\n        text \\\"Brak wiadomości\\\"\\n      item message\\n        messageBubble(message)\"),\n    CodeSample(id = \"s2\", label = \"Magazyn szyfrowany i kolejka offline\", body = \"store outbox local encrypted\\n  list OutboxItem\\n  key \\\"app.outbox.v1\\\"\\n\\nfunction enqueue(body Text)\\n  add OutboxItem(\\n    id = newId(),\\n    idempotencyKey = newId(),\\n    body = body,\\n    attempts = 0,\\n    status = pending\\n  ) to outbox\\n  save outbox\\n  flushOutbox()\"),\n    CodeSample(id = \"s3\", label = \"Dostępność wymuszona przez kompilator\", body = \"button icon \\\"mic\\\"\\n  accessibility label \\\"Nagraj wiadomość głosową\\\"\\n  when tapped\\n    startVoiceInput()\\n\\nimage user.avatarUrl\\n  alt \\\"Zdjęcie profilowe\\\"\\n\\nfield email\\n  label \\\"E-mail\\\"\\n  type email\\n  required\"),\n    CodeSample(id = \"s4\", label = \"API z limitem czasu i ponawianiem\", body = \"api CloudflareApi\\n  base \\\"https://api.example.com\\\"\\n  auth bearer\\n  timeout 20000\\n  retry 3\\n\\n  function login(email Text, password Text) returns Json\\n    post \\\"/auth/login\\\"\\n    body { email, password }\")\n  ]\n  save samples\n\nfunction toggleSample(id Text)\n  if expandedSampleId is id\n    set expandedSampleId = \"\"\n  else\n    set expandedSampleId = id\n\nfunction featureList() returns List<Feature>\n  return features\n\nfunction legendList() returns List<LegendRow>\n  return legends\n\nfunction sampleList() returns List<CodeSample>\n  return samples\n\ncomponent FeatureCard(feature Feature)\n  card\n    column\n      gap 8\n      text feature.title\n        style subheading\n      text feature.detail\n        style body\n        max lines 5\n\ncomponent LegendRowView(row LegendRow)\n  row\n    gap 12\n    align start\n    justify spaceBetween\n    text row.kind\n      style badgeAccent\n    text row.purpose\n      style caption\n      max lines 3\n\ncomponent CodeSampleView(sample CodeSample)\n  card\n    column\n      gap 12\n      text sample.label\n        style subheading\n      if expandedSampleId is sample.id\n        text sample.body\n          style codeBlock\n          max lines 30\n        button \"Zwiń\"\n          style buttonSecondary\n          accessibility label \"Zwiń przykład kodu\"\n          when tapped\n            toggleSample(sample.id)\n      else\n        button \"Pokaż kod\"\n          style buttonSecondary\n          accessibility label \"Pokaż przykład kodu\"\n          when tapped\n            toggleSample(sample.id)\n\ncomponent StageChip(label Text, state StageKind)\n  row\n    gap 8\n    align center\n    text label\n      style body\n    if state is StageKind.done\n      text \"gotowe\"\n        style badgeAccent\n    else\n      text \"w planie\"\n        style pill\n\nscreen Start\n  title \"AXCode\"\n\n  when screen appears\n    load content\n    announceLocal(\"Strona o języku AXCode.\")\n\n  scroll\n    direction vertical\n\n    column\n      padding 24\n      gap 24\n\n      text \"axcode\"\n        style badgeAccent\n\n      text \"Jeden plik. Cała aplikacja.\"\n        style hero\n\n      text \"AXCode to deklaratywny język aplikacji. Jeden plik opisuje interfejs, stan, dane, nawigację, bezpieczeństwo, dostępność, tryb offline i testy.\"\n        style body\n        max lines 4\n\n      row\n        gap 12\n        align center\n        button \"Zobacz możliwości\"\n          style buttonPrimary\n          accessibility label \"Przejdź do sekcji możliwości\"\n          when tapped\n            set currentSection = \"mozliwosci\"\n        button \"Przykłady kodu\"\n          style buttonSecondary\n          accessibility label \"Przejdź do przykładów kodu\"\n          when tapped\n            set currentSection = \"przyklady\"\n\n      row\n        gap 20\n        align center\n        column\n          gap 4\n          text \"1\"\n            style title\n          text \"plik .ax definiuje całą aplikację\"\n            style caption\n            max lines 2\n        column\n          gap 4\n          text \"3\"\n            style title\n          text \"platformy: iOS, Android, Web\"\n            style caption\n            max lines 2\n        column\n          gap 4\n          text \"40-60%\"\n            style title\n          text \"realistyczna redukcja kodu\"\n            style caption\n            max lines 2\n\n      text \"Czym AXCode jest\"\n        style title\n\n      text \"To język warstwy aplikacji: ekranów, stanu, danych, nawigacji, uprawnień, dostępności i testów. Kod natywny pozostaje dostępny przez most, ale nie jest wymagany do zbudowania typowego produktu.\"\n        style body\n        max lines 4\n\n      grid\n        columns 2\n        gap 16\n\n        list featureList()\n          empty\n            text \"Brak treści.\"\n              style caption\n          item feature\n            FeatureCard(feature)\n\n      text \"Jednostki języka\"\n        style title\n\n      list legendList()\n        empty\n          text \"Brak treści.\"\n            style caption\n        item row\n          LegendRowView(row)\n\n      text \"Architektura kompilatora\"\n        style title\n\n      column\n        gap 10\n        StageChip(\"Lexer\", StageKind.done)\n        StageChip(\"Parser\", StageKind.done)\n        StageChip(\"Type Checker\", StageKind.done)\n        StageChip(\"AXIR\", StageKind.done)\n        StageChip(\"Generator dla Web\", StageKind.done)\n        StageChip(\"Generator dla iOS\", StageKind.planned)\n        StageChip(\"Generator dla Android\", StageKind.planned)\n\n      text \"Przykłady kodu\"\n        style title\n\n      column\n        gap 16\n        list sampleList()\n          empty\n            text \"Brak przykładów.\"\n              style caption\n          item sample\n            CodeSampleView(sample)\n\n      text \"Czego AXCode nie robi\"\n        style title\n\n      column\n        gap 8\n        text \"Nie jest językiem systemowym i nie zastępuje Swifta, Kotlina, Rusta ani C++.\"\n          style body\n        text \"Nie odsłania bezpośrednio DOM, UIKit ani Jetpack Compose.\"\n          style body\n        text \"Nie pozwala na dowolne makra ani dynamiczną ewaluację kodu.\"\n          style body\n        text \"Nie gwarantuje identycznego wyglądu piksel w piksel na każdej platformie.\"\n          style body\n        text \"Nie zastępuje backendu i nie wysyła automatycznie wyników AI bez potwierdzenia.\"\n          style body\n\n      text \"Zasady wersjonowania\"\n        style title\n\n      column\n        gap 8\n        text \"1.x — stabilny rdzeń języka.\"\n          style body\n        text \"2.x — rozszerzenia platformowe i zaawansowane animacje.\"\n          style body\n        text \"3.x — edytor wizualny, wtyczki i asystent AI.\"\n          style body\n\n      text \"AXCode — jeden plik, cała aplikacja.\"\n        style subheading\n\n      button \"Od początku\"\n        style buttonSecondary\n        accessibility label \"Przewiń do początku strony\"\n        when tapped\n          set currentSection = \"start\"\n          announceLocal(\"Początek strony.\")\n\ntest \"Lista funkcji ma sześć pozycji\"\n  given value loadContent()\n  when length(featureList())\n  then result is 6\n\ntest \"Przełączanie przykładu rozwija i zwija\"\n  given value toggleSample(\"s1\")\n  when expandedSampleId\n  then result is \"s1\"\n\naccessibility test \"Strona o AXCode\"\n  open Start\n  expect all fields have labels\n  expect all images have alt\n  expect all icon buttons have accessibility label\n  expect contrast passes AA\n";
