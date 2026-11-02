// src/parser.rs — analiza skladniowa AXCode
//
// Parser rekurencyjnie zstepujacy z odzyskiwaniem po bledzie. Kazda deklaracja
// najwyzszego poziomu jest samodzielna: blad w jednej nie przerywa parsowania
// pozostalych, a parser synchronizuje sie na poczatku nastepnej deklaracji.
//
// Skladnia opiera sie na wcieciach tak jak Python: blok nalezy do naglowka
// o wyzszym wcieciu. Poziom wciecia jest liczony w spacji (tab = 4 spacje).

use crate::ast::*;
use crate::diagnostics::{Diagnostics, ErrorCatalog};
use std::collections::HashMap;

/// Poziom wciecia liczony w znakach.
const TAB_WIDTH: usize = 4;

/// Jedna linia zrodla z policzonym wcieciem.
y
#[derive(Debug, Clone)]
struct Line<'a> {
    text: &'a str,
    indent: usize,
    line_no: usize,
    col: usize,
}

/// Parser pliku AXCode.
pub struct Parser<'a> {
    source: &'a str,
    lines: Vec<Line<'a>>,
    pos: usize,
    diagnostics: &'a mut Diagnostics,
    /// Rejestr typow zadeklarowanych w pliku, na potrzeby sprawdzen typow.
    type_env: HashMap<String, TypeRef>,
}

impl<'a> Parser<'a> {
    pub fn new(source: &'a str, diagnostics: &'a mut Diagnostics) -> Self {
        Self {
            source,
            lines: Vec::new(),
            pos: 0,
            diagnostics,
            type_env: HashMap::new(),
        }
    }

    /// Punkt wejscia: buduje liste linii i parsuje cala jednostke translacyjna.
    pub fn parse(&mut self) -> Program {
        self.lines = build_lines(self.source);
        let mut program = Program::default();
        while self.pos < self.lines.len() {
            let line = self.lines[self.pos].clone();
            if line.text.is_empty() {
                self.pos += 1;
                continue;
            }
            let before = self.pos;
            self.parse_top_level(&mut program);
            if self.pos == before {
                self.error_at(&line, ErrorCatalog::Syntax("nieznana deklaracja najwyzszego poziomu"), None);
                self.pos += 1;
            }
        }
        program
    }

    // --- deklaracje najwyzszego poziomu ---

    fn parse_top_level(&mut self, program: &mut Program) {
        let head = self.lines[self.pos].clone();
        let keyword = first_word(head.text);
        match keyword {
            "app" => self.parse_app(program),
            "version" => self.parse_app_field(program, AppField::Version),
            "language" => self.parse_app_field(program, AppField::Language),
            "supports" => self.parse_app_field(program, AppField::Supports),
            "orientation" => self.parse_app_field(program, AppField::Orientation),
            "offline" => self.parse_app_field(program, AppField::Offline),
            "entry" => self.parse_app_field(program, AppField::Entry),
            "theme" => self.parse_theme(program),
            "style" => self.parse_style(program),
            "enum" => self.parse_enum(program),
            "data" => self.parse_data(program),
            "store" => self.parse_store(program),
            "global" => self.parse_global(program),
            "permission" => self.parse_permission(program),
            "api" => self.parse_api(program),
            "function" => self.parse_function(program),
            "component" => self.parse_component(program),
            "screen" => self.parse_screen(program),
            "test" => self.parse_test(program, TestKind::Unit),
            "ui" => self.parse_test(program, TestKind::Ui),
            "accessibility" => self.parse_test(program, TestKind::Accessibility),
            _ => {
                self.error_at(
                    &head,
                    ErrorCatalog::Syntax("oczekiwano deklaracji: app, theme, style, enum, data, store, global, permission, api, function, component, screen, test"),
                    None,
                );
                self.pos += 1;
            }
        }
    }

    fn parse_app(&mut self, program: &mut Program) {
        let line = self.lines[self.pos].clone();
        let rest = line.text[3..].trim();
        match unquote(rest) {
            Some(name) => program.app.name = Some(name),
            None => self.error_at(&line, ErrorCatalog::Syntax("nazwa aplikacji musi byc w cudzyslowie"), None),
        }
        program.app.line = line.line_no;
        self.pos += 1;
    }

    fn parse_app_field(&mut self, program: &mut Program, field: AppField) {
        let line = self.lines[self.pos].clone();
        let rest = line.text.trim_start_matches(|c: char| c.is_ascii_alphanumeric()).trim();
        let value = rest.trim_matches('"').to_string();
        match field {
            AppField::Version => program.app.version = Some(value),
            AppField::Language => program.app.language = Some(value),
            AppField::Supports => {
                program.app.supports = value
                    .split_whitespace()
                    .map(|s| s.trim_matches(',').to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
            }
            AppField::Orientation => program.app.orientation = Some(value),
            AppField::Offline => program.app.offline = Some(value),
            AppField::Entry => program.app.entry = Some(value),
        }
        self.pos += 1;
    }

    fn parse_theme(&mut self, program: &mut Program) {
        let head = self.lines[self.pos].clone();
        let name = head.text[5..].trim().to_string();
        let body = self.collect_block();
        let mut theme = Theme { name: name.clone(), line: head.line_no, ..Default::default() };
        for ln in &body {
            let trimmed = ln.text.trim();
            if trimmed.is_empty() {
                continue;
            }
            if trimmed.starts_with("follows") {
                theme.follows_system = trimmed.ends_with("system");
                continue;
            }
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() >= 3 && parts[0] == "color" {
                let token = parts[1].to_string();
                let has_variant = parts.len() >= 4;
                if has_variant {
                    let variant = parts[2].to_string();
                    let value = parts[3].to_string();
                    theme
                        .colors
                        .entry(token)
                        .or_default()
                        .insert(variant, value);
                } else {
                    theme
                        .colors
                        .entry(token)
                        .or_default()
                        .insert("base".to_string(), parts[2].to_string());
                }
            } else if parts.len() >= 3 && parts[0] == "radius" {
                if let Ok(v) = parts[2].parse::<f64>() {
                    theme.radius.insert(parts[1].to_string(), v);
                }
            } else if parts.len() >= 3 && parts[0] == "font" {
                theme.fonts.insert(parts[1].to_string(), parts[2].trim_matches('"').to_string());
            }
        }
        program.themes.push(theme);
    }

    fn parse_style(&mut self, program: &mut Program) {
        let head = self.lines[self.pos].clone();
        let name = head.text[5..].trim().to_string();
        let body = self.collect_block();
        let mut props = Vec::new();
        for ln in &body {
            let trimmed = ln.text.trim();
            if trimmed.is_empty() {
                continue;
            }
            let (key, value) = split_pair(trimmed);
            props.push(StyleProp { key, value, line: ln.line_no });
        }
        program.styles.push(NamedStyle { name, props, line: head.line_no });
    }

    fn parse_enum(&mut self, program: &mut Program) {
        let head = self.lines[self.pos].clone();
        let name = head.text[4..].trim().to_string();
        let body = self.collect_block();
        let mut variants = Vec::new();
        for ln in &body {
            let trimmed = ln.text.trim();
            if trimmed.is_empty() {
                continue;
            }
            variants.push(EnumVariant { name: trimmed.to_string(), line: ln.line_no });
        }
        if variants.is_empty() {
            self.error_at(&head, ErrorCatalog::Syntax("enum musi miec co najmniej jeden wariant"), None);
        }
        self.type_env.insert(name.clone(), TypeRef::Named(name.clone()));
        program.enums.push(EnumDecl { name, variants, line: head.line_no });
    }

    fn parse_data(&mut self, program: &mut Program) {
        let head = self.lines[self.pos].clone();
        let name = head.text[4..].trim().to_string();
        let body = self.collect_block();
        let mut fields = Vec::new();
        for ln in &body {
            let trimmed = ln.text.trim();
            if trimmed.is_empty() {
                continue;
            }
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() < 2 {
                self.error_at(ln, ErrorCatalog::Syntax("pole modelu wymaga nazwy i typu"), None);
                continue;
            }
            let field_name = parts[0].to_string();
            let raw_type = parts[1].to_string();
            let ty = parse_type(&raw_type);
            if !is_known_type(&ty, &self.type_env) {
                self.error_at(
                    ln,
                    ErrorCatalog::Types(format!("nieznany typ {}", raw_type)),
                    Some(format!("zadeklaruj typ {} blokiem data, albo uzyj typu wbudowanego", raw_type)),
                );
            }
            fields.push(FieldDecl { name: field_name, ty, line: ln.line_no });
        }
        if fields.is_empty() {
            self.error_at(&head, ErrorCatalog::Syntax("model danych musi miec co najmniej jedno pole"), None);
        }
        self.type_env.insert(name.clone(), TypeRef::Named(name.clone()));
        program.data.push(DataDecl { name, fields, line: head.line_no });
    }

    fn parse_store(&mut self, program: &mut Program) {
        let head = self.lines[self.pos].clone();
        let rest: Vec<&str> = head.text[5..].split_whitespace().collect();
        if rest.is_empty() {
            self.error_at(&head, ErrorCatalog::State("store wymaga nazwy"), None);
            self.pos += 1;
            return;
        }
        let name = rest[0].to_string();
        let mut class = StoreClass::Local;
        let mut encryption = false;
        for token in rest.iter().skip(1) {
            match *token {
                "local" => class = StoreClass::Local,
                "synced" => class = StoreClass::Synced,
                "cache" => class = StoreClass::Cache,
                "queue" => class = StoreClass::Queue,
                "secure" => {
                    class = StoreClass::Secure;
                    encryption = true;
                }
                "encrypted" => encryption = true,
                other => self.error_at(
                    &head,
                    ErrorCatalog::State(format!("nieznana klasa store: {}", other)),
                    Some("dozwolone klasy: local, synced, cache, queue, secure, encrypted".to_string()),
                ),
            }
        }
        let body = self.collect_block();
        let mut shape = StoreShape::List(TypeRef::Json);
        let mut key = None;
        for ln in &body {
            let trimmed = ln.text.trim();
            if trimmed.is_empty() {
                continue;
            }
            if trimmed.starts_with("single ") {
                let ty = parse_type(trimmed[7..].trim());
                shape = StoreShape::Single(ty);
            } else if trimmed.starts_with("list ") {
                let ty = parse_type(trimmed[5..].trim());
                shape = StoreShape::List(ty);
            } else if trimmed.starts_with("map ") {
                let ty = parse_type(trimmed[4..].trim());
                shape = StoreShape::Map(TypeRef::Text, ty);
            } else if trimmed.starts_with("key ") {
                key = unquote(trimmed[4..].trim());
            }
        }
        if key.is_none() {
            self.error_at(&head, ErrorCatalog::State("store musi deklarowac klucz przez key \"...\""), Some("np. key \"app.messages\"".to_string()));
        }
        program.stores.push(StoreDecl { name, class, encryption, shape, key, line: head.line_no });
    }

    fn parse_global(&mut self, program: &mut Program) {
        let head = self.lines[self.pos].clone();
        let rest = head.text.trim_start_matches("global").trim_start_matches("state").trim();
        let (decl, init) = match rest.split_once('=') {
            Some((d, i)) => (d.trim(), Some(i.trim().to_string())),
            None => (rest, None),
        };
        let parts: Vec<&str> = decl.split_whitespace().collect();
        if parts.len() < 2 {
            self.error_at(&head, ErrorCatalog::State("stan globalny wymaga nazwy i typu"), None);
            self.pos += 1;
            return;
        }
        program.globals.push(GlobalDecl {
            name: parts[0].to_string(),
            ty: parse_type(parts[1]),
            initial: init,
            line: head.line_no,
        });
        self.pos += 1;
    }

    fn parse_permission(&mut self, program: &mut Program) {
        let head = self.lines[self.pos].clone();
        let kind = head.text[10..].trim().to_string();
        let body = self.collect_block();
        let mut purpose = String::new();
        let mut required = true;
        for ln in &body {
            let trimmed = ln.text.trim();
            if trimmed.starts_with("purpose ") {
                purpose = unquote(trimmed[8..].trim()).unwrap_or_default();
            } else if trimmed == "required false" {
                required = false;
            }
        }
        if purpose.is_empty() {
            self.error_at(&head, ErrorCatalog::Security("kazde uprawnienie musi opisac cel przez purpose \"...\""), None);
        }
        program.permissions.push(PermissionDecl { kind, purpose, required, line: head.line_no });
    }

    fn parse_api(&mut self, program: &mut Program) {
        let head = self.lines[self.pos].clone();
        let name = head.text[3..].trim().to_string();
        let body = self.collect_block();
        let mut api = ApiDecl { name, line: head.line_no, ..Default::default() };
        let mut i = 0usize;
        while i < body.len() {
            let ln = &body[i];
            let trimmed = ln.text.trim();
            if trimmed.is_empty() {
                i += 1;
                continue;
            }
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            match parts.first().copied() {
                Some("base") if parts.len() >= 2 => api.base = Some(unquote(parts[1]).unwrap_or_default()),
                Some("auth") if parts.len() >= 2 => api.auth = Some(parts[1].to_string()),
                Some("timeout") if parts.len() >= 2 => api.timeout_ms = parts[1].parse::<u64>().ok(),
                Some("retry") if parts.len() >= 2 => api.retry = parts[1].parse::<u32>().ok(),
                Some("function") => {
                    // naglowek funkcji API; cialo to kolejne linie o wyzszym wcieciu
                    let decl = trimmed.trim_start_matches("function").trim();
                    let (sig, ret) = match decl.split_once("returns" ) {
                        Some((s, r)) => (s.trim(), Some(parse_type(r.trim()))),
                        None => (decl, None),
                    };
                    let fname = sig.split('(').next().unwrap_or("").trim().to_string();
                    let params = parse_params(sig);
                    let header_indent = ln.indent;
                    let mut j = i + 1;
                    let mut route = None;
                    let mut method = None;
                    while j < body.len() && (body[j].text.trim().is_empty() || body[j].indent > header_indent) {
                        let inner = body[j].text.trim();
                        let ip: Vec<&str> = inner.split_whitespace().collect();
                        if ip.len() >= 2 && is_http_verb(ip[0]) {
                            method = Some(ip[0].to_uppercase());
                            route = Some(unquote(ip[1]).unwrap_or_else(|| ip[1].to_string()));
                        }
                        j += 1;
                    }
                    if method.is_none() {
                        self.error_at(ln, ErrorCatalog::Api(format!("funkcja API {} nie deklaruje zadania", fname)), Some("dodaj linie np. get \"/items\" albo post \"/items\"".to_string()));
                    }
                    api.functions.push(ApiFunction {
                        name: fname,
                        params,
                        returns: ret,
                        method,
                        route,
                        line: ln.line_no,
                    });
                    i = j;
                    continue;
                }
                _ => {}
            }
            i += 1;
        }
        if api.base.is_none() {
            self.error_at(&head, ErrorCatalog::Api("api musi deklarowac base \"https://...\""), None);
        }
        program.apis.push(api);
    }

    fn parse_function(&mut self, program: &mut Program) {
        let head = self.lines[self.pos].clone();
        let decl = head.text.trim_start_matches("function").trim();
        let (sig, ret) = match decl.split_once("returns") {
            Some((s, r)) => (s.trim(), Some(parse_type(r.trim()))),
            None => (decl, None),
        };
        let name = sig.split('(').next().unwrap_or("").trim().to_string();
        if name.is_empty() {
            self.error_at(&head, ErrorCatalog::Syntax("funkcja wymaga nazwy"), None);
        }
        let params = parse_params(sig);
        let body_lines = self.collect_block();
        let body = self.parse_block(&body_lines);
        program.functions.push(FunctionDecl { name, params, returns: ret, body, line: head.line_no, is_api: false });
    }

    fn parse_component(&mut self, program: &mut Program) {
        let head = self.lines[self.pos].clone();
        let decl = head.text.trim_start_matches("component").trim();
        let name = decl.split('(').next().unwrap_or("").trim().to_string();
        let params = parse_params(decl);
        let body_lines = self.collect_block();
        let body = self.parse_block(&body_lines);
        if body.is_empty() {
            self.error_at(&head, ErrorCatalog::Layout("komponent nie ma tresci"), None);
        }
        program.components.push(ComponentDecl { name, params, body, line: head.line_no });
    }

    fn parse_screen(&mut self, program: &mut Program) {
        let head = self.lines[self.pos].clone();
        let rest = head.text[6..].trim();
        let name = rest.split('(').next().unwrap_or("").trim().to_string();
        let params = parse_params(rest);
        let mut screen = ScreenDecl { name: name.clone(), params, line: head.line_no, ..Default::default() };

        // najpierw zbieramy naglowek ekranu (title, state, param) i cialo
        let body_lines = self.collect_block();
        let mut content_start = 0usize;
        for (idx, ln) in body_lines.iter().enumerate() {
            let trimmed = ln.text.trim();
            if trimmed.is_empty() {
                continue;
            }
            if let Some(title) = trimmed.strip_prefix("title ") {
                screen.title = unquote(title.trim());
                content_start = idx + 1;
            } else if trimmed.starts_with("state ") {
                if let Some(state) = parse_state_line(ln, &self.type_env) {
                    screen.states.push(state);
                }
                content_start = idx + 1;
            } else if trimmed.starts_with("param ") {
                let p = trimmed.trim_start_matches("param ").trim();
                let parts: Vec<&str> = p.split_whitespace().collect();
                if parts.len() >= 2 {
                    screen.params.push(Param { name: parts[0].to_string(), ty: parse_type(parts[1]) });
                }
                content_start = idx + 1;
            } else {
                break;
            }
        }
        if screen.title.is_none() {
            self.error_at(&head, ErrorCatalog::Layout(format!("ekran {} nie ma tytulu", name)), Some("dodaj linie title \"...\"".to_string()));
        }
        let content_lines = &body_lines[content_start.min(body_lines.len())..];
        screen.body = self.parse_block(content_lines);
        program.screens.push(screen);
    }

    fn parse_test(&mut self, program: &mut Program, kind: TestKind) {
        let head = self.lines[self.pos].clone();
        let name = unquote(collect_quoted(&head.text)).unwrap_or_default();
        let body_lines = self.collect_block();
        let mut steps = Vec::new();
        for ln in &body_lines {
            let trimmed = ln.text.trim();
            if trimmed.is_empty() {
                continue;
            }
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            let keyword = parts.first().copied().unwrap_or("");
            let rest = trimmed[..].trim_start_matches(keyword).trim().to_string();
            let step_kind = match keyword {
                "given" => StepKind::Given,
                "when" => StepKind::When,
                "then" => StepKind::Then,
                "expect" => StepKind::Expect,
                "open" => StepKind::Open,
                "fill" => StepKind::Fill,
                "tap" => StepKind::Tap,
                "type" => StepKind::Type,
                _ => StepKind::Raw,
            };
            steps.push(TestStep { kind: step_kind, text: rest, line: ln.line_no });
        }
        program.tests.push(TestDecl { name, kind, steps, line: head.line_no });
    }

    // --- bloki i wyrazenia ---

    /// Zbiera linie bloku nalezace do biezacego naglowka.
    fn collect_block(&mut self) -> Vec<Line<'a>> {
        let header_indent = if self.pos < self.lines.len() { self.lines[self.pos].indent } else { 0 };
        let mut out = Vec::new();
        let mut i = self.pos + 1;
        // pomijamy naglowek
        self.pos += 1;
        while i < self.lines.len() {
            let ln = &self.lines[i];
            if ln.text.is_empty() {
                i += 1;
                continue;
            }
            if ln.indent <= header_indent {
                break;
            }
            out.push(ln.clone());
            i += 1;
        }
        self.pos = i;
        out
    }

    /// Buduje liste instrukcji z linii bloku.
    fn parse_block(&mut self, lines: &[Line<'a>]) -> Vec<Stmt> {
        let mut out = Vec::new();
        let mut i = 0usize;
        while i < lines.len() {
            let ln = &lines[i];
            let trimmed = ln.text.trim();
            if trimmed.is_empty() {
                i += 1;
                continue;
            }

            // bloki sterujace zbieraja zagniezdzony blok
            if is_block_header(trimmed) {
                let mut nested = Vec::new();
                let mut j = i + 1;
                while j < lines.len() {
                    let inner = &lines[j];
                    if !inner.text.is_empty() && inner.indent <= ln.indent {
                        break;
                    }
                    nested.push(inner.clone());
                    j += 1;
                }
                let inner_stmts = self.parse_block(&nested);
                out.push(Stmt::Block { header: trimmed.to_string(), body: inner_stmts, line: ln.line_no });
                i = j;
                continue;
            }

            out.push(self.parse_stmt(ln));
            i += 1;
        }
        out
    }

    /// Zamienia pojedyncza linie w instrukcje.
    fn parse_stmt(&mut self, ln: &Line<'a>) -> Stmt {
        let trimmed = ln.text.trim();
        let (head, rest) = split_verb(trimmed);
        match head {
            "text" | "button" | "field" | "checkbox" | "image" | "spinner" => {
                let label = unquote(rest).unwrap_or_else(|| rest.to_string());
                Stmt::Widget { kind: head.to_string(), label, props: Vec::new(), line: ln.line_no }
            }
            "list" | "row" | "column" | "grid" | "scroll" | "card" | "spacer" => {
                Stmt::Container { kind: head.to_string(), target: rest.to_string(), props: Vec::new(), body: Vec::new(), line: ln.line_no }
            }
            "set" => Stmt::Set { expr: rest.to_string(), line: ln.line_no },
            "add" => Stmt::Add { expr: rest.to_string(), line: ln.line_no },
            "remove" => Stmt::Remove { expr: rest.to_string(), line: ln.line_no },
            "update" => Stmt::Update { expr: rest.to_string(), line: ln.line_no },
            "save" => Stmt::Save { target: rest.to_string(), line: ln.line_no },
            "load" => Stmt::Load { target: rest.to_string(), line: ln.line_no },
            "clear" => Stmt::Clear { target: rest.to_string(), line: ln.line_no },
            "enqueue" => Stmt::Enqueue { expr: rest.to_string(), line: ln.line_no },
            "flush" => Stmt::Flush { target: rest.to_string(), line: ln.line_no },
            "go" => Stmt::Go { target: rest.to_string(), line: ln.line_no },
            "open" => Stmt::Open { target: rest.to_string(), line: ln.line_no },
            "close" => Stmt::Close { target: rest.to_string(), line: ln.line_no },
            "show" => Stmt::Show { target: rest.to_string(), line: ln.line_no },
            "return" => Stmt::Return { expr: rest.to_string(), line: ln.line_no },
            "announce" => Stmt::Announce { text: rest.to_string(), urgent: false, line: ln.line_no },
            "announceUrgent" => Stmt::Announce { text: rest.to_string(), urgent: true, line: ln.line_no },
            "requestCameraPermission" => Stmt::PermissionRequest { kind: "camera".to_string(), line: ln.line_no },
            "requestMicrophonePermission" => Stmt::PermissionRequest { kind: "microphone".to_string(), line: ln.line_no },
            "requestLocationPermission" => Stmt::PermissionRequest { kind: "location".to_string(), line: ln.line_no },
            "vibrate" => Stmt::Vibrate { pattern: rest.to_string(), line: ln.line_no },
            "player" | "musicPlayer" => Stmt::Call { name: head.to_string(), args: rest.to_string(), line: ln.line_no },
            "key" | "accessibility" | "role" | "state" | "style" | "color" | "alt" | "padding" | "gap" | "width" | "height" | "disabled" | "variant" | "placeholder" | "type" | "error" | "label" | "min length" | "max length" | "align" | "justify" | "columns" | "direction" | "max lines" | "controls" | "subtitles" | "facing" => {
                Stmt::Property { key: format!("{} {}", head, rest).trim().to_string(), line: ln.line_no }
            }
            "playlist" | "queue" => ({
                Stmt::Container { kind: head.to_string(), target: rest.to_string(), props: Vec::new(), body: Vec::new(), line: ln.line_no }
            })(),
            "when" => Stmt::When { event: rest.to_string(), body: Vec::new(), line: ln.line_no },
            "derive" => Stmt::Derive { body: Vec::new(), line: ln.line_no },
            "try" => Stmt::Try { body: Vec::new(), handler: Vec::new(), finally: Vec::new(), line: ln.line_no },
            "catch" => Stmt::CatchMarker { line: ln.line_no },
            "finally" => Stmt::FinallyMarker { line: ln.line_no },
            _ => {
                // wywolanie funkcji albo wyrazenie
                if rest.is_empty() {
                    Stmt::Call { name: head.to_string(), args: String::new(), line: ln.line_no }
                } else {
                    Stmt::Expr { text: trimmed.to_string(), line: ln.line_no }
                }
            }
        }
    }

    // --- diagnostyka ---

    fn error_at(&mut self, line: &Line<'a>, code: ErrorCatalog, suggestion: Option<String>) {
        let mut d = Diagnostics::single(code, line.line_no, line.col);
        if let Some(s) = suggestion {
            d.suggestion = Some(s);
        }
        self.diagnostics.push_diagnostic(d);
    }
}

// --- funkcje pomocnicze ---

#[derive(Debug, Clone, Copy)]
enum AppField {
    Version,
    Language,
    Supports,
    Orientation,
    Offline,
    Entry,
}

/// Buduje liste linii, liczac wciecia na spacji.
fn build_lines(source: &str) -> Vec<Line<'_>> {
    let mut out = Vec::new();
    for (idx, raw) in source.lines().enumerate() {
        let mut indent = 0usize;
        let mut trimmed_start = raw.len();
        for (i, ch) in raw.char_indices() {
            match ch {
                ' ' => indent += 1,
                '\t' => indent += TAB_WIDTH,
                _ => {
                    trimmed_start = i;
                    break;
                }
            }
        }
        let text = raw[trimmed_start.min(raw.len())..].trim_end();
        let text = if let Some(stripped) = text.strip_prefix("//") {
            let _ = stripped;
            ""
        } else {
            text
        };
        out.push(Line { text, indent, line_no: idx + 1, col: indent + 1 });
    }
    out
}

fn first_word(s: &str) -> &str {
    s.split_whitespace().next().unwrap_or("")
}

fn unquote(s: &str) -> Option<String> {
    let t = s.trim();
    if t.len() >= 2 && t.starts_with('"') && t.ends_with('"') {
        Some(t[1..t.len() - 1].to_string())
    } else {
        None
    }
}

fn collect_quoted(s: &str) -> &str {
    if let Some(start) = s.find('"') {
        if let Some(end) = s[start + 1..].find('"') {
            return &s[start..start + end + 2];
        }
    }
    s
}

fn split_pair(s: &str) -> (String, String) {
    match s.split_once(' ') {
        Some((k, v)) => (k.trim().to_string(), v.trim().to_string()),
        None => (s.trim().to_string(), String::new()),
    }
}

fn split_verb(s: &str) -> (&str, &str) {
    match s.split_once(' ') {
        Some((h, r)) => (h.trim(), r.trim()),
        None => (s.trim(), ""),
    }
}

fn is_http_verb(s: &str) -> bool {
    matches!(s, "get" | "post" | "put" | "patch" | "delete")
}

fn is_block_header(s: &str) -> bool {
    let head = first_word(s);
    matches!(
        head,
        "when" | "if" | "else" | "try" | "catch" | "finally" | "for" | "derive" | "queued"
    )
}

fn parse_type(raw: &str) -> TypeRef {
    let t = raw.trim().trim_end_matches('?');
    let optional = raw.trim().ends_with('?');
    let base = if let Some(inner) = t.strip_prefix("List<") {
        TypeRef::List(Box::new(parse_type(inner.trim_end_matches('>'))))
    } else if let Some(inner) = t.strip_prefix("Map<") {
        let parts: Vec<&str> = inner.trim_end_matches('>').split(',').collect();
        let k = parts.first().map(|s| parse_type(s)).unwrap_or(TypeRef::Text);
        let v = parts.get(1).map(|s| parse_type(s)).unwrap_or(TypeRef::Json);
        TypeRef::Map(Box::new(k), Box::new(v))
    } else {
        match t {
            "Text" => TypeRef::Text,
            "Number" => TypeRef::Number,
            "Boolean" => TypeRef::Boolean,
            "Date" => TypeRef::Date,
            "Duration" => TypeRef::Duration,
            "Json" => TypeRef::Json,
            "File" => TypeRef::File,
            "Image" => TypeRef::Image,
            "Video" => TypeRef::Video,
            "Audio" => TypeRef::Audio,
            "Location" => TypeRef::Location,
            "none" => TypeRef::Optional(Box::new(TypeRef::Json)),
            other => TypeRef::Named(other.to_string()),
        }
    };
    if optional {
        TypeRef::Optional(Box::new(base))
    } else {
        base
    }
}

fn is_known_type(ty: &TypeRef, env: &HashMap<String, TypeRef>) -> bool {
    match ty {
        TypeRef::Text
        | TypeRef::Number
        | TypeRef::Boolean
        | TypeRef::Date
        | TypeRef::Duration
        | TypeRef::Json
        | TypeRef::File
        | TypeRef::Image
        | TypeRef::Video
        | TypeRef::Audio
        | TypeRef::Location => true,
        TypeRef::List(inner) | TypeRef::Optional(inner) => is_known_type(inner, env),
        TypeRef::Map(k, v) => is_known_type(k, env) && is_known_type(v, env),
        TypeRef::Named(name) => env.contains_key(name),
    }
}

fn parse_params(sig: &str) -> Vec<Param> {
    let mut out = Vec::new();
    let Some(open) = sig.find('(') else { return out };
    let Some(close) = sig.rfind(')') else { return out };
    if close <= open {
        return out;
    }
    let inner = &sig[open + 1..close];
    for part in inner.split(',') {
        let p = part.trim();
        if p.is_empty() {
            continue;
        }
        let mut it = p.split_whitespace();
        let name = it.next().unwrap_or("").to_string();
        let ty = it.next().unwrap_or("Json").to_string();
        if name.is_empty() {
            continue;
        }
        out.push(Param { name, ty: parse_type(&ty) });
    }
    out
}

fn parse_state_line(ln: &Line<'_>, env: &HashMap<String, TypeRef>) -> Option<StateDecl> {
    let trimmed = ln.text.trim().trim_start_matches("state ").trim();
    let (decl, init) = match trimmed.split_once('=') {
        Some((d, i)) => (d.trim(), Some(i.trim().to_string())),
        None => (trimmed, None),
    };
    let parts: Vec<&str> = decl.split_whitespace().collect();
    if parts.len() < 2 {
        return None;
    }
    let name = parts[0].to_string();
    let raw = parts[1].to_string();
    let colon = raw.find(':');
    let (ty_raw, derived) = match colon {
        Some(idx) => (raw[..idx].to_string(), Some(raw[idx + 1..].to_string())),
        None => (raw.clone(), None),
    };
    let _ = env;
    Some(StateDecl { name, ty: parse_type(&ty_raw), initial: init, derive: derived, line: ln.line_no })
}
