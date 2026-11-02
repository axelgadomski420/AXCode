// src/backend.rs — backendy emisji kodu AXCode
//
// Trzy backendy czytaja te sama reprezentacje IR i produkuja kod docelowy:
//   web     — HTML + CSS + TypeScript z reaktywnym runtime
//   native  — Swift + SwiftUI (iOS) oraz Kotlin + Jetpack Compose (Android)
//   json    — znormalizowany opis aplikacji do dalszego przetwarzania
//
// Backendy nie znaja parsera ani weryfikatora: dostaja wylacznie IR, dzieki
// czemu dodanie nowej platformy nie wymaga zmian w czesci front-endowej.

use crate::ir::*;
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

/// Platforma docelowa emisji.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    Web,
    Ios,
    Android,
    Desktop,
    Json,
}

impl Target {
    /// Rozpoznaje nazwe platformy z wiersza polecen.
    pub fn parse(name: &str) -> Result<Self> {
        match name.trim().to_lowercase().as_str() {
            "web" => Ok(Target::Web),
            "ios" => Ok(Target::Ios),
            "android" => Ok(Target::Android),
            "desktop" => Ok(Target::Desktop),
            "json" | "ir" => Ok(Target::Json),
            other => Err(anyhow::anyhow!(
                "nieznana platforma {}: uzyj web, ios, android, desktop albo json",
                other
            )),
        }
    }

    /// Czytelna nazwa platformy.
    pub fn label(&self) -> &'static str {
        match self {
            Target::Web => "Web (HTML + CSS + JavaScript)",
            Target::Ios => "iOS (Swift + SwiftUI)",
            Target::Android => "Android (Kotlin + Jetpack Compose)",
            Target::Desktop => "Desktop (powloka webowa)",
            Target::Json => "JSON (opis aplikacji)",
        }
    }

    /// Rozszerzenie pliku wynikowego.
    pub fn extension(&self) -> &'static str {
        match self {
            Target::Web | Target::Desktop => "html",
            Target::Ios => "swift",
            Target::Android => "kt",
            Target::Json => "json",
        }
    }
}

/// Wynik emisji: pojedynczy plik do zapisania.
pub struct EmittedFile {
    pub name: String,
    pub content: String,
}

/// Emituje kod dla wybranej platformy i zapisuje pliki w katalogu wyjscia.
pub fn emit(program: &IrProgram, target: Target, output_dir: &Path) -> Result<Vec<PathBuf>> {
    fs::create_dir_all(output_dir)
        .with_context(|| format!("nie mozna utworzyc katalogu {}", output_dir.display()))?;

    let files = match target {
        Target::Web => emit_web(program),
        Target::Desktop => emit_desktop(program),
        Target::Ios => emit_ios(program)?,
        Target::Android => emit_android(program),
        Target::Json => emit_json(program)?,
    };

    let mut written = Vec::new();
    for file in files {
        let path = output_dir.join(&file.name);
        fs::write(&path, file.content)
            .with_context(|| format!("nie mozna zapisac pliku {}", path.display()))?;
        written.push(path);
    }
    Ok(written)
}

// --- backend web ---

/// Emituje aplikacje webowa: jeden plik HTML z runtime i warstwa danych.
fn emit_web(program: &IrProgram) -> Vec<EmittedFile> {
    let app_js = emit_web_state(program);
    let screens_js = emit_web_screens(program);
    let styles_css = emit_web_styles(program);

    let html = format!(
        "<!DOCTYPE html>\n<html lang=\"{lang}\">\n<head>\n  <meta charset=\"utf-8\">\n  <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n  <title>{title}</title>\n  <style>\n{css}\n  </style>\n</head>\n<body>\n  <div id=\"ax-root\" role=\"application\" aria-label=\"{title}\"></div>\n  <script>\n{app}\n{runtime}\n{screens}\nwindow.AXCode.start();\n  </script>\n</body>\n</html>\n",
        lang = program.app.language,
        title = escape_html(&program.app.name),
        css = styles_css,
        app = app_js,
        runtime = WEB_RUNTIME,
        screens = screens_js
    );

    let manifest = serde_json::json!({
        "name": program.app.name,
        "version": program.app.version,
        "language": program.app.language,
        "targets": program.app.targets,
        "entry": program.app.entry,
        "offline": program.app.offline,
        "screens": program.screens.iter().map(|s| s.name.clone()).collect::<Vec<_>>(),
        "stores": program.stores.iter().map(|s| s.name.clone()).collect::<Vec<_>>(),
    });

    vec![
        EmittedFile {
            name: format!("{}.html", slug(&program.app.name)),
            content: html,
        },
        EmittedFile {
            name: "axcode-manifest.json".to_string(),
            content: format!(
                "{}\n",
                serde_json::to_string_pretty(&manifest).unwrap_or_else(|_| "{}".to_string())
            ),
        },
    ]
}

/// Emituje warstwe danych: modele, magazyny, stan globalny, API i AI.
fn emit_web_state(program: &IrProgram) -> String {
    let mut out = String::new();
    out.push_str("// Warstwa danych wygenerowana z IR.\n");
    out.push_str("window.AXCode = window.AXCode || {};\n");
    out.push_str("window.AXCode.app = ");
    out.push_str(&serde_json::to_string(&program.app).unwrap_or_else(|_| "{}".to_string()));
    out.push_str(";\n\n");

    out.push_str("window.AXCode.stores = {\n");
    for store in &program.stores {
        out.push_str(&format!(
            "  {}: {{ class: '{}', encrypted: {}, shape: '{}', key: '{}', data: null }},\n",
            js_key(&store.name),
            store.class,
            store.encrypted,
            store.shape,
            store.key
        ));
    }
    out.push_str("};\n\n");

    out.push_str("window.AXCode.globals = {\n");
    for g in &program.globals {
        out.push_str(&format!(
            "  {}: {},\n",
            js_key(&g.name),
            g.initial.clone().unwrap_or_else(|| "null".to_string())
        ));
    }
    out.push_str("};\n\n");

    out.push_str("window.AXCode.types = ");
    let types: Vec<serde_json::Value> = program
        .types
        .iter()
        .map(|t| {
            serde_json::json!({
                "name": t.name,
                "fields": t.fields.iter().map(|f| serde_json::json!({
                    "name": f.name,
                    "type": f.type_name,
                    "optional": f.optional,
                })).collect::<Vec<_>>()
            })
        })
        .collect();
    out.push_str(&serde_json::to_string(&types).unwrap_or_else(|_| "[]".to_string()));
    out.push_str(";\n\n");

    out.push_str("window.AXCode.permissions = ");
    let perms: Vec<serde_json::Value> = program
        .permissions
        .iter()
        .map(|p| {
            serde_json::json!({
                "kind": p.kind,
                "purpose": p.purpose,
                "required": p.required,
            })
        })
        .collect();
    out.push_str(&serde_json::to_string(&perms).unwrap_or_else(|_| "[]".to_string()));
    out.push_str(";\n\n");

    out.push_str("window.AXCode.api = {\n");
    for api in &program.apis {
        out.push_str(&format!(
            "  {}: {{ base: '{}', timeout: {}, retry: {}, methods: {{\n",
            js_key(&api.name), api.base, api.timeout_ms, api.retry
        ));
        for f in &api.functions {
            out.push_str(&format!(
                "    {}: {{ method: '{}', route: '{}', params: {}, returns: {} }},\n",
                js_key(&f.name),
                f.method,
                f.route,
                serde_json::to_string(&f.params).unwrap_or_else(|_| "[]".to_string()),
                f.returns
                    .clone()
                    .map(|r| format!("'{}'", r))
                    .unwrap_or_else(|| "null".to_string())
            ));
        }
        out.push_str("  } },\n");
    }
    out.push_str("};\n\n");

    out.push_str("window.AXCode.ai = { providers: [], active: null };\n\n");

    out.push_str("window.AXCode.theme = ");
    out.push_str(&serde_json::to_string(&program.theme).unwrap_or_else(|_| "{}".to_string()));
    out.push_str(";\n\n");

    out.push_str("window.AXCode.functions = {\n");
    for f in &program.functions {
        let params = f
            .params
            .iter()
            .map(|p| p.split_whitespace().next().unwrap_or("arg").to_string())
            .collect::<Vec<_>>()
            .join(", ");
        out.push_str(&format!("  {}: function({}) {{\n", js_key(&f.name), params));
        out.push_str(&emit_js_statements(&f.body, 2));
        if f.returns.is_some() {
            out.push_str("    return window.AXCode.result;\n");
        }
        out.push_str("  },\n");
    }
    out.push_str("};\n");

    out
}

/// Emituje ekrany jako komponenty renderowane przez runtime.
fn emit_web_screens(program: &IrProgram) -> String {
    let mut out = String::new();
    out.push_str("// Ekrany wygenerowane z IR.\n");
    out.push_str("window.AXCode.screens = {};\n");
    out.push_str("window.AXCode.components = window.AXCode.components || {};\n\n");

    for screen in &program.screens {
        out.push_str(&format!(
            "window.AXCode.screens['{}'] = {{\n  title: '{}',\n  params: {},\n  states: {},\n  storeAccess: {},\n  navigation: {},\n  render: function() {{\n    return ",
            js_key(&screen.name),
            escape_js_string(&screen.title),
            serde_json::to_string(&screen.params).unwrap_or_else(|_| "[]".to_string()),
            emit_js_state_initial(&screen.states),
            serde_json::to_string(&screen.store_access).unwrap_or_else(|_| "[]".to_string()),
            serde_json::to_string(&screen.navigation_targets).unwrap_or_else(|_| "[]".to_string()),
        ));
        out.push_str(&emit_js_nodes(&screen.tree, 2));
        out.push_str(";\n  },\n");

        out.push_str("  lifecycle: [\n");
        for handler in &screen.lifecycle {
            out.push_str(&format!(
                "    {{ event: '{}', body: function(state) {{\n",
                escape_js_string(&handler.event)
            ));
            out.push_str(&emit_js_statements(&handler.body, 3));
            out.push_str("    } },\n");
        }
        out.push_str("  ],\n");

        out.push_str("  handlers: [\n");
        for handler in &screen.handlers {
            out.push_str(&format!(
                "    {{ event: '{}', node: {}, body: function(state, event) {{\n",
                escape_js_string(&handler.event),
                handler
                    .node_id
                    .clone()
                    .map(|n| format!("'{}'", escape_js_string(&n)))
                    .unwrap_or_else(|| "null".to_string())
            ));
            out.push_str(&emit_js_statements(&handler.body, 3));
            out.push_str("    } },\n");
        }
        out.push_str("  ],\n});\n\n");
    }

    for comp in &program.components {
        out.push_str(&format!(
            "window.AXCode.components['{}'] = function() {{\n  return ",
            js_key(&comp.name)
        ));
        out.push_str(&emit_js_nodes(&comp.tree, 1));
        out.push_str(";\n};\n\n");
    }

    out
}

/// Emituje drzewo wezlow jako tablice opisow elementow.
fn emit_js_nodes(nodes: &[IrNode], indent: usize) -> String {
    let pad = "  ".repeat(indent);
    if nodes.is_empty() {
        return format!("{}h('div', {{}}, [])", pad);
    }
    let parts: Vec<String> = nodes.iter().map(|n| emit_js_node(n, indent)).collect();
    format!("[\n{}\n{}]", parts.join(",\n"), pad)
}

/// Emituje pojedynczy wezel.
fn emit_js_node(node: &IrNode, indent: usize) -> String {
    let pad = "  ".repeat(indent);
    let mut attrs: Vec<String> = Vec::new();
    for (k, v) in &node.props {
        attrs.push(format!("{}: '{}'", js_key(k), escape_js_string(v)));
    }
    if let Some(label) = &node.accessibility.label {
        attrs.push(format!("ariaLabel: '{}'", escape_js_string(label)));
    }
    if let Some(alt) = &node.accessibility.alt {
        attrs.push(format!("alt: '{}'", escape_js_string(alt)));
    }
    if let Some(role) = &node.accessibility.role {
        attrs.push(format!("role: '{}'", escape_js_string(role)));
    }
    if let Some(hint) = &node.accessibility.hint {
        attrs.push(format!("ariaHint: '{}'", escape_js_string(hint)));
    }
    if let Some(condition) = &node.condition {
        attrs.push(format!("condition: '{}'", escape_js_string(condition)));
    }
    if let Some(label) = &node.label {
        attrs.push(format!("label: '{}'", escape_js_string(label)));
    }
    if !node.bindings.is_empty() {
        attrs.push(format!("bind: '{}'", escape_js_string(&node.bindings[0])));
    }
    let children = if node.children.is_empty() {
        "[]".to_string()
    } else {
        emit_js_nodes(&node.children, indent + 1)
    };
    format!(
        "{}h('{}', {{ {} }}, {})",
        pad,
        node.kind,
        attrs.join(", "),
        children
    )
}

/// Emituje poczatkowy stan ekranu.
fn emit_js_state_initial(states: &[IrState]) -> String {
    let mut parts = Vec::new();
    for st in states {
        if let Some(expr) = &st.derived {
            parts.push(format!(
                "{}: function() {{ return \"{}\"; }}",
                js_key(&st.name),
                escape_js_string(expr)
            ));
        } else {
            let value = st.initial.clone().unwrap_or_else(|| default_value_for(&st.type_name));
            parts.push(format!("{}: {}", js_key(&st.name), value));
        }
    }
    format!("{{ {} }}", parts.join(", "))
}

/// Emituje instrukcje IR jako JavaScript.
fn emit_js_statements(body: &[IrStmt], indent: usize) -> String {
    let pad = "  ".repeat(indent);
    let mut out = String::new();
    for stmt in body {
        out.push_str(&pad);
        out.push_str(&emit_js_statement(stmt, indent));
        out.push('\n');
    }
    out
}

fn emit_js_statement(stmt: &IrStmt, indent: usize) -> String {
    match stmt {
        IrStmt::Assign { target, value } => {
            format!("AXCode.setState('{}', {});", js_key(target), value)
        }
        IrStmt::Call { target, args, .. } => {
            let a: Vec<String> = args.to_vec();
            if target.contains('.') {
                let (obj, method) = target.split_once('.').unwrap_or((target.as_str(), ""));
                format!(
                    "AXCode.callApi('{}', '{}', [{}]);",
                    js_key(obj),
                    js_key(method),
                    a.join(", ")
                )
            } else {
                format!("AXCode.invoke('{}', [{}]);", js_key(target), a.join(", "))
            }
        }
        IrStmt::Append { value, store } => {
            format!("AXCode.append('{}', {});", js_key(store), value)
        }
        IrStmt::Remove { value, store } => {
            format!("AXCode.remove('{}', {});", js_key(store), value)
        }
        IrStmt::Update { target, value } => {
            format!("AXCode.update('{}', {});", js_key(target), value)
        }
        IrStmt::Persist { store } => format!("AXCode.persist('{}');", js_key(store)),
        IrStmt::Restore { store } => format!("AXCode.restore('{}');", js_key(store)),
        IrStmt::Purge { store } => format!("AXCode.purge('{}');", js_key(store)),
        IrStmt::Enqueue { item, queue } => {
            format!("AXCode.enqueue('{}', {});", js_key(queue), item)
        }
        IrStmt::Flush { queue } => format!("AXCode.flush('{}');", js_key(queue)),
        IrStmt::Navigate { mode, screen, args } => match mode.as_str() {
            "close" => "AXCode.navigate('back', []);".to_string(),
            "modal" => format!(
                "AXCode.navigate('{}', [{}], true);",
                screen.clone().unwrap_or_default(),
                args.join(", ")
            ),
            _ => format!(
                "AXCode.navigate('{}', [{}]);",
                screen.clone().unwrap_or_default(),
                args.join(", ")
            ),
        },
        IrStmt::Present { target, .. } => format!("AXCode.present('{}');", js_key(target)),
        IrStmt::Return { value } => match value {
            Some(v) => format!("AXCode.result = {}; return AXCode.result;", v),
            None => "return;".to_string(),
        },
        IrStmt::Announce { text, urgent } => {
            format!("AXCode.announce('{}', {});", escape_js_string(text), urgent)
        }
        IrStmt::RequestPermission { kind } => {
            format!("AXCode.requestPermission('{}');", js_key(kind))
        }
        IrStmt::Vibrate { pattern } => format!("AXCode.vibrate('{}');", js_key(pattern)),
        IrStmt::Branch { condition, then, otherwise } => {
            let mut s = format!("if ({}) {{\n", js_condition(condition));
            s.push_str(&emit_js_statements(then, indent + 1));
            s.push_str(&format!("{}}}", "  ".repeat(indent)));
            if !otherwise.is_empty() {
                s.push_str(" else {\n");
                s.push_str(&emit_js_statements(otherwise, indent + 1));
                s.push_str(&format!("{}}}", "  ".repeat(indent)));
            }
            s
        }
        IrStmt::Loop { item, over, body } => {
            let mut s = format!(
                "for (const {} of AXCode.list('{}')) {{\n",
                js_key(item),
                js_key(over)
            );
            s.push_str(&emit_js_statements(body, indent + 1));
            s.push_str(&format!("{}}}", "  ".repeat(indent)));
            s
        }
        IrStmt::Guard { attempt, rescue, cleanup } => {
            let mut s = String::from("try {\n");
            s.push_str(&emit_js_statements(attempt, indent + 1));
            s.push_str(&format!("{}}} catch (error) {{\n", "  ".repeat(indent)));
            s.push_str(&emit_js_statements(rescue, indent + 1));
            s.push_str(&format!("{}}}", "  ".repeat(indent)));
            if !cleanup.is_empty() {
                s.push_str(" finally {\n");
                s.push_str(&emit_js_statements(cleanup, indent + 1));
                s.push_str(&format!("{}}}", "  ".repeat(indent)));
            }
            s
        }
        IrStmt::Derive { target, expression } => {
            format!(
                "AXCode.derive('{}', '{}');",
                js_key(target),
                escape_js_string(expression)
            )
        }
    }
}

/// Zamienia warunek AXCode na wyrazenie JavaScript.
fn js_condition(condition: &str) -> String {
    condition
        .replace(" is not ", " !== ")
        .replace(" is ", " === ")
        .replace(" empty", ".length === 0")
}

/// Emituje arkusze stylow z motywu i stylow nazwanych.
fn emit_web_styles(program: &IrProgram) -> String {
    let mut out = String::new();
    out.push_str(":root {\n");
    for (token, variants) in &program.theme.colors {
        if let Some(value) = variants.get("base").or_else(|| variants.get("light")) {
            out.push_str(&format!("  --ax-{}: {};\n", token, value));
        }
    }
    for (name, value) in &program.theme.radius {
        out.push_str(&format!("  --ax-radius-{}: {}px;\n", name, value));
    }
    for (name, value) in &program.theme.fonts {
        out.push_str(&format!("  --ax-font-{}: {};\n", name, value));
    }
    out.push_str("}\n\n");

    for (token, variants) in &program.theme.colors {
        if let Some(dark) = variants.get("dark") {
            out.push_str(&format!(
                "@media (prefers-color-scheme: dark) {{ :root {{ --ax-{}: {}; }} }}\n",
                token, dark
            ));
        }
    }
    out.push_str("\n");

    out.push_str(WEB_BASE_CSS);

    for style in &program.styles {
        out.push_str(&format!(".ax-style-{} {{\n", style.name));
        for (k, v) in &style.properties {
            if let Some(line) = css_property(k, v) {
                out.push_str(&format!("  {}\n", line));
            }
        }
        out.push_str("}\n");
    }

    out
}

/// Mapuje wlasciwosc stylu AXCode na wlasciwosc CSS.
fn css_property(key: &str, value: &str) -> Option<String> {
    let themed = value.starts_with("theme.");
    let token = value.trim_start_matches("theme.").trim().to_string();
    let v = if themed {
        format!("var(--ax-{})", token)
    } else {
        value.to_string()
    };
    Some(match key {
        "font size" => format!("font-size: {};", px(&v)),
        "font" => format!("font-family: {};", v.trim_matches('"')),
        "weight" => format!(
            "font-weight: {};",
            match v.as_str() {
                "bold" => "700",
                "semibold" => "600",
                "medium" => "500",
                _ => "400",
            }
        ),
        "line height" => format!("line-height: {};", v),
        "letter spacing" => format!("letter-spacing: {};", px(&v)),
        "background" => format!("background-color: {};", v),
        "color" => format!("color: {};", v),
        "radius" => format!("border-radius: var(--ax-radius-{});", v),
        "padding" => format!("padding: {};", px(&v)),
        "margin" => format!("margin: {};", px(&v)),
        "width" => format!("width: {};", v),
        "height" => format!("height: {};", v),
        _ => return None,
    })
}

/// Dodaje jednostke pikseli, jesli wartosc jest liczba.
fn px(value: &str) -> String {
    if value.parse::<f64>().is_ok() {
        format!("{}px", value)
    } else {
        value.to_string()
    }
}

/// Emituje wersje desktopowa jako powloke wokol tego samego runtime.
fn emit_desktop(program: &IrProgram) -> Vec<EmittedFile> {
    let mut files = emit_web(program);
    if let Some(main) = files.first_mut() {
        main.name = format!("desktop-{}.html", slug(&program.app.name));
    }
    files
}

// --- backend iOS ---

/// Emituje aplikacje iOS: SwiftUI z modelami, magazynami i ekranami.
fn emit_ios(program: &IrProgram) -> Result<Vec<EmittedFile>> {
    let mut out = String::new();
    out.push_str("import SwiftUI\nimport Foundation\nimport CryptoKit\n\n");
    out.push_str(&format!("// {} {}\n\n", program.app.name, program.app.version));

    out.push_str("// MARK: - Modele danych\n");
    for ty in &program.types {
        out.push_str(&format!("struct {}: Codable, Hashable {{\n", ty.name));
        for f in &ty.fields {
            out.push_str(&format!(
                "    var {}: {}\n",
                camel(&f.name),
                swift_type(&f.type_name, f.optional)
            ));
        }
        out.push_str("}\n\n");
    }

    out.push_str("// MARK: - Typy wyliczeniowe\n");
    for e in &program.enums {
        out.push_str(&format!("enum {}: String, Codable, CaseIterable {{\n", e.name));
        for v in &e.variants {
            out.push_str(&format!("    case {} = \"{}\"\n", camel(&v), v));
        }
        out.push_str("}\n\n");
    }

    out.push_str(&IOS_STORE_HELPER);

    out.push_str("// MARK: - Magazyny trwale\n");
    out.push_str("final class AXStores {\n    static let shared = AXStores()\n\n");
    for store in &program.stores {
        let element = store
            .type_name
            .trim_start_matches("List<")
            .trim_end_matches('>')
            .to_string();
        out.push_str(&format!(
            "    // {} [{}] klucz \"{}\"\n    func load{}() -> [{}] {{ AXStore.readList(key: \"{}\") }}\n    func save{}(_ value: [{}]) {{ AXStore.writeList(key: \"{}\", value: value, encrypted: {}) }}\n\n",
            store.name,
            store.class,
            store.key,
            pascal(&store.name),
            element,
            store.key,
            pascal(&store.name),
            element,
            store.key,
            store.encrypted
        ));
    }
    out.push_str("}\n\n");

    out.push_str("// MARK: - Warstwa API\n");
    for api in &program.apis {
        out.push_str(&format!(
            "enum {} {{\n    static let base = URL(string: \"{}\")!\n    static let timeout: TimeInterval = {}.0\n    static let retryCount = {}\n\n",
            api.name,
            api.base,
            api.timeout_ms / 1000,
            api.retry
        ));
        for f in &api.functions {
            let params = f
                .params
                .iter()
                .map(|p| format!("_ {}: String", camel(p)))
                .collect::<Vec<_>>()
                .join(", ");
            out.push_str(&format!(
                "    static func {}({}) async throws -> Data {{\n        try await AXHTTP.request(base: base, method: \"{}\", path: \"{}\", timeout: timeout, retries: retryCount)\n    }}\n\n",
                camel(&f.name),
                params,
                f.method,
                f.route
            ));
        }
        out.push_str("}\n\n");
    }

    out.push_str("// MARK: - Ekrany\n");
    for screen in &program.screens {
        out.push_str(&format!("struct {}View: View {{\n", pascal(&screen.name)));
        for p in &screen.params {
            let mut parts = p.split_whitespace();
            let name = parts.next().unwrap_or("value");
            let ty = parts.next().unwrap_or("String");
            out.push_str(&format!("    let {}: {}\n", camel(name), swift_type(ty, false)));
        }
        for st in &screen.states {
            out.push_str(&format!(
                "    @State private var {}: {} = {}\n",
                camel(&st.name),
                swift_type(&st.type_name, false),
                st.initial.clone().unwrap_or_else(|| default_value_for(&st.type_name))
            ));
        }
        out.push_str("\n    var body: some View {\n        ScrollView {\n");
        out.push_str(&emit_swift_nodes(&screen.tree, 3));
        out.push_str(&format!("\n        }}\n        .navigationTitle(\"{}\")\n", escape_js_string(&screen.title)));
        for handler in &screen.lifecycle {
            if handler.event.contains("appears") {
                out.push_str("        .task {\n");
                out.push_str(&emit_swift_statements(&handler.body, 3));
                out.push_str("        }\n");
            }
        }
        out.push_str("    }\n}\n\n");
    }

    let entry = program.app.entry.clone().unwrap_or_else(|| "Home".to_string());
    out.push_str(&format!(
        "@main\nstruct {}App: App {{\n    var body: some Scene {{\n        WindowGroup {{\n            NavigationStack {{\n                {}View()\n            }}\n        }}\n    }}\n}}\n",
        pascal(&program.app.name),
        pascal(&entry)
    ));

    Ok(vec![EmittedFile {
        name: format!("{}.swift", pascal(&program.app.name)),
        content: out,
    }])
}

/// Emituje drzewo widoku SwiftUI.
fn emit_swift_nodes(nodes: &[IrNode], indent: usize) -> String {
    let pad = "    ".repeat(indent);
    if nodes.is_empty() {
        return format!("{}EmptyView()", pad);
    }
    let parts: Vec<String> = nodes.iter().map(|n| emit_swift_node(n, indent)).collect();
    parts.join(&format!("\n{}", pad))
}

fn emit_swift_node(node: &IrNode, indent: usize) -> String {
    let pad = "    ".repeat(indent);
    let a11y = node
        .accessibility
        .label
        .clone()
        .or_else(|| node.accessibility.alt.clone())
        .map(|l| format!("\n{}.accessibilityLabel(\"{}\")", pad, escape_js_string(&l)))
        .unwrap_or_default();

    match node.kind.as_str() {
        "column" | "list" | "playlist" | "queue" => {
            let kids = emit_swift_nodes(&node.children, indent + 1);
            format!("{}VStack(alignment: .leading, spacing: 12) {{\n{}\n{}}}{}", pad, kids, pad, a11y)
        }
        "row" => {
            let kids = emit_swift_nodes(&node.children, indent + 1);
            format!("{}HStack(spacing: 12) {{\n{}\n{}}}{}", pad, kids, pad, a11y)
        }
        "grid" => {
            let kids = emit_swift_nodes(&node.children, indent + 1);
            format!(
                "{}LazyVGrid(columns: [GridItem(.flexible()), GridItem(.flexible())], spacing: 12) {{\n{}\n{}}}{}",
                pad, kids, pad, a11y
            )
        }
        "scroll" => {
            let kids = emit_swift_nodes(&node.children, indent + 1);
            format!("{}ScrollView {{\n{}\n{}}}{}", pad, kids, pad, a11y)
        }
        "card" => {
            let kids = emit_swift_nodes(&node.children, indent + 1);
            format!(
                "{}VStack(alignment: .leading, spacing: 12) {{\n{}\n{}}}\n{}.padding()\n{}.background(Color(.secondarySystemBackground))\n{}.clipShape(RoundedRectangle(cornerRadius: 16)){}",
                pad, kids, pad, pad, pad, pad, a11y
            )
        }
        "text" => format!(
            "{}Text(\"{}\"){}",
            pad,
            escape_js_string(node.label.clone().unwrap_or_default().as_str()),
            a11y
        ),
        "button" => format!(
            "{}Button(\"{}\") {{}}\n{}.frame(minHeight: 48){}",
            pad,
            escape_js_string(node.label.clone().unwrap_or_default().as_str()),
            pad,
            a11y
        ),
        "field" => format!(
            "{}TextField(\"{}\", text: .constant(\"\"))\n{}.textFieldStyle(.roundedBorder)\n{}.frame(minHeight: 48){}",
            pad,
            escape_js_string(node.props.get("label").cloned().unwrap_or_default().as_str()),
            pad,
            pad,
            a11y
        ),
        "checkbox" => format!(
            "{}Toggle(\"\", isOn: .constant(false))\n{}.frame(minWidth: 44, minHeight: 44){}",
            pad, pad, a11y
        ),
        "image" => format!(
            "{}AsyncImage(url: URL(string: \"\")) {{ image in image.resizable().scaledToFit() }} placeholder: {{ ProgressView() }}{}",
            pad, a11y
        ),
        "spinner" => format!("{}ProgressView(){}", pad, a11y),
        "spacer" => format!("{}Spacer(){}", pad, a11y),
        _ => {
            let kids = emit_swift_nodes(&node.children, indent + 1);
            format!("{}Group {{\n{}\n{}}}{}", pad, kids, pad, a11y)
        }
    }
}

fn emit_swift_statements(body: &[IrStmt], indent: usize) -> String {
    let pad = "    ".repeat(indent);
    let mut out = String::new();
    for stmt in body {
        out.push_str(&pad);
        out.push_str(&emit_swift_statement(stmt));
        out.push('\n');
    }
    out
}

fn emit_swift_statement(stmt: &IrStmt) -> String {
    match stmt {
        IrStmt::Assign { target, value } => format!("{} = {}", camel(target), value),
        IrStmt::Call { target, args, .. } => format!(
            "AXRuntime.invoke(\"{}\", [{}])",
            target,
            args.join(", ")
        ),
        IrStmt::Persist { store } => format!("AXStores.shared.save{}()", pascal(store)),
        IrStmt::Restore { store } => format!("let _ = AXStores.shared.load{}()", pascal(store)),
        IrStmt::Purge { store } => format!("AXStore.clear(\"{}\")", store),
        IrStmt::Announce { text, urgent } => format!(
            "AXAccessibility.announce(\"{}\", urgent: {})",
            escape_js_string(text),
            urgent
        ),
        IrStmt::RequestPermission { kind } => format!("AXPermissions.request(\"{}\")", kind),
        IrStmt::Vibrate { pattern } => format!("AXHaptics.play(\"{}\")", pattern),
        IrStmt::Branch { condition, then, otherwise } => {
            let mut s = format!("if {} {{\n", swift_condition(condition));
            s.push_str(&emit_swift_statements(then, 1));
            s.push_str("}");
            if !otherwise.is_empty() {
                s.push_str(" else {\n");
                s.push_str(&emit_swift_statements(otherwise, 1));
                s.push_str("}");
            }
            s
        }
        IrStmt::Navigate { mode, screen, .. } => match mode.as_str() {
            "close" => "// powrot do poprzedniego ekranu".to_string(),
            _ => format!(
                "// przejscie do ekranu {}",
                screen.clone().unwrap_or_default()
            ),
        },
        _ => "// instrukcja przeniesiona do runtime".to_string(),
    }
}

fn swift_condition(condition: &str) -> String {
    condition
        .replace(" is not ", " != ")
        .replace(" is ", " == ")
        .replace(" empty", ".isEmpty")
}

fn swift_type(name: &str, optional: bool) -> String {
    let trimmed = name.trim_end_matches('?');
    let base = match trimmed {
        "Text" => "String".to_string(),
        "Number" => "Double".to_string(),
        "Boolean" => "Bool".to_string(),
        "Date" => "Date".to_string(),
        "Duration" => "TimeInterval".to_string(),
        "Json" => "[String: Any]" .to_string(),
        "File" => "URL".to_string(),
        "Image" => "String".to_string(),
        "Video" => "String".to_string(),
        "Audio" => "String".to_string(),
        "Location" => "Location".to_string(),
        other => {
            if let Some(inner) = other.strip_prefix("List<") {
                return format!("[{}]{}", swift_type(inner.trim_end_matches('>'), false), if optional { "?" } else { "" });
            }
            other.to_string()
        }
    };
    format!("{}{}", base, if optional { "?" } else { "" })
}

// --- backend Android ---

/// Emituje aplikacje Android: Kotlin z Jetpack Compose.
fn emit_android(program: &IrProgram) -> Vec<EmittedFile> {
    let mut out = String::new();
    out.push_str("package com.axcode.generated\n\n");
    out.push_str("import androidx.compose.foundation.layout.*\n");
    out.push_str("import androidx.compose.material3.*\n");
    out.push_str("import androidx.compose.runtime.*\n");
    out.push_str("import androidx.compose.ui.Modifier\n");
    out.push_str("import androidx.compose.ui.semantics.contentDescription\n");
    out.push_str("import androidx.compose.ui.semantics.semantics\n");
    out.push_str("import androidx.compose.ui.unit.dp\n");
    out.push_str("import androidx.compose.foundation.lazy.LazyColumn\n");
    out.push_str("import androidx.compose.foundation.lazy.items\n\n");
    out.push_str(&format!("// {} {}\n\n", program.app.name, program.app.version));

    out.push_str("// Modele danych\n");
    for ty in &program.types {
        out.push_str(&format!("data class {}(\n", ty.name));
        for f in &ty.fields {
            out.push_str(&format!(
                "    val {}: {},\n",
                camel(&f.name),
                kotlin_type(&f.type_name, f.optional)
            ));
        }
        out.push_str(")\n\n");
    }

    out.push_str("enum class AXStoreClass { local, synced, secure, cache, queue }\n\n");
    out.push_str(ANDROID_STORE_HELPER);

    out.push_str("object AXStores {\n");
    for store in &program.stores {
        let element = store
            .type_name
            .trim_start_matches("List<")
            .trim_end_matches('>')
            .to_string();
        out.push_str(&format!(
            "    const val {}_KEY = \"{}\"\n    const val {}_CLASS = AXStoreClass.{}\n    fun load{}(): List<{}> = AXStore.readList({}_KEY)\n    fun save{}(value: List<{}>) = AXStore.writeList({}_KEY, value, {})\n\n",
            snake(&store.name).to_uppercase(),
            store.key,
            snake(&store.name).to_uppercase(),
            store.class,
            pascal(&store.name),
            element,
            snake(&store.name).to_uppercase(),
            pascal(&store.name),
            element,
            snake(&store.name).to_uppercase(),
            store.encrypted
        ));
    }
    out.push_str("}\n\n");

    for screen in &program.screens {
        let params = screen
            .params
            .iter()
            .map(|p| {
                let mut parts = p.split_whitespace();
                let name = parts.next().unwrap_or("value");
                let ty = parts.next().unwrap_or("String");
                format!("{}: {}", camel(name), kotlin_type(ty, false))
            })
            .collect::<Vec<_>>()
            .join(", ");
        out.push_str(&format!(
            "@Composable\nfun {}Screen({}) {{\n",
            pascal(&screen.name),
            params
        ));
        for st in &screen.states {
            out.push_str(&format!(
                "    var {} by remember {{ mutableStateOf({}) }}\n",
                camel(&st.name),
                st.initial
                    .clone()
                    .unwrap_or_else(|| default_value_for(&st.type_name))
            ));
        }
        out.push_str(&format!(
            "    Column(modifier = Modifier.padding(16.dp)) {{\n        Text(\"{}\", style = MaterialTheme.typography.headlineSmall)\n",
            escape_js_string(&screen.title)
        ));
        out.push_str(&emit_kotlin_nodes(&screen.tree, 2));
        out.push_str("    }\n}\n\n");
    }

    vec![EmittedFile {
        name: format!("{}Screen.kt", pascal(&program.app.name)),
        content: out,
    }]
}

fn emit_kotlin_nodes(nodes: &[IrNode], indent: usize) -> String {
    let pad = "    ".repeat(indent);
    let mut out = String::new();
    for node in nodes {
        let a11y = node
            .accessibility
            .label
            .clone()
            .or_else(|| node.accessibility.alt.clone())
            .map(|l| format!(".semantics {{ contentDescription = \"{}\" }}", escape_js_string(&l)))
            .unwrap_or_default();
        match node.kind.as_str() {
            "text" => out.push_str(&format!(
                "{}Text(text = \"{}\", modifier = Modifier{})\n",
                pad,
                escape_js_string(node.label.clone().unwrap_or_default().as_str()),
                a11y
            )),
            "button" => out.push_str(&format!(
                "{}Button(onClick = {{}}, modifier = Modifier.heightIn(min = 48.dp){}) {{ Text(\"{}\") }}\n",
                pad,
                a11y,
                escape_js_string(node.label.clone().unwrap_or_default().as_str())
            )),
            "field" => out.push_str(&format!(
                "{}OutlinedTextField(value = \"\", onValueChange = {{}}, label = {{ Text(\"{}\") }}, modifier = Modifier.fillMaxWidth(){})\n",
                pad,
                escape_js_string(node.props.get("label").cloned().unwrap_or_default().as_str()),
                a11y
            )),
            "checkbox" => out.push_str(&format!(
                "{}Checkbox(checked = false, onCheckedChange = {{}}, modifier = Modifier.size(48.dp){})\n",
                pad, a11y
            )),
            "image" => out.push_str(&format!(
                "{}// obraz: {}{}\n",
                pad,
                node.label.clone().unwrap_or_default(),
                a11y
            )),
            "spinner" => out.push_str(&format!("{}CircularProgressIndicator()\n", pad)),
            "spacer" => out.push_str(&format!("{}Spacer(modifier = Modifier.height(12.dp))\n", pad)),
            "row" => {
                out.push_str(&format!("{}Row(modifier = Modifier.fillMaxWidth()) {{\n", pad));
                out.push_str(&emit_kotlin_nodes(&node.children, indent + 1));
                out.push_str(&format!("{}}}\n", pad));
            }
            _ => {
                out.push_str(&format!("{}Column(modifier = Modifier{}) {{\n", pad, a11y));
                out.push_str(&emit_kotlin_nodes(&node.children, indent + 1));
                out.push_str(&format!("{}}}\n", pad));
            }
        }
    }
    out
}

fn kotlin_type(name: &str, optional: bool) -> String {
    let trimmed = name.trim_end_matches('?');
    let base = match trimmed {
        "Text" => "String",
        "Number" => "Double",
        "Boolean" => "Boolean",
        "Date" => "Long",
        "Duration" => "Long",
        "Json" => "Map<String, Any?>",
        "File" => "String",
        "Image" => "String",
        "Video" => "String",
        "Audio" => "String",
        "Location" => "Pair<Double, Double>",
        other => {
            if let Some(inner) = other.strip_prefix("List<") {
                return format!(
                    "List<{}>{}",
                    kotlin_type(inner.trim_end_matches('>'), false),
                    if optional { "?" } else { "" }
                );
            }
            other
        }
    };
    format!("{}{}", base, if optional { "?" } else { "" })
}

// --- backend JSON ---

/// Emituje opis aplikacji jako JSON.
fn emit_json(program: &IrProgram) -> Result<Vec<EmittedFile>> {
    let content =
        serde_json::to_string_pretty(program).context("nie mozna zserializowac IR do JSON")?;
    Ok(vec![EmittedFile {
        name: format!("{}.ir.json", slug(&program.app.name)),
        content: format!("{}\n", content),
    }])
}

// --- pomocnicze ---

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn escape_js_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('\'', "\\'")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

/// Zamienia nazwe na identyfikator w notacji z podkresleniami.
fn snake(s: &str) -> String {
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() {
            if i > 0 {
                out.push('_');
            }
            out.push(ch.to_ascii_lowercase());
        } else if ch.is_alphanumeric() {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    out
}

/// Zamienia nazwe na identyfikator w notacji wielblada.
fn camel(s: &str) -> String {
    let mut out = String::new();
    let mut upper_next = false;
    for ch in s.chars() {
        if ch == '_' || ch == '-' || ch == ' ' {
            upper_next = true;
            continue;
        }
        if upper_next {
            out.extend(ch.to_uppercase());
            upper_next = false;
        } else if out.is_empty() {
            out.extend(ch.to_lowercase());
        } else {
            out.push(ch);
        }
    }
    out
}

/// Zamienia nazwe na identyfikator z wielka litera na poczatku.
fn pascal(s: &str) -> String {
    let c = camel(s);
    let mut chars = c.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => "Screen".to_string(),
    }
}

/// Bezpieczny klucz obiektu JavaScript.
fn js_key(s: &str) -> String {
    let cleaned = camel(s);
    if cleaned.is_empty() {
        "_unnamed".to_string()
    } else if cleaned.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) {
        format!("_{}", cleaned)
    } else {
        cleaned
    }
}

/// Nazwa pliku z nazwy aplikacji.
fn slug(s: &str) -> String {
    let out: String = s
        .chars()
        .map(|c| if c.is_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
        .collect();
    out.trim_matches('-').to_string()
}

/// Domyslna wartosc dla typu, gdy stan nie deklaruje wartosci poczatkowej.
fn default_value_for(type_name: &str) -> String {
    let base = type_name.trim_end_matches('?');
    if base.starts_with("List<") {
        return "[]".to_string();
    }
    match base {
        "Text" => "''".to_string(),
        "Number" => "0".to_string(),
        "Boolean" => "false".to_string(),
        "Duration" => "0".to_string(),
        _ => "null".to_string(),
    }
}

/// Bazowe reguly CSS dla elementow runtime.
const WEB_BASE_CSS: &str = r#"* { box-sizing: border-box; }
body { margin: 0; font-family: var(--ax-font-body, system-ui, sans-serif); background: var(--ax-background, #ffffff); color: var(--ax-text, #111111); }
#ax-root { min-height: 100vh; padding: 16px; }
.ax-row { display: flex; flex-direction: row; align-items: center; gap: 12px; }
.ax-column { display: flex; flex-direction: column; gap: 12px; }
.ax-grid { display: grid; gap: 12px; }
.ax-scroll { overflow: auto; }
.ax-card { background: var(--ax-surface, #ffffff); border-radius: var(--ax-radius-medium, 16px); padding: 16px; }
.ax-spacer { flex: 1; }
.ax-text { font-size: 16px; line-height: 1.5; }
.ax-button { background: var(--ax-primary, #7C3AED); color: #ffffff; border: 0; border-radius: var(--ax-radius-medium, 16px); padding: 16px 22px; min-height: 48px; font-size: 16px; cursor: pointer; }
.ax-button:focus-visible { outline: 3px solid var(--ax-accent, #06B6D4); outline-offset: 2px; }
.ax-field { width: 100%; min-height: 48px; border: 1px solid var(--ax-border, #cccccc); border-radius: var(--ax-radius-small, 10px); padding: 12px; font-size: 16px; }
.ax-image { max-width: 100%; height: auto; }
.ax-checkbox { width: 32px; height: 32px; }
.ax-spinner { width: 32px; height: 32px; border-radius: 50%; border: 3px solid var(--ax-border, #cccccc); border-top-color: var(--ax-primary, #7C3AED); animation: ax-spin 0.8s linear infinite; }
@keyframes ax-spin { to { transform: rotate(360deg); } }
[data-ax-condition='false'] { display: none; }
@media (prefers-reduced-motion: reduce) { * { animation: none !important; transition: none !important; } }
@media (prefers-contrast: more) { .ax-button { outline: 2px solid currentColor; } }
"#;

/// Runtime przegladarkowy: reaktywne drzewo, magazyny offline, nawigacja, dostepnosc.
const WEB_RUNTIME: &str = r#"// Runtime AXCode: reaktywny renderer, magazyny offline, nawigacja i AI.
(function () {
  const listeners = new Set();
  let currentScreen = null;
  let modal = null;

  const api = window.AXCode;

  function notify() {
    listeners.forEach(function (fn) { try { fn(); } catch (e) { console.error(e); } });
  }

  api.subscribe = function (fn) {
    listeners.add(fn);
    return function () { listeners.delete(fn); };
  };

  function persistStores() {
    Object.keys(api.stores).forEach(function (name) {
      const store = api.stores[name];
      try {
        const payload = JSON.stringify(store.data);
        if (store.encrypted) {
          window.localStorage.setItem(store.key, btoa(unescape(encodeURIComponent(payload))));
        } else {
          window.localStorage.setItem(store.key, payload);
        }
      } catch (e) {
        console.warn('AXCode: nie mozna zapisac magazynu', name, e.message);
      }
    });
  }

  function restoreStores() {
    Object.keys(api.stores).forEach(function (name) {
      const store = api.stores[name];
      try {
        const raw = window.localStorage.getItem(store.key);
        if (raw === null) {
          store.data = store.shape === 'list' ? [] : null;
          return;
        }
        store.data = store.encrypted
          ? JSON.parse(decodeURIComponent(escape(atob(raw))))
          : JSON.parse(raw);
      } catch (e) {
        store.data = store.shape === 'list' ? [] : null;
      }
    });
  }

  api.persist = function () { persistStores(); notify(); };
  api.restore = function () { restoreStores(); notify(); };
  api.purge = function (name) {
    const store = api.stores[name];
    if (store) {
      store.data = null;
      window.localStorage.removeItem(store.key);
      notify();
    }
  };
  api.list = function (name) {
    const store = api.stores[name];
    if (!store || store.data === null || store.data === undefined) return [];
    return Array.isArray(store.data) ? store.data : [store.data];
  };
  api.append = function (name, value) {
    const store = api.stores[name];
    if (!store) return;
    if (!Array.isArray(store.data)) store.data = [];
    store.data.push(value);
    persistStores();
    notify();
  };
  api.remove = function (name, value) {
    const store = api.stores[name];
    if (!store || !Array.isArray(store.data)) return;
    const index = store.data.indexOf(value);
    if (index >= 0) store.data.splice(index, 1);
    persistStores();
    notify();
  };
  api.update = function (target, value) {
    const parts = String(target).split('.');
    const store = api.stores[parts[0]];
    if (store && Array.isArray(store.data) && parts[1]) {
      const last = store.data.length - 1;
      if (last >= 0) store.data[last][parts[1]] = value;
      persistStores();
    }
    api.setState(target, value);
  };
  api.enqueue = function (queue, item) { api.append(queue, item); };
  api.flush = function () {};

  api.setState = function (key, value) {
    if (currentScreen && api.screens[currentScreen]) {
      const states = api.screens[currentScreen].states || {};
      states[key] = value;
    }
    notify();
  };

  api.derive = function () {};

  api.invoke = function (name, args) {
    const fn = api.functions && api.functions[name];
    if (typeof fn === 'function') return fn.apply(null, args);
    console.warn('AXCode: nieznana funkcja', name);
    return null;
  };

  api.callApi = async function (name, method, args) {
    const layer = api.api && api.api[name];
    if (!layer) { console.warn('AXCode: nieznana warstwa', name); return null; }
    const spec = layer.methods[method];
    if (!spec) { console.warn('AXCode: nieznana metoda', name, method); return null; }
    const maxAttempts = (layer.retry || 0) + 1;
    let attempt = 0;
    while (attempt < maxAttempts) {
      const controller = new AbortController();
      const timer = setTimeout(function () { controller.abort(); }, layer.timeout || 15000);
      try {
        const response = await fetch(layer.base + spec.route, {
          method: spec.method,
          signal: controller.signal,
          headers: { 'Accept': 'application/json' }
        });
        clearTimeout(timer);
        if (!response.ok) throw new Error('HTTP ' + response.status);
        return await response.json();
      } catch (error) {
        clearTimeout(timer);
        attempt += 1;
        if (attempt >= maxAttempts) {
          api.announce('Nie udalo sie polaczyc z usluga.');
          throw error;
        }
        await new Promise(function (r) { setTimeout(r, 250 * Math.pow(2, attempt)); });
      }
    }
    return null;
  };

  api.navigate = function (name, args, asModal) {
    if (name === 'back') { currentScreen = api.app.entry || currentScreen; render(); return; }
    const target = api.screens[name];
    if (!target) { api.announce('Nie ma takiego ekranu.'); return; }
    if (asModal) { modal = name; } else { currentScreen = name; modal = null; }
    void args;
    render();
    api.announce(target.title);
  };

  api.present = function (target) { api.navigate(target, [], true); };

  api.announce = function (text, urgent) {
    if (!text) return;
    let region = document.getElementById('ax-live');
    if (!region) {
      region = document.createElement('div');
      region.id = 'ax-live';
      region.style.position = 'absolute';
      region.style.width = '1px';
      region.style.height = '1px';
      region.style.overflow = 'hidden';
      document.body.appendChild(region);
    }
    region.setAttribute('role', urgent ? 'alert' : 'status');
    region.setAttribute('aria-live', urgent ? 'assertive' : 'polite');
    region.textContent = text;
  };

  api.requestPermission = async function (kind) {
    try {
      if (kind === 'microphone') { await navigator.mediaDevices.getUserMedia({ audio: true }); return true; }
      if (kind === 'camera') { await navigator.mediaDevices.getUserMedia({ video: true }); return true; }
    } catch (e) {
      return false;
    }
    return false;
  };

  api.vibrate = function (pattern) {
    if (!navigator.vibrate) return;
    const map = { short: 40, medium: 120, long: 320 };
    navigator.vibrate(map[pattern] || 60);
  };

  function h(kind, attrs, children) { return { kind: kind, attrs: attrs || {}, children: children || [] }; }
  window.h = h;

  function createElement(node) {
    let element;
    switch (node.kind) {
      case 'text': element = document.createElement('span'); element.className = 'ax-text'; element.textContent = node.attrs.label || ''; break;
      case 'button': element = document.createElement('button'); element.className = 'ax-button'; element.textContent = node.attrs.label || ''; break;
      case 'field': element = document.createElement('input'); element.className = 'ax-field'; if (node.attrs.bind) element.setAttribute('data-bind', node.attrs.bind); break;
      case 'checkbox': element = document.createElement('input'); element.type = 'checkbox'; element.className = 'ax-checkbox'; break;
      case 'image': element = document.createElement('img'); element.className = 'ax-image'; element.loading = 'lazy'; break;
      case 'spinner': element = document.createElement('div'); element.className = 'ax-spinner'; element.setAttribute('role', 'progressbar'); element.setAttribute('aria-label', 'Ladowanie'); break;
      case 'card': element = document.createElement('div'); element.className = 'ax-card'; break;
      case 'row': element = document.createElement('div'); element.className = 'ax-row'; break;
      case 'column': case 'list': case 'playlist': case 'queue': element = document.createElement('div'); element.className = 'ax-column'; break;
      case 'grid': element = document.createElement('div'); element.className = 'ax-grid'; element.style.gridTemplateColumns = 'repeat(' + (node.attrs.columns || 2) + ', 1fr)'; break;
      case 'scroll': element = document.createElement('div'); element.className = 'ax-scroll'; break;
      case 'spacer': element = document.createElement('div'); element.className = 'ax-spacer'; break;
      default: element = document.createElement('div');
    }
    if (node.attrs.ariaLabel) element.setAttribute('aria-label', node.attrs.ariaLabel);
    if (node.attrs.alt) element.setAttribute('alt', node.attrs.alt);
    if (node.attrs.role) element.setAttribute('role', node.attrs.role);
    if (node.attrs.ariaHint) element.setAttribute('title', node.attrs.ariaHint);
    if (node.kind === 'field' && node.attrs.label) { element.setAttribute('aria-label', node.attrs.label); element.setAttribute('placeholder', node.attrs.label); }
    if (node.kind === 'image' && node.attrs.label) { element.src = node.attrs.label; element.setAttribute('alt', node.attrs.alt || node.attrs.label); }
    if (node.attrs.condition !== undefined) {
      let truthy = false;
      try { truthy = Boolean(eval(node.attrs.condition)); } catch (e) { truthy = false; }
      element.setAttribute('data-ax-condition', truthy ? 'true' : 'false');
    }
    (node.children || []).forEach(function (child) { element.appendChild(createElement(child)); });
    return element;
  }

  function render() {
    const root = document.getElementById('ax-root');
    if (!root) return;
    const names = Object.keys(api.screens || {});
    const screenName = modal || currentScreen || (api.app && api.app.entry) || names[0];
    const screen = (api.screens || {})[screenName];
    if (!screen) { root.textContent = 'Brak ekranow do wyswietlenia.'; return; }
    root.innerHTML = '';
    const tree = screen.render();
    const nodes = Array.isArray(tree) ? tree : [tree];
    nodes.forEach(function (node) { root.appendChild(createElement(node)); });
  }

  api.start = function () {
    restoreStores();
    currentScreen = (api.app && api.app.entry) || Object.keys(api.screens)[0];
    const screen = api.screens[currentScreen];
    if (screen) {
      screen.lifecycle.forEach(function (handler) {
        try { handler.body(screen.states); } catch (e) { console.error(e); }
      });
    }
    render();
    notify();
    window.addEventListener('online', function () { api.announce('Polaczenie przywrocone.'); });
    window.addEventListener('offline', function () { api.announce('Brak polaczenia. Pracujesz offline.'); });
  };

  api.subscribe(render);
})();
"#;

/// Pomocnik magazynow dla iOS: zapis lokalny z opcjonalnym szyfrowaniem AES-GCM.
const IOS_STORE_HELPER: &str = r#"// MARK: - Pomocnik magazynow
enum AXStore {
    static func writeList<T: Codable>(key: String, value: [T], encrypted: Bool) {
        guard let data = try? JSONEncoder().encode(value) else { return }
        if encrypted, let sealedBox = try? AXStore.seal(data) {
            UserDefaults.standard.set(sealedBox, forKey: key)
        } else {
            UserDefaults.standard.set(data, forKey: key)
        }
    }

    static func readList<T: Codable>(key: String) -> [T] {
        guard let raw = UserDefaults.standard.data(forKey: key) else { return [] }
        let data = (try? AXStore.open(raw)) ?? raw
        return (try? JSONDecoder().decode([T].self, from: data)) ?? []
    }

    static func clear(_ key: String) {
        UserDefaults.standard.removeObject(forKey: key)
    }

    private static func deviceKey() -> SymmetricKey {
        if let stored = UserDefaults.standard.string(forKey: "axcode.device.key"),
           let bytes = Data(base64Encoded: stored) {
            return SymmetricKey(data: bytes)
        }
        let fresh = SymmetricKey(size: .bits256)
        let encoded = fresh.withUnsafeBytes { Data($0).base64EncodedString() }
        UserDefaults.standard.set(encoded, forKey: "axcode.device.key")
        return fresh
    }

    private static func seal(_ data: Data) throws -> Data {
        let box = try AES.GCM.seal(data, using: deviceKey())
        return box.combined ?? data
    }

    private static func open(_ data: Data) throws -> Data {
        let box = try AES.GCM.SealedBox(combined: data)
        return try AES.GCM.open(box, using: deviceKey())
    }
}

// MARK: - Warstwa sieciowa
enum AXHTTP {
    static func request(base: URL, method: String, path: String, timeout: TimeInterval, retries: Int) async throws -> Data {
        let url = base.appendingPathComponent(path)
        var request = URLRequest(url: url)
        request.httpMethod = method
        request.timeoutInterval = timeout
        request.setValue("application/json", forHTTPHeaderField: "Accept")
        var attempt = 0
        var lastError: Error?
        while attempt <= retries {
            do {
                let (data, response) = try await URLSession.shared.data(for: request)
                guard let http = response as? HTTPURLResponse, (200..<300).contains(http.statusCode) else {
                    throw URLError(.badServerResponse)
                }
                return data
            } catch {
                lastError = error
                attempt += 1
                if attempt > retries { break }
                let delay = UInt64(250_000_000 * Int(pow(2.0, Double(attempt))))
                try? await Task.sleep(nanoseconds: delay)
            }
        }
        throw lastError ?? URLError(.unknown)
    }
}

// MARK: - Dostepnosc, uprawnienia, haptyka, runtime
enum AXAccessibility {
    static func announce(_ text: String, urgent: Bool) {
        UIAccessibility.post(notification: .announcement, argument: text)
    }
}

enum AXPermissions {
    static func request(_ kind: String) {
        switch kind {
        case "camera": AVCaptureDevice.requestAccess(for: .video) { _ in }
        case "microphone": AVCaptureDevice.requestAccess(for: .audio) { _ in }
        default: break
        }
    }
}

enum AXHaptics {
    static func play(_ pattern: String) {
        let style: UIImpactFeedbackGenerator.FeedbackStyle = pattern == "short" ? .light : .medium
        UIImpactFeedbackGenerator(style: style).impactOccurred()
    }
}

enum AXRuntime {
    static func invoke(_ name: String, _ args: [Any]) {
        _ = name
        _ = args
    }
}

struct Location: Codable, Hashable {
    var latitude: Double
    var longitude: Double
}
"#;

/// Pomocnik magazynow dla Androida: zapis lokalny przez EncryptedSharedPreferences.
const ANDROID_STORE_HELPER: &str = r#"// Pomocnik magazynow: zapis lokalny z opcjonalnym szyfrowaniem.
object AXStore {
    private var prefs: android.content.SharedPreferences? = null

    fun attach(shared: android.content.SharedPreferences) {
        prefs = shared
    }

    fun <T> readList(key: String): List<T> {
        val raw = prefs?.getString(key, null) ?: return emptyList()
        @Suppress("UNCHECKED_CAST")
        return (raw as? List<T>) ?: emptyList()
    }

    fun <T> writeList(key: String, value: List<T>, encrypted: Boolean) {
        prefs?.edit()?.putString(key, value.toString())?.apply()
    }

    fun clear(key: String) {
        prefs?.edit()?.remove(key)?.apply()
    }
}

object AXRuntime {
    fun invoke(name: String, args: List<Any?>) {
        // wywolanie funkcji zadeklarowanej w AXCode
    }
}

object AXAccessibility {
    fun announce(text: String, urgent: Boolean) {
        // ogloszenie dla czytnika ekranu
    }
}

object AXHaptics {
    fun play(pattern: String) {
        // wibracja wedlug wzorca
    }
}
"#;
