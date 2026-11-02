// src/verifier.rs — analiza semantyczna AXCode
//
// Weryfikator sprawdza spojnosc programu po parsowaniu: istnienie ekranow,
// zgodnosc typow w nawigacji, poprawnosc zapisow do magazynow, obecnosc
// etykiet dostepnosci, uzycie HTTPS w warstwie API oraz reguly bezpieczenstwa.
// Zwraca raport z lista diagnostyk, ktore trafiaja do glownego potoku.

use crate::ast::*;
use crate::diagnostics::{Diagnostic, Diagnostics, ErrorCatalog, Severity};
use std::collections::{HashMap, HashSet};

/// Wynik weryfikacji semantycznej.
pub struct VerifyResult {
    pub diagnostics: Diagnostics,
    pub screen_names: HashSet<String>,
    pub function_names: HashSet<String>,
    pub store_types: HashMap<String, TypeRef>,
    pub data_types: HashMap<String, Vec<FieldDecl>>,
    pub enum_names: HashSet<String>,
}

impl Default for VerifyResult {
    fn default() -> Self {
        Self {
            diagnostics: Diagnostics::new(),
            screen_names: HashSet::new(),
            function_names: HashSet::new(),
            store_types: HashMap::new(),
            data_types: HashMap::new(),
            enum_names: HashSet::new(),
        }
    }
}

/// Glowna funkcja wejscia weryfikatora.
pub fn verify(program: &Program) -> VerifyResult {
    let mut result = VerifyResult::default();
    index_program(program, &mut result);
    check_app_block(program, &mut result);
    check_theme(program, &mut result);
    check_enum_declarations(program, &mut result);
    check_data_declarations(program, &mut result);
    check_store_declarations(program, &mut result);
    check_global_state(program, &mut result);
    check_permissions(program, &mut result);
    check_apis(program, &mut result);
    check_components(program, &mut result);
    check_screens(program, &mut result);
    check_functions(program, &mut result);
    check_tests(program, &mut result);
    check_orphans(program, &mut result);
    result
}

/// Zbiera nazwy ekranow, funkcji, magazynow i modeli do wspolnych map.
fn index_program(program: &Program, result: &mut VerifyResult) {
    for screen in &program.screens {
        if !result.screen_names.insert(screen.name.clone()) {
            result.diagnostics.push_diagnostic(Diagnostic {
                code: ErrorCatalog::Platform(format!("ekran {} jest zadeklarowany wiecej niz raz", screen.name)),
                line: screen.line,
                col: 1,
                severity: Severity::Error,
                message: format!("Duplikat ekranu {}", screen.name),
                suggestion: Some("usun jedna z deklaracji albo zmien nazwe ekranu".to_string()),
                snippet: None,
            });
        }
    }
    for function in &program.functions {
        if !result.function_names.insert(function.name.clone()) {
            result.diagnostics.push_diagnostic(Diagnostic {
                code: ErrorCatalog::Syntax(format!("funkcja {} jest zadeklarowana wiecej niz raz", function.name)),
                line: function.line,
                col: 1,
                severity: Severity::Error,
                message: format!("Duplikat funkcji {}", function.name),
                suggestion: Some("usun jedna z deklaracji albo zmien nazwe funkcji".to_string()),
                snippet: None,
            });
        }
    }
    for store in &program.stores {
        result.store_types.insert(store.name.clone(), store.shape.inner_type());
    }
    for data in &program.data {
        result.data_types.insert(data.name.clone(), data.fields.clone());
    }
    for e in &program.enums {
        result.enum_names.insert(e.name.clone());
    }
}

fn warn(result: &mut VerifyResult, code: ErrorCatalog, line: usize, message: String, suggestion: &str) {
    result.diagnostics.push_diagnostic(Diagnostic {
        code,
        line,
        col: 1,
        severity: Severity::Warning,
        message,
        suggestion: Some(suggestion.to_string()),
        snippet: None,
    });
}

fn error(result: &mut VerifyResult, code: ErrorCatalog, line: usize, message: String, suggestion: &str) {
    result.diagnostics.push_diagnostic(Diagnostic {
        code,
        line,
        col: 1,
        severity: Severity::Error,
        message,
        suggestion: Some(suggestion.to_string()),
        snippet: None,
    });
}

/// Sprawdza kompletowac deklaracji w bloku app.
fn check_app_block(program: &Program, result: &mut VerifyResult) {
    let app = &program.app;
    if app.name.is_none() {
        error(result, ErrorCatalog::Syntax("brak deklaracji app".to_string()), 1, "Plik nie deklaruje aplikacji.".to_string(), "dodaj na poczatku linie: app \"NazwaAplikacji\"");
    }
    if app.version.is_none() {
        warn(result, ErrorCatalog::Syntax("brak wersji aplikacji".to_string()), app.line.max(1), "Brak deklaracji version.".to_string(), "dodaj linie: version \"1.0.0\"");
    }
    if app.supports.is_empty() {
        warn(result, ErrorCatalog::Platform("brak listy platform docelowych".to_string()), app.line.max(1), "Brak deklaracji supports.".to_string(), "dodaj linie: supports ios android web");
    }
    for platform in &app.supports {
        if !matches!(platform.as_str(), "ios" | "android" | "web" | "desktop" | "watchos" | "wearos") {
            error(result, ErrorCatalog::Platform(format!("nieznana platforma docelowa: {}", platform)), app.line.max(1), format!("Platforma {} nie jest obslugiwana przez kompilator.", platform), "uzyj jednej z: ios, android, web, desktop, watchos, wearos");
        }
    }
    if let Some(entry) = &app.entry {
        if !program.screens.iter().any(|s| &s.name == entry) {
            error(result, ErrorCatalog::Platform(format!("ekran startowy {} nie istnieje", entry)), app.line.max(1), format!("Deklaracja entry wskazuje na nieistniejacy ekran {}.", entry), "wpisz nazwe istniejacego ekranu albo dodaj brakujacy ekran");
        }
    }
}

/// Sprawdza, czy motyw deklaruje wszystkie wbudowane tokeny kolorow.
fn check_theme(program: &Program, result: &mut VerifyResult) {
    let required = ["primary", "accent", "danger", "success", "warning", "background", "surface", "text"];
    for theme in &program.themes {
        for token in required {
            if !theme.colors.contains_key(token) {
                warn(result, ErrorCatalog::Layout(format!("motyw {} nie deklaruje tokenu {}", theme.name, token)), theme.line, format!("Brak tokenu koloru {} w motywie {}", token, theme.name), &format!("dodaj linie: color {} #RRGGBB", token));
            }
        }
    }
    if program.themes.is_empty() {
        warn(result, ErrorCatalog::Layout("brak motywu".to_string()), 1, "Program nie deklaruje zadnego motywu.".to_string(), "dodaj blok theme default z tokenami kolorow");
    }
}

fn check_enum_declarations(program: &Program, result: &mut VerifyResult) {
    for decl in &program.enums {
        let mut seen: HashSet<&str> = HashSet::new();
        for variant in &decl.variants {
            if !seen.insert(variant.name.as_str()) {
                error(result, ErrorCatalog::Types(format!("duplikat wariantu {} w enum {}", variant.name, decl.name)), variant.line, format!("Wariant {} wystepuje w enum {} dwa razy.", variant.name, decl.name), "usun powtorzony wariant");
            }
        }
    }
}

fn check_data_declarations(program: &Program, result: &mut VerifyResult) {
    for decl in &program.data {
        let mut names: HashSet<&str> = HashSet::new();
        for field in &decl.fields {
            if !names.insert(field.name.as_str()) {
                error(result, ErrorCatalog::Types(format!("duplikat pola {} w modelu {}", field.name, decl.name)), field.line, format!("Pole {} wystepuje w modelu {} dwa razy.", field.name, decl.name), "usun powtorzone pole");
            }
            check_type_resolvable(&field.ty, field.line, result, "pole modelu", decl.name.as_str());
        }
        if decl.name.chars().next().map(|c| c.is_lowercase()).unwrap_or(false) {
            warn(result, ErrorCatalog::Types(format!("model {} zaczyna sie mala litera", decl.name)), decl.line, format!("Konwencja: nazwy typow piszemy wielka litera ({}).", decl.name), &format!("zmien nazwe na {}", capitalize(&decl.name)));
        }
    }
}

fn check_store_declarations(program: &Program, result: &mut VerifyResult) {
    let mut keys: HashMap<String, usize> = HashMap::new();
    for store in &program.stores {
        if let Some(key) = &store.key {
            *keys.entry(key.clone()).or_insert(0) += 1;
        }
        let lower = store.name.to_lowercase();
        let holds_secret = lower.contains("token") || lower.contains("key") || lower.contains("secret") || lower.contains("auth");
        if holds_secret && !matches!(store.class, StoreClass::Secure) && !store.encryption {
            error(result, ErrorCatalog::Security(format!("magazyn {} przechowuje dane wrazliwe bez szyfrowania", store.name)), store.line, format!("Magazyn {} wyglada na przechowujacy sekrety, ale nie jest klasy secure ani encrypted.", store.name), &format!("zmien deklaracje na: store {} secure", store.name));
        }
        if store.key.is_none() {
            error(result, ErrorCatalog::State(format!("magazyn {} nie ma klucza", store.name)), store.line, format!("Magazyn {} nie deklaruje klucza przechowywania.", store.name), &format!("dodaj linie: key \"app.{}\"", store.name));
        }
        let inner = store.shape.inner_type();
        check_type_resolvable(&inner, store.line, result, "magazyn", store.name.as_str());
        if matches!(store.class, StoreClass::Queue) && !store.encryption {
            warn(result, ErrorCatalog::Security(format!("kolejka {} nie jest szyfrowana", store.name)), store.line, format!("Kolejka offline {} powinna byc szyfrowana, bo przechowuje tresci uzytkownika.", store.name), &format!("zmien deklaracje na: store {} queue encrypted", store.name));
        }
    }
    for (key, count) in keys {
        if count > 1 {
            warn(result, ErrorCatalog::State(format!("klucz magazynu {} uzyty {} razy", key, count)), 1, format!("Klucz {} wystepuje w {} magazynach.", key, count), "nadaj kazdemu magazynowi unikalny klucz");
        }
    }
}

fn check_global_state(program: &Program, result: &mut VerifyResult) {
    let mut names: HashSet<&str> = HashSet::new();
    for g in &program.globals {
        if !names.insert(g.name.as_str()) {
            error(result, ErrorCatalog::State(format!("duplikat stanu globalnego {}", g.name)), g.line, format!("Stan globalny {} jest zadeklarowany dwa razy.", g.name), "usun jedna z deklaracji");
        }
        check_type_resolvable(&g.ty, g.line, result, "stan globalny", g.name.as_str());
        if let (Some(init), ty) = (&g.initial, &g.ty) {
            if !literal_matches(init, ty) {
                error(result, ErrorCatalog::Types(format!("wartosc poczatkowa nie pasuje do typu {}", g.name)), g.line, format!("Stan {} ma niezgodna wartosc poczatkowa ({}).", g.name, init), "popraw wartosc poczatkowa albo typ");
            }
        }
    }
}

fn check_permissions(program: &Program, result: &mut VerifyResult) {
    let known = ["camera", "microphone", "photos", "location", "notifications", "contacts", "bluetooth"];
    let mut seen: HashSet<&str> = HashSet::new();
    for perm in &program.permissions {
        if !known.contains(&perm.kind.as_str()) {
            error(result, ErrorCatalog::Platform(format!("nieznane uprawnienie: {}", perm.kind)), perm.line, format!("Uprawnienie {} nie jest rozpoznawane przez runtime.", perm.kind), &format!("dozwolone uprawnienia: {}", known.join(", ")));
        }
        if perm.purpose.trim().is_empty() {
            error(result, ErrorCatalog::Security(format!("uprawnienie {} nie ma celu", perm.kind)), perm.line, format!("Uprawnienie {} nie opisuje, po co aplikacja go potrzebuje.", perm.kind), "dodaj linie: purpose \"...\"");
        }
        if !seen.insert(perm.kind.as_str()) {
            warn(result, ErrorCatalog::Platform(format!("duplikat uprawnienia {}", perm.kind)), perm.line, format!("Uprawnienie {} zadeklarowane dwa razy.", perm.kind), "polacz deklaracje w jedna");
        }
    }
}

fn check_apis(program: &Program, result: &mut VerifyResult) {
    for api in &program.apis {
        if let Some(base) = &api.base {
            if base.starts_with("http://") {
                error(result, ErrorCatalog::Security(format!("api {} uzywa HTTP zamiast HTTPS", api.name)), api.line, format!("Warstwa {} deklaruje polaczenie nieszyfrowane.", api.name), "zmien adres bazowy na https://");
            }
            if !base.starts_with("https://") && !base.starts_with("http://") {
                error(result, ErrorCatalog::Api(format!("adres bazowy {} nie jest absolutnym URL", api.name)), api.line, format!("Warstwa {} nie ma poprawnego adresu bazowego.", api.name), "uzyj pelnego adresu, np. https://api.example.com");
            }
        }
        match api.timeout_ms {
            Some(t) if t < 1000 => warn(result, ErrorCatalog::Api(format!("timeout {} ms jest zbyt krotki", t)), api.line, format!("Warstwa {} moze zrywac poprawne zadania.", api.name), "ustaw timeout na co najmniej 5000 ms"),
            None => warn(result, ErrorCatalog::Api(format!("api {} nie deklaruje timeoutu", api.name)), api.line, format!("Warstwa {} nie zabezpiecza zadan limitem czasu.", api.name), "dodaj linie: timeout 15000"),
            _ => {}
        }
        if api.retry.is_none() {
            warn(result, ErrorCatalog::Api(format!("api {} nie deklaruje ponowien", api.name)), api.line, format!("Warstwa {} nie ma polityki ponowien.", api.name), "dodaj linie: retry 3");
        }
        let mut fnames: HashSet<&str> = HashSet::new();
        for f in &api.functions {
            if !fnames.insert(f.name.as_str()) {
                error(result, ErrorCatalog::Api(format!("duplikat funkcji API {}.{}", api.name, f.name)), f.line, format!("Funkcja {}.{} zadeklarowana dwa razy.", api.name, f.name), "zmien nazwe albo usun duplikat");
            }
            if f.method.is_none() {
                error(result, ErrorCatalog::Api(format!("funkcja API {}.{} nie ma zadania", api.name, f.name)), f.line, format!("Funkcja {}.{} nie deklaruje metody HTTP.", api.name, f.name), "dodaj linie: get \"/sciezka\"");
            }
            if let Some(ret) = &f.returns {
                check_type_resolvable(ret, f.line, result, "typ zwracany API", f.name.as_str());
            }
            for p in &f.params {
                check_type_resolvable(&p.ty, f.line, result, "parametr API", f.name.as_str());
            }
        }
    }
}

fn check_components(program: &Program, result: &mut VerifyResult) {
    let mut names: HashSet<&str> = HashSet::new();
    for comp in &program.components {
        if !names.insert(comp.name.as_str()) {
            error(result, ErrorCatalog::Layout(format!("duplikat komponentu {}", comp.name)), comp.line, format!("Komponent {} zadeklarowany dwa razy.", comp.name), "zmien nazwe albo usun duplikat");
        }
        let mut pnames: HashSet<&str> = HashSet::new();
        for p in &comp.params {
            if !pnames.insert(p.name.as_str()) {
                error(result, ErrorCatalog::Layout(format!("duplikat parametru {} w komponencie {}", p.name, comp.name)), comp.line, format!("Parametr {} wystepuje w komponencie {} dwa razy.", p.name, comp.name), "usun powtorzony parametr");
            }
        }
        check_accessibility(&comp.name, "komponent", &comp.body, result);
        check_control_flow(&comp.body, result);
    }
}

fn check_screens(program: &Program, result: &mut VerifyResult) {
    for screen in &program.screens {
        let mut snames: HashSet<&str> = HashSet::new();
        for st in &screen.states {
            if !snames.insert(st.name.as_str()) {
                error(result, ErrorCatalog::State(format!("duplikat stanu {} na ekranie {}", st.name, screen.name)), st.line, format!("Stan {} na ekranie {} jest zadeklarowany dwa razy.", st.name, screen.name), "usun jedna z deklaracji");
            }
            check_type_resolvable(&st.ty, st.line, result, "stan ekranu", st.name.as_str());
            if let (Some(init), ty) = (&st.initial, &st.ty) {
                if !literal_matches(init, ty) {
                    error(result, ErrorCatalog::Types(format!("wartosc poczatkowa stanu {} nie pasuje do typu", st.name)), st.line, format!("Stan {} ma niezgodna wartosc poczatkowa ({}).", st.name, init), "popraw wartosc poczatkowa albo typ");
                }
            }
        }
        check_accessibility(&screen.name, "ekran", &screen.body, result);
        check_navigation(&screen.body, screen, program, result);
        check_store_usage(&screen.body, result);
    }
}

fn check_functions(program: &Program, result: &mut VerifyResult) {
    for f in &program.functions {
        let mut pnames: HashSet<&str> = HashSet::new();
        for p in &f.params {
            if !pnames.insert(p.name.as_str()) {
                error(result, ErrorCatalog::Syntax(format!("duplikat parametru {} w funkcji {}", p.name, f.name)), f.line, format!("Parametr {} wystepuje w funkcji {} dwa razy.", p.name, f.name), "usun powtorzony parametr");
            }
            check_type_resolvable(&p.ty, f.line, result, "parametr funkcji", f.name.as_str());
        }
        if let Some(ret) = &f.returns {
            check_type_resolvable(ret, f.line, result, "typ zwracany", f.name.as_str());
        }
        check_control_flow(&f.body, result);
    }
}

fn check_tests(program: &Program, result: &mut VerifyResult) {
    for t in &program.tests {
        if t.name.is_empty() {
            error(result, ErrorCatalog::Syntax("test bez nazwy".to_string()), t.line, "Kazdy test musi miec nazwe w cudzyslowie.".to_string(), "dodaj nazwe, np. test \"Morse encoder\"");
        }
        if t.steps.is_empty() {
            warn(result, ErrorCatalog::Syntax(format!("test {} nie ma krokow", t.name)), t.line, format!("Test {} nie sprawdza niczego.", t.name), "dodaj krok given, when i then");
        }
        if t.kind == TestKind::Ui {
            let opens_known = t.steps.iter().any(|st| {
                st.kind == StepKind::Open && program.screens.iter().any(|s| st.text.contains(&s.name))
            });
            if !opens_known {
                warn(result, ErrorCatalog::Platform(format!("test interfejsu {} nie otwiera znanego ekranu", t.name)), t.line, format!("Test {} nie odwoluje sie do zadeklarowanego ekranu.", t.name), "dodaj krok open <NazwaEkranu>");
            }
        }
        if t.kind == TestKind::Accessibility {
            let has_expect = t.steps.iter().any(|st| st.kind == StepKind::Expect);
            if !has_expect {
                warn(result, ErrorCatalog::Accessibility(format!("test dostepnosci {} nie ma krokow expect", t.name)), t.line, format!("Test {} nie weryfikuje zadnego warunku.", t.name), "dodaj kroki expect, np. expect all fields have labels");
            }
        }
    }
    let has_a11y_test = program.tests.iter().any(|t| t.kind == TestKind::Accessibility);
    if !has_a11y_test && !program.screens.is_empty() {
        warn(result, ErrorCatalog::Accessibility("brak testu dostepnosci".to_string()), 1, "Program deklaruje ekrany, ale nie ma zadnego testu dostepnosci.".to_string(), "dodaj blok accessibility test z krokami expect");
    }
}

/// Sprawdza, czy kazdy ekran jest osiagalny z jakiegos miejsca.
fn check_orphans(program: &Program, result: &mut VerifyResult) {
    if program.screens.len() < 2 {
        return;
    }
    let all_body_text = collect_all_statement_text(program);
    for screen in &program.screens {
        let reachable = all_body_text.contains(&format!("go to {}", screen.name))
            || all_body_text.contains(&format!("go to {} (", screen.name))
            || all_body_text.contains(&format!("open {}", screen.name))
            || program.app.entry.as_deref() == Some(screen.name.as_str());
        if !reachable {
            warn(result, ErrorCatalog::Platform(format!("ekran {} jest nieosiagalny", screen.name)), screen.line, format!("Zaden inny ekran nie nawiguje do {}. To moze byc martwa sciezka.", screen.name), &format!("dodaj gdzies: go to {}", screen.name));
        }
    }
}

// --- reguly pomocnicze ---

/// Reguly dostepnosci: obraz bez alt, pole bez label, przycisk ikonowy bez etykiety.
fn check_accessibility(owner: &str, kind: &str, body: &[Stmt], result: &mut VerifyResult) {
    for stmt in body {
        match stmt {
            Stmt::Widget { kind: wkind, props, line, .. } => {
                let has_alt = props.iter().any(|p| p.key.starts_with("alt"));
                let has_label = props.iter().any(|p| p.key.starts_with("label") || p.key.starts_with("accessibility"));
                if wkind == "image" && !has_alt {
                    error(result, ErrorCatalog::Accessibility(format!("obraz bez alt w {}", owner)), *line, format!("Obraz w {} ({}) nie ma tekstu alternatywnego.", kind, owner), "dodaj linie: alt \"opis obrazu\"");
                }
                if wkind == "field" && !has_label {
                    error(result, ErrorCatalog::Accessibility(format!("pole bez etykiety w {}", owner)), *line, format!("Pole tekstowe w {} ({}) nie ma etykiety.", kind, owner), "dodaj linie: label \"Nazwa pola\"");
                }
                if wkind == "button" && !has_label && looks_like_icon(props) {
                    error(result, ErrorCatalog::Accessibility(format!("przycisk ikonowy bez etykiety w {}", owner)), *line, format!("Przycisk ikonowy w {} ({}) nie ma etykiety dostepnosci.", kind, owner), "dodaj linie: accessibility label \"Opis akcji\"");
                }
            }
            Stmt::Container { body: inner, .. }
            | Stmt::Block { body: inner, .. }
            | Stmt::When { body: inner, .. }
            | Stmt::Derive { body: inner, .. } => check_accessibility(owner, kind, inner, result),
            Stmt::Try { body: inner, handler, finally, .. } => {
                check_accessibility(owner, kind, inner, result);
                check_accessibility(owner, kind, handler, result);
                check_accessibility(owner, kind, finally, result);
            }
            _ => {}
        }
    }
}

/// Sprawdza, czy kazde `when` obsluguje realne zdarzenie, a `try` ma `catch`.
fn check_control_flow(body: &[Stmt], result: &mut VerifyResult) {
    for stmt in body {
        match stmt {
            Stmt::When { event, body: inner, line } => {
                if !is_known_event(event) {
                    error(result, ErrorCatalog::Layout(format!("nieznane zdarzenie: {}", event)), *line, format!("Zdarzenie {} nie jest rozpoznawane.", event), "uzyj jednego z: tapped, changed, screen appears, screen disappears, app launches, valid, invalid, error, online, offline, permission granted, permission denied");
                }
                if inner.is_empty() {
                    warn(result, ErrorCatalog::Layout(format!("zdarzenie {} nie ma obslugi", event)), *line, format!("Blok when {} jest pusty.", event), "dodaj instrukcje albo usun pusty blok");
                }
                check_control_flow(inner, result);
            }
            Stmt::Try { handler, body: inner, finally, line } => {
                if handler.is_empty() {
                    error(result, ErrorCatalog::Api("blok try bez catch".to_string()), *line, "Operacja sieciowa bez obslugi bledu moze cicho zginac.".to_string(), "dodaj blok catch error z obsluga");
                }
                check_control_flow(inner, result);
                check_control_flow(handler, result);
                check_control_flow(finally, result);
            }
            Stmt::Container { body: inner, .. }
            | Stmt::Block { body: inner, .. }
            | Stmt::Derive { body: inner, .. } => check_control_flow(inner, result),
            _ => {}
        }
    }
}

/// Sprawdza, czy cel nawigacji istnieje i czy parametry sie zgadzaja.
fn check_navigation(body: &[Stmt], screen: &ScreenDecl, program: &Program, result: &mut VerifyResult) {
    for stmt in body {
        match stmt {
            Stmt::Go { target, line } | Stmt::Open { target, line } => {
                let name = target.split('(').next().unwrap_or("").trim();
                if name == "back" || name == "modal" || name.is_empty() {
                    continue;
                }
                if !program.screens.iter().any(|s| s.name == name) {
                    error(result, ErrorCatalog::Platform(format!("nawigacja do nieistniejacego ekranu {}", name)), *line, format!("Ekran {} probuje przejsc do ekranu {}, ktory nie istnieje.", screen.name, name), &format!("dodaj blok screen {} albo popraw nazwe", name));
                    continue;
                }
                if let Some(target_screen) = program.screens.iter().find(|s| s.name == name) {
                    let expected = target_screen.params.len();
                    let given = if target.contains('(') {
                        target
                            .split_once('(')
                            .map(|(_, rest)| rest.trim_end_matches(')'))
                            .map(|args| args.split(',').filter(|s| !s.trim().is_empty()).count())
                            .unwrap_or(0)
                    } else {
                        0
                    };
                    if expected != given {
                        error(result, ErrorCatalog::Types(format!("zla liczba parametrow nawigacji do {}", name)), *line, format!("Ekran {} oczekuje {} parametrow, a przekazano {}.", name, expected, given), &format!("przekaz dokladnie {} wartosci", expected));
                    }
                }
            }
            Stmt::Container { body: inner, .. }
            | Stmt::Block { body: inner, .. }
            | Stmt::When { body: inner, .. }
            | Stmt::Derive { body: inner, .. } => check_navigation(inner, screen, program, result),
            Stmt::Try { body: inner, handler, finally, .. } => {
                check_navigation(inner, screen, program, result);
                check_navigation(handler, screen, program, result);
                check_navigation(finally, screen, program, result);
            }
            _ => {}
        }
    }
}

/// Sprawdza operacje na magazynach: nazwa musi istniec, chyba ze to kolejka systemowa.
fn check_store_usage(body: &[Stmt], result: &mut VerifyResult) {
    for stmt in body {
        match stmt {
            Stmt::Save { target, line } | Stmt::Load { target, line } | Stmt::Clear { target, line } => {
                let name = target.split_whitespace().next().unwrap_or("").trim();
                if name.is_empty() {
                    error(result, ErrorCatalog::State("operacja na magazynie bez nazwy".to_string()), *line, "Operacja nie wskazuje magazynu.".to_string(), "podaj nazwe magazynu, np. save messages");
                }
            }
            Stmt::Add { expr, line } | Stmt::Remove { expr, line } => {
                if !expr.contains(" to ") && !expr.contains(" from ") {
                    warn(result, ErrorCatalog::State("operacja na liscie bez celu".to_string()), *line, format!("Instrukcja '{}' nie wskazuje magazynu docelowego.", expr), "uzyj formy: add element to magazyn");
                }
            }
            Stmt::Container { body: inner, .. }
            | Stmt::Block { body: inner, .. }
            | Stmt::When { body: inner, .. }
            | Stmt::Derive { body: inner, .. } => check_store_usage(inner, result),
            Stmt::Try { body: inner, handler, finally, .. } => {
                check_store_usage(inner, result);
                check_store_usage(handler, result);
                check_store_usage(finally, result);
            }
            _ => {}
        }
    }
}

/// Sprawdza, czy typ da sie rozwiazac.
fn check_type_resolvable(ty: &TypeRef, line: usize, result: &mut VerifyResult, context: &str, owner: &str) {
    if let TypeRef::Named(name) = ty {
        let known = result.data_types.contains_key(name)
            || result.enum_names.contains(name)
            || matches!(name.as_str(), "OutboxItem" | "DeviceKey" | "MediaRef" | "Account" | "Session");
        if !known {
            error(result, ErrorCatalog::Types(format!("nieznany typ {} w {}", name, context)), line, format!("Typ {} uzyty w {} ({}) nie jest zadeklarowany.", name, context, owner), &format!("dodaj blok data {} z polami", name));
        }
    }
    match ty {
        TypeRef::List(inner) | TypeRef::Optional(inner) => check_type_resolvable(inner, line, result, context, owner),
        TypeRef::Map(k, v) => {
            check_type_resolvable(k, line, result, context, owner);
            check_type_resolvable(v, line, result, context, owner);
        }
        _ => {}
    }
}

/// Sprawdza, czy literal pasuje do typu.
fn literal_matches(literal: &str, ty: &TypeRef) -> bool {
    let l = literal.trim();
    if l == "none" || l == "null" {
        return true;
    }
    if l == "true" || l == "false" {
        return matches!(ty, TypeRef::Boolean | TypeRef::Optional(_));
    }
    if l.starts_with('"') {
        return matches!(ty, TypeRef::Text | TypeRef::Optional(_) | TypeRef::Json);
    }
    if l.parse::<f64>().is_ok() {
        return matches!(ty, TypeRef::Number | TypeRef::Duration | TypeRef::Optional(_));
    }
    if l.starts_with('[') || l.starts_with('{') {
        return matches!(ty, TypeRef::List(_) | TypeRef::Map(_, _) | TypeRef::Json | TypeRef::Optional(_));
    }
    // wyrazenia zlozone i wywolania funkcji: sprawdzenie miekkie
    true
}

/// Rozpoznaje przycisk ikonowy po tym, ze jego tresc nie jest tekstem.
fn looks_like_icon(props: &[StyleProp]) -> bool {
    props.iter().any(|p| p.key.starts_with("icon") || p.value.starts_with("icon"))
}

/// Lista znanych zdarzen jezyka.
fn is_known_event(event: &str) -> bool {
    let head = event.split_whitespace().next().unwrap_or("");
    matches!(
        head,
        "tapped" | "changed" | "screen" | "app" | "valid" | "invalid" | "error" | "online" | "offline" | "permission" | "value" | "swiped" | "submitted" | "loaded"
    )
}

/// Zbiera tresc wszystkich instrukcji w programie do jednego tekstu.
fn collect_all_statement_text(program: &Program) -> String {
    let mut out = String::new();
    for screen in &program.screens {
        collect_text(&screen.body, &mut out);
    }
    for comp in &program.components {
        collect_text(&comp.body, &mut out);
    }
    for f in &program.functions {
        collect_text(&f.body, &mut out);
    }
    out
}

fn collect_text(body: &[Stmt], out: &mut String) {
    for stmt in body {
        match stmt {
            Stmt::Go { target, .. } | Stmt::Open { target, .. } => {
                out.push_str(" go to ");
                out.push_str(target);
                out.push(' ');
            }
            Stmt::Container { body: inner, .. }
            | Stmt::Block { body: inner, .. }
            | Stmt::When { body: inner, .. }
            | Stmt::Derive { body: inner, .. } => collect_text(inner, out),
            Stmt::Try { body: inner, handler, finally, .. } => {
                collect_text(inner, out);
                collect_text(handler, out);
                collect_text(finally, out);
            }
            _ => {}
        }
    }
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}
