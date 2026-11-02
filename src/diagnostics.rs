// // src/diagnostics.rs — system diagnostyki
//
// Kazdy komunikat ma identyfikator z zakresu AX1000-AX9999, kategorie, pozycje
// w pliku, tresc problemu i proponowana poprawe. Format jest zgodny ze
// specyfikacja jezyka: blad ma uczyc, a nie tylko przerywac budowanie.

use crate::ast::Span;
use colored::*;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Waga komunikatu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Severity {
    /// Uwaga porzadkowa, nie blokuje kompilacji.
    Hint,
    /// Mozliwy problem, nie blokuje kompilacji.
    Warning,
    /// Blad, blokuje kompilacje.
    Error,
}

impl Severity {
    pub fn label(&self) -> &'static str {
        match self {
            Severity::Hint => "uwaga",
            Severity::Warning => "ostrzezenie",
            Severity::Error => "blad",
        }
    }

    pub fn colored_label(&self) -> ColoredString {
        match self {
            Severity::Hint => "uwaga".bright_blue(),
            Severity::Warning => "ostrzezenie".yellow(),
            Severity::Error => "blad".red().bold(),
        }
    }

    pub fn symbol(&self) -> &'static str {
        match self {
            Severity::Hint => "i",
            Severity::Warning => "!",
            Severity::Error => "x",
        }
    }
}

/// Klasa bledu wyznaczona przez zakres kodu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorClass {
    Syntax,
    Types,
    Interface,
    State,
    Api,
    Security,
    Accessibility,
    Platform,
    Performance,
    Unknown,
}

impl ErrorClass {
    /// Wyznacza klase na podstawie kodu numerycznego.
    pub fn from_code(code: u32) -> ErrorClass {
        match code {
            1000..=1999 => ErrorClass::Syntax,
            2000..=2999 => ErrorClass::Types,
            3000..=3999 => ErrorClass::Interface,
            4000..=4999 => ErrorClass::State,
            5000..=5999 => ErrorClass::Api,
            6000..=6999 => ErrorClass::Security,
            7000..=7999 => ErrorClass::Accessibility,
            8000..=8999 => ErrorClass::Platform,
            9000..=9999 => ErrorClass::Performance,
            _ => ErrorClass::Unknown,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            ErrorClass::Syntax => "skladnia",
            ErrorClass::Types => "typy",
            ErrorClass::Interface => "interfejs i layout",
            ErrorClass::State => "stan i magazyny",
            ErrorClass::Api => "API i siec",
            ErrorClass::Security => "bezpieczenstwo",
            ErrorClass::Accessibility => "dostepnosc",
            ErrorClass::Platform => "platforma i nawigacja",
            ErrorClass::Performance => "wydajnosc",
            ErrorClass::Unknown => "nieznana",
        }
    }

    pub fn range(&self) -> &'static str {
        match self {
            ErrorClass::Syntax => "AX1000-AX1999",
            ErrorClass::Types => "AX2000-AX2999",
            ErrorClass::Interface => "AX3000-AX3999",
            ErrorClass::State => "AX4000-AX4999",
            ErrorClass::Api => "AX5000-AX5999",
            ErrorClass::Security => "AX6000-AX6999",
            ErrorClass::Accessibility => "AX7000-AX7999",
            ErrorClass::Platform => "AX8000-AX8999",
            ErrorClass::Performance => "AX9000-AX9999",
            ErrorClass::Unknown => "AX0000",
        }
    }
}

/// Pojedynczy komunikat diagnostyczny.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    pub code: String,
    pub numeric_code: u32,
    pub class: ErrorClass,
    pub severity: Severity,
    pub message: String,
    pub fix: Option<String>,
    pub span: Option<Span>,
    /// Fragment kodu zrodlowego dla kontekstu.
    pub snippet: Option<String>,
}

impl Diagnostic {
    pub fn error(code: u32, message: String, fix: Option<String>, span: Option<Span>) -> Self {
        Diagnostic {
            code: format!("AX{}", code),
            numeric_code: code,
            class: ErrorClass::from_code(code),
            severity: Severity::Error,
            message,
            fix,
            span,
            snippet: None,
        }
    }

    pub fn warning(code: u32, message: String, fix: Option<String>, span: Option<Span>) -> Self {
        Diagnostic {
            code: format!("AX{}", code),
            numeric_code: code,
            class: ErrorClass::from_code(code),
            severity: Severity::Warning,
            message,
            fix,
            span,
            snippet: None,
        }
    }

    pub fn hint(code: u32, message: String, fix: Option<String>, span: Option<Span>) -> Self {
        Diagnostic {
            code: format!("AX{}", code),
            numeric_code: code,
            class: ErrorClass::from_code(code),
            severity: Severity::Hint,
            message,
            fix,
            span,
            snippet: None,
        }
    }

    /// Dolacza fragment zrodla dla kontekstu komunikatu.
    pub fn with_snippet(mut self, source: &str) -> Self {
        if let Some(span) = self.span {
            if span.line > 0 {
                let lines: Vec<&str> = source.split('\n').collect();
                if span.line <= lines.len() {
                    self.snippet = Some(lines[span.line - 1].to_string());
                }
            }
        }
        self
    }

    /// Formatuje komunikat w stylu kompilatora: kod, kategoria, pozycja, wskaznik.
    pub fn format_pretty(&self) -> String {
        let mut out = String::new();

        out.push_str(&format!(
            "{} {} {}\n",
            format!("[{}]", self.code).bright_red().bold(),
            self.class.name().bright_cyan(),
            self.severity.colored_label()
        ));

        out.push_str(&format!("  {}\n", self.message));

        if let (Some(span), Some(snippet)) = (&self.span, &self.snippet) {
            if span.line > 0 {
                out.push_str(&format!(
                    "\n  {}:{}:{}\n",
                    "-->".bright_blue(),
                    span.line,
                    span.column + 1
                ));
                out.push_str(&format!("   {} {}\n", "|".bright_blue(), snippet.trim_end()));
                out.push_str(&format!(
                    "   {} {}{}\n",
                    "|".bright_blue(),
                    " ".repeat(span.column),
                    "^".repeat(span.length.max(1)).bright_red().bold()
                ));
            }
        } else if let Some(span) = &self.span {
            if span.line > 0 {
                out.push_str(&format!(
                    "  {} linia {}, kolumna {}\n",
                    "w".bright_blue(),
                    span.line,
                    span.column + 1
                ));
            }
        }

        if let Some(fix) = &self.fix {
            out.push_str(&format!("\n  {} {}\n", "poprawka:".green().bold(), fix));
        }

        out
    }

    /// Format jednolinijkowy dla raportow zbiorczych.
    pub fn format_short(&self) -> String {
        let position = self
            .span
            .as_ref()
            .filter(|s| s.line > 0)
            .map(|s| format!("{}:{} ", s.line, s.column + 1))
            .unwrap_or_default();
        format!(
            "{} {}: {}{}",
            self.code,
            self.severity.label(),
            position,
            self.message
        )
    }

    /// Format JSON.
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "code": self.code,
            "numeric": self.numeric_code,
            "class": self.class.name(),
            "range": self.class.range(),
            "severity": self.severity.label(),
            "message": self.message,
            "fix": self.fix,
            "line": self.span.as_ref().map(|s| s.line),
            "column": self.span.as_ref().map(|s| s.column + 1),
            "length": self.span.as_ref().map(|s| s.length),
        })
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.format_pretty())
    }
}

/// Zbior komunikatow z podsumowaniem.
#[derive(Debug, Default, Clone)]
pub struct Diagnostics {
    pub items: Vec<Diagnostic>,
}

impl Diagnostics {
    pub fn new() -> Self {
        Diagnostics { items: Vec::new() }
    }

    pub fn push(&mut self, diagnostic: Diagnostic) {
        self.items.push(diagnostic);
    }

    pub fn error(&mut self, code: u32, message: impl Into<String>, fix: Option<String>, span: Option<Span>) {
        self.items.push(Diagnostic::error(code, message.into(), fix, span));
    }

    pub fn warning(&mut self, code: u32, message: impl Into<String>, fix: Option<String>, span: Option<Span>) {
        self.items.push(Diagnostic::warning(code, message.into(), fix, span));
    }

    pub fn hint(&mut self, code: u32, message: impl Into<String>, fix: Option<String>, span: Option<Span>) {
        self.items.push(Diagnostic::hint(code, message.into(), fix, span));
    }

    pub fn errors(&self) -> Vec<&Diagnostic> {
        self.items
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .collect()
    }

    pub fn warnings(&self) -> Vec<&Diagnostic> {
        self.items
            .iter()
            .filter(|d| d.severity == Severity::Warning)
            .collect()
    }

    pub fn hints(&self) -> Vec<&Diagnostic> {
        self.items
            .iter()
            .filter(|d| d.severity == Severity::Hint)
            .collect()
    }

    pub fn error_count(&self) -> usize {
        self.errors().len()
    }

    pub fn warning_count(&self) -> usize {
        self.warnings().len()
    }

    pub fn hint_count(&self) -> usize {
        self.hints().len()
    }

    pub fn has_errors(&self) -> bool {
        self.error_count() > 0
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Grupuje komunikaty wedlug klasy bledu.
    pub fn by_class(&self) -> Vec<(ErrorClass, Vec<&Diagnostic>)> {
        let mut groups: Vec<(ErrorClass, Vec<&Diagnostic>)> = Vec::new();
        let classes = [
            ErrorClass::Syntax,
            ErrorClass::Types,
            ErrorClass::Interface,
            ErrorClass::State,
            ErrorClass::Api,
            ErrorClass::Security,
            ErrorClass::Accessibility,
            ErrorClass::Platform,
            ErrorClass::Performance,
            ErrorClass::Unknown,
        ];
        for class in classes {
            let items: Vec<&Diagnostic> =
                self.items.iter().filter(|d| d.class == class).collect();
            if !items.is_empty() {
                groups.push((class, items));
            }
        }
        groups
    }

    /// Sortuje komunikaty po numerze linii.
    pub fn sorted(&self) -> Vec<&Diagnostic> {
        let mut items: Vec<&Diagnostic> = self.items.iter().collect();
        items.sort_by_key(|d| d.span.as_ref().map(|s| s.line).unwrap_or(0));
        items
    }

    /// Dolacza fragmenty zrodla do wszystkich komunikatow.
    pub fn attach_snippets(&mut self, source: &str) {
        for item in &mut self.items {
            if let Some(span) = item.span {
                if span.line > 0 {
                    let lines: Vec<&str> = source.split('\n').collect();
                    if span.line <= lines.len() {
                        item.snippet = Some(lines[span.line - 1].to_string());
                    }
                }
            }
        }
    }

    /// Podsumowanie liczbowe.
    pub fn summary(&self) -> String {
        if self.is_empty() {
            return "brak komunikatow".green().bold().to_string();
        }
        format!(
            "{} bledow, {} ostrzezen, {} uwag",
            self.error_count(),
            self.warning_count(),
            self.hint_count()
        )
    }
}

/// Rejestr wszystkich kodow bledow z opisem. Uzywany przez polecenie info
/// i przez dokumentacje generowana z kodu.
pub struct ErrorCatalog;

impl ErrorCatalog {
    pub fn all() -> Vec<(u32, &'static str, &'static str, &'static str)> {
        vec![
            (1001, "Skladnia", "Nieznana jednostka programu na poziomie zerowego wciecia", "Uzyj jednej z trzynastu jednostek jezyka"),
            (1002, "Skladnia", "Wciecie nie jest wielokrotnoscia dwoch spacji", "Popraw wciecie w tej linii"),
            (1003, "Skladnia", "Skok wciecia wiekszy niz dwa poziomy", "Zmniejsz wciecie albo dodaj poziom posredni"),
            (1004, "Skladnia", "Nieznana jednostka programu", "Sprawdz nazwe jednostki"),
            (1005, "Skladnia", "Linia poza blokiem jednostki", "Przypisz linie do bloku"),
            (1006, "Skladnia", "Nierozpoznane zdarzenie", "Sprawdz nazwe zdarzenia"),
            (1007, "Skladnia", "Nierozpoznana instrukcja", "Sprawdz skladnie instrukcji"),
            (2001, "Typy", "Pole modelu uzywa nieznanego typu", "Uzyj typu wbudowanego albo zadeklaruj data lub enum"),
            (2002, "Typy", "Pole modelu nie ma typu", "Dodaj typ po nazwie pola"),
            (2003, "Typy", "Niezgodnosc typow w przypisaniu", "Dostosuj typ wartosci do typu zmiennej"),
            (2004, "Typy", "Nieznany typ w parametrze funkcji", "Popraw typ parametru"),
            (2005, "Typy", "Wywolanie funkcji z nieprawidlowa liczba argumentow", "Dostosuj liczbe argumentow do deklaracji"),
            (3001, "Interfejs", "Odwolanie do nieistniejacego stylu", "Dodaj styl albo popraw nazwe"),
            (3002, "Interfejs", "Kontener bez zawartosci", "Dodaj elementy albo usun kontener"),
            (3003, "Interfejs", "Siatka bez liczby kolumn", "Dodaj akcesorium columns z liczba kolumn"),
            (4001, "Stan", "Nieprawidlowa deklaracja stanu", "Poprawna forma: state nazwa Typ = wartosc"),
            (4002, "Stan", "Operacja na niezadeklarowanym magazynie", "Zadeklaruj magazyn skladnia store nazwa klasa"),
            (4003, "Stan", "Nieznana klasa magazynu", "Dozwolone klasy: local, synced, encrypted, secure, cache, queue"),
            (4004, "Stan", "Magazyn bez klucza", "Dodaj klucz wersjonowany, na przyklad key app.dane.v1"),
            (5001, "API", "Brak limitu czasu w deklaracji API", "Dodaj timeout 20000"),
            (5002, "API", "Brak ponawiania w deklaracji API", "Dodaj retry 3"),
            (5003, "API", "Endpoint bez sciezki", "Dodaj metode i sciezke, na przyklad get /lista"),
            (5004, "API", "Adres bazowy bez protokolu HTTPS", "Uzyj https zamiast http"),
            (6001, "Bezpieczenstwo", "Magazyn na sekrety bez klasy secure", "Zmien na: store nazwa secure"),
            (6002, "Bezpieczenstwo", "Klucz prywatny w magazynie jawnym", "Przenies do magazynu klasy secure"),
            (6003, "Bezpieczenstwo", "Logowanie danych wrazliwych", "Usun logowanie tokenow i hasel"),
            (6004, "Bezpieczenstwo", "Wysylanie klucza API do backendu", "Klucz musi pozostac na urzadzeniu"),
            (7001, "Dostepnosc", "Obraz bez tekstu alternatywnego", "Dodaj alt z opisem obrazu"),
            (7002, "Dostepnosc", "Przycisk ikonowy bez etykiety dostepnosci", "Dodaj accessibility label z opisem akcji"),
            (7003, "Dostepnosc", "Pole bez etykiety", "Dodaj label z nazwa pola"),
            (7004, "Dostepnosc", "Element interaktywny bez roli", "Dodaj role opisujaca przeznaczenie"),
            (7005, "Dostepnosc", "Animacja bez respektowania ograniczenia ruchu", "Dodaj warunek ograniczenia ruchu"),
            (8001, "Platforma", "Nawigacja do nieistniejacego ekranu", "Popraw nazwe ekranu albo dodaj brakujacy ekran"),
            (8002, "Platforma", "Uprawnienie bez uzasadnienia", "Dodaj purpose z wyjasnieniem celu"),
            (8003, "Platforma", "Ekran startowy nie istnieje", "Popraw pole entry albo dodaj brakujacy ekran"),
            (9001, "Wydajnosc", "Lista bez pustego stanu", "Dodaj galaz empty z komunikatem"),
            (9002, "Wydajnosc", "Zbyt wiele elementow w jednym ekranie", "Podziel ekran na mniejsze"),
            (9003, "Wydajnosc", "Magazyn bez limitu rozmiaru", "Rozwaz klase cache albo limit wpisow"),
        ]
    }

    pub fn find(code: u32) -> Option<&'static str> {
        Self::all()
            .into_iter()
            .find(|(c, _, _, _)| *c == code)
            .map(|(_, _, msg, _)| msg)
    }

    pub fn count() -> usize {
        Self::all().len()
    }
}/diagnostics.rs — system diagnostyki
//
// Kazdy komunikat ma identyfikator z zakresu AX1000-AX9999, kategorie, pozycje
// w pliku, tresc problemu i proponowana poprawe. Format jest zgodny ze
// specyfikacja jezyka: blad ma uczyc, a nie tylko przerywac budowanie.

use crate::ast::Span;
use colored::*;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Waga komunikatu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Severity {
    /// Uwaga porzadkowa, nie blokuje kompilacji.
    Hint,
    /// Mozliwy problem, nie blokuje kompilacji.
    Warning,
    /// Blad, blokuje kompilacje.
    Error,
}

impl Severity {
    pub fn label(&self) -> &'static str {
        match self {
            Severity::Hint => "uwaga",
            Severity::Warning => "ostrzezenie",
            Severity::Error => "blad",
        }
    }

    pub fn colored_label(&self) -> ColoredString {
        match self {
            Severity::Hint => "uwaga".bright_blue(),
            Severity::Warning => "ostrzezenie".yellow(),
            Severity::Error => "blad".red().bold(),
        }
    }

    pub fn symbol(&self) -> &'static str {
        match self {
            Severity::Hint => "i",
            Severity::Warning => "!",
            Severity::Error => "x",
        }
    }
}

/// Klasa bledu wyznaczona przez zakres kodu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorClass {
    Syntax,
    Types,
    Interface,
    State,
    Api,
    Security,
    Accessibility,
    Platform,
    Performance,
    Unknown,
}

impl ErrorClass {
    /// Wyznacza klase na podstawie kodu numerycznego.
    pub fn from_code(code: u32) -> ErrorClass {
        match code {
            1000..=1999 => ErrorClass::Syntax,
            2000..=2999 => ErrorClass::Types,
            3000..=3999 => ErrorClass::Interface,
            4000..=4999 => ErrorClass::State,
            5000..=5999 => ErrorClass::Api,
            6000..=6999 => ErrorClass::Security,
            7000..=7999 => ErrorClass::Accessibility,
            8000..=8999 => ErrorClass::Platform,
            9000..=9999 => ErrorClass::Performance,
            _ => ErrorClass::Unknown,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            ErrorClass::Syntax => "skladnia",
            ErrorClass::Types => "typy",
            ErrorClass::Interface => "interfejs i layout",
            ErrorClass::State => "stan i magazyny",
            ErrorClass::Api => "API i siec",
            ErrorClass::Security => "bezpieczenstwo",
            ErrorClass::Accessibility => "dostepnosc",
            ErrorClass::Platform => "platforma i nawigacja",
            ErrorClass::Performance => "wydajnosc",
            ErrorClass::Unknown => "nieznana",
        }
    }

    pub fn range(&self) -> &'static str {
        match self {
            ErrorClass::Syntax => "AX1000-AX1999",
            ErrorClass::Types => "AX2000-AX2999",
            ErrorClass::Interface => "AX3000-AX3999",
            ErrorClass::State => "AX4000-AX4999",
            ErrorClass::Api => "AX5000-AX5999",
            ErrorClass::Security => "AX6000-AX6999",
            ErrorClass::Accessibility => "AX7000-AX7999",
            ErrorClass::Platform => "AX8000-AX8999",
            ErrorClass::Performance => "AX9000-AX9999",
            ErrorClass::Unknown => "AX0000",
        }
    }
}

/// Pojedynczy komunikat diagnostyczny.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    pub code: String,
    pub numeric_code: u32,
    pub class: ErrorClass,
    pub severity: Severity,
    pub message: String,
    pub fix: Option<String>,
    pub span: Option<Span>,
    /// Fragment kodu zrodlowego dla kontekstu.
    pub snippet: Option<String>,
}

impl Diagnostic {
    pub fn error(code: u32, message: String, fix: Option<String>, span: Option<Span>) -> Self {
        Diagnostic {
            code: format!("AX{}", code),
            numeric_code: code,
            class: ErrorClass::from_code(code),
            severity: Severity::Error,
            message,
            fix,
            span,
            snippet: None,
        }
    }

    pub fn warning(code: u32, message: String, fix: Option<String>, span: Option<Span>) -> Self {
        Diagnostic {
            code: format!("AX{}", code),
            numeric_code: code,
            class: ErrorClass::from_code(code),
            severity: Severity::Warning,
            message,
            fix,
            span,
            snippet: None,
        }
    }

    pub fn hint(code: u32, message: String, fix: Option<String>, span: Option<Span>) -> Self {
        Diagnostic {
            code: format!("AX{}", code),
            numeric_code: code,
            class: ErrorClass::from_code(code),
            severity: Severity::Hint,
            message,
            fix,
            span,
            snippet: None,
        }
    }

    /// Dolacza fragment zrodla dla kontekstu komunikatu.
    pub fn with_snippet(mut self, source: &str) -> Self {
        if let Some(span) = self.span {
            if span.line > 0 {
                let lines: Vec<&str> = source.split('\n').collect();
                if span.line <= lines.len() {
                    self.snippet = Some(lines[span.line - 1].to_string());
                }
            }
        }
        self
    }

    /// Formatuje komunikat w stylu kompilatora: kod, kategoria, pozycja, wskaznik.
    pub fn format_pretty(&self) -> String {
        let mut out = String::new();

        out.push_str(&format!(
            "{} {} {}\n",
            format!("[{}]", self.code).bright_red().bold(),
            self.class.name().bright_cyan(),
            self.severity.colored_label()
        ));

        out.push_str(&format!("  {}\n", self.message));

        if let (Some(span), Some(snippet)) = (&self.span, &self.snippet) {
            if span.line > 0 {
                out.push_str(&format!(
                    "\n  {}:{}:{}\n",
                    "-->".bright_blue(),
                    span.line,
                    span.column + 1
                ));
                out.push_str(&format!("   {} {}\n", "|".bright_blue(), snippet.trim_end()));
                out.push_str(&format!(
                    "   {} {}{}\n",
                    "|".bright_blue(),
                    " ".repeat(span.column),
                    "^".repeat(span.length.max(1)).bright_red().bold()
                ));
            }
        } else if let Some(span) = &self.span {
            if span.line > 0 {
                out.push_str(&format!(
                    "  {} linia {}, kolumna {}\n",
                    "w".bright_blue(),
                    span.line,
                    span.column + 1
                ));
            }
        }

        if let Some(fix) = &self.fix {
            out.push_str(&format!("\n  {} {}\n", "poprawka:".green().bold(), fix));
        }

        out
    }

    /// Format jednolinijkowy dla raportow zbiorczych.
    pub fn format_short(&self) -> String {
        let position = self
            .span
            .as_ref()
            .filter(|s| s.line > 0)
            .map(|s| format!("{}:{} ", s.line, s.column + 1))
            .unwrap_or_default();
        format!(
            "{} {}: {}{}",
            self.code,
            self.severity.label(),
            position,
            self.message
        )
    }

    /// Format JSON.
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "code": self.code,
            "numeric": self.numeric_code,
            "class": self.class.name(),
            "range": self.class.range(),
            "severity": self.severity.label(),
            "message": self.message,
            "fix": self.fix,
            "line": self.span.as_ref().map(|s| s.line),
            "column": self.span.as_ref().map(|s| s.column + 1),
            "length": self.span.as_ref().map(|s| s.length),
        })
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.format_pretty())
    }
}

/// Zbior komunikatow z podsumowaniem.
#[derive(Debug, Default, Clone)]
pub struct Diagnostics {
    pub items: Vec<Diagnostic>,
}

impl Diagnostics {
    pub fn new() -> Self {
        Diagnostics { items: Vec::new() }
    }

    pub fn push(&mut self, diagnostic: Diagnostic) {
        self.items.push(diagnostic);
    }

    pub fn error(&mut self, code: u32, message: impl Into<String>, fix: Option<String>, span: Option<Span>) {
        self.items.push(Diagnostic::error(code, message.into(), fix, span));
    }

    pub fn warning(&mut self, code: u32, message: impl Into<String>, fix: Option<String>, span: Option<Span>) {
        self.items.push(Diagnostic::warning(code, message.into(), fix, span));
    }

    pub fn hint(&mut self, code: u32, message: impl Into<String>, fix: Option<String>, span: Option<Span>) {
        self.items.push(Diagnostic::hint(code, message.into(), fix, span));
    }

    pub fn errors(&self) -> Vec<&Diagnostic> {
        self.items
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .collect()
    }

    pub fn warnings(&self) -> Vec<&Diagnostic> {
        self.items
            .iter()
            .filter(|d| d.severity == Severity::Warning)
            .collect()
    }

    pub fn hints(&self) -> Vec<&Diagnostic> {
        self.items
            .iter()
            .filter(|d| d.severity == Severity::Hint)
            .collect()
    }

    pub fn error_count(&self) -> usize {
        self.errors().len()
    }

    pub fn warning_count(&self) -> usize {
        self.warnings().len()
    }

    pub fn hint_count(&self) -> usize {
        self.hints().len()
    }

    pub fn has_errors(&self) -> bool {
        self.error_count() > 0
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Grupuje komunikaty wedlug klasy bledu.
    pub fn by_class(&self) -> Vec<(ErrorClass, Vec<&Diagnostic>)> {
        let mut groups: Vec<(ErrorClass, Vec<&Diagnostic>)> = Vec::new();
        let classes = [
            ErrorClass::Syntax,
            ErrorClass::Types,
            ErrorClass::Interface,
            ErrorClass::State,
            ErrorClass::Api,
            ErrorClass::Security,
            ErrorClass::Accessibility,
            ErrorClass::Platform,
            ErrorClass::Performance,
            ErrorClass::Unknown,
        ];
        for class in classes {
            let items: Vec<&Diagnostic> =
                self.items.iter().filter(|d| d.class == class).collect();
            if !items.is_empty() {
                groups.push((class, items));
            }
        }
        groups
    }

    /// Sortuje komunikaty po numerze linii.
    pub fn sorted(&self) -> Vec<&Diagnostic> {
        let mut items: Vec<&Diagnostic> = self.items.iter().collect();
        items.sort_by_key(|d| d.span.as_ref().map(|s| s.line).unwrap_or(0));
        items
    }

    /// Dolacza fragmenty zrodla do wszystkich komunikatow.
    pub fn attach_snippets(&mut self, source: &str) {
        for item in &mut self.items {
            if let Some(span) = item.span {
                if span.line > 0 {
                    let lines: Vec<&str> = source.split('\n').collect();
                    if span.line <= lines.len() {
                        item.snippet = Some(lines[span.line - 1].to_string());
                    }
                }
            }
        }
    }

    /// Podsumowanie liczbowe.
    pub fn summary(&self) -> String {
        if self.is_empty() {
            return "brak komunikatow".green().bold().to_string();
        }
        format!(
            "{} bledow, {} ostrzezen, {} uwag",
            self.error_count(),
            self.warning_count(),
            self.hint_count()
        )
    }
}

/// Rejestr wszystkich kodow bledow z opisem. Uzywany przez polecenie info
/// i przez dokumentacje generowana z kodu.
pub struct ErrorCatalog;

impl ErrorCatalog {
    pub fn all() -> Vec<(u32, &'static str, &'static str, &'static str)> {
        vec![
            (1001, "Skladnia", "Nieznana jednostka programu na poziomie zerowego wciecia", "Uzyj jednej z trzynastu jednostek jezyka"),
            (1002, "Skladnia", "Wciecie nie jest wielokrotnoscia dwoch spacji", "Popraw wciecie w tej linii"),
            (1003, "Skladnia", "Skok wciecia wiekszy niz dwa poziomy", "Zmniejsz wciecie albo dodaj poziom posredni"),
            (1004, "Skladnia", "Nieznana jednostka programu", "Sprawdz nazwe jednostki"),
            (1005, "Skladnia", "Linia poza blokiem jednostki", "Przypisz linie do bloku"),
            (1006, "Skladnia", "Nierozpoznane zdarzenie", "Sprawdz nazwe zdarzenia"),
            (1007, "Skladnia", "Nierozpoznana instrukcja", "Sprawdz skladnie instrukcji"),
            (2001, "Typy", "Pole modelu uzywa nieznanego typu", "Uzyj typu wbudowanego albo zadeklaruj data lub enum"),
            (2002, "Typy", "Pole modelu nie ma typu", "Dodaj typ po nazwie pola"),
            (2003, "Typy", "Niezgodnosc typow w przypisaniu", "Dostosuj typ wartosci do typu zmiennej"),
            (2004, "Typy", "Nieznany typ w parametrze funkcji", "Popraw typ parametru"),
            (2005, "Typy", "Wywolanie funkcji z nieprawidlowa liczba argumentow", "Dostosuj liczbe argumentow do deklaracji"),
            (3001, "Interfejs", "Odwolanie do nieistniejacego stylu", "Dodaj styl albo popraw nazwe"),
            (3002, "Interfejs", "Kontener bez zawartosci", "Dodaj elementy albo usun kontener"),
            (3003, "Interfejs", "Siatka bez liczby kolumn", "Dodaj akcesorium columns z liczba kolumn"),
            (4001, "Stan", "Nieprawidlowa deklaracja stanu", "Poprawna forma: state nazwa Typ = wartosc"),
            (4002, "Stan", "Operacja na niezadeklarowanym magazynie", "Zadeklaruj magazyn skladnia store nazwa klasa"),
            (4003, "Stan", "Nieznana klasa magazynu", "Dozwolone klasy: local, synced, encrypted, secure, cache, queue"),
            (4004, "Stan", "Magazyn bez klucza", "Dodaj klucz wersjonowany, na przyklad key app.dane.v1"),
            (5001, "API", "Brak limitu czasu w deklaracji API", "Dodaj timeout 20000"),
            (5002, "API", "Brak ponawiania w deklaracji API", "Dodaj retry 3"),
            (5003, "API", "Endpoint bez sciezki", "Dodaj metode i sciezke, na przyklad get /lista"),
            (5004, "API", "Adres bazowy bez protokolu HTTPS", "Uzyj https zamiast http"),
            (6001, "Bezpieczenstwo", "Magazyn na sekrety bez klasy secure", "Zmien na: store nazwa secure"),
            (6002, "Bezpieczenstwo", "Klucz prywatny w magazynie jawnym", "Przenies do magazynu klasy secure"),
            (6003, "Bezpieczenstwo", "Logowanie danych wrazliwych", "Usun logowanie tokenow i hasel"),
            (6004, "Bezpieczenstwo", "Wysylanie klucza API do backendu", "Klucz musi pozostac na urzadzeniu"),
            (7001, "Dostepnosc", "Obraz bez tekstu alternatywnego", "Dodaj alt z opisem obrazu"),
            (7002, "Dostepnosc", "Przycisk ikonowy bez etykiety dostepnosci", "Dodaj accessibility label z opisem akcji"),
            (7003, "Dostepnosc", "Pole bez etykiety", "Dodaj label z nazwa pola"),
            (7004, "Dostepnosc", "Element interaktywny bez roli", "Dodaj role opisujaca przeznaczenie"),
            (7005, "Dostepnosc", "Animacja bez respektowania ograniczenia ruchu", "Dodaj warunek ograniczenia ruchu"),
            (8001, "Platforma", "Nawigacja do nieistniejacego ekranu", "Popraw nazwe ekranu albo dodaj brakujacy ekran"),
            (8002, "Platforma", "Uprawnienie bez uzasadnienia", "Dodaj purpose z wyjasnieniem celu"),
            (8003, "Platforma", "Ekran startowy nie istnieje", "Popraw pole entry albo dodaj brakujacy ekran"),
            (9001, "Wydajnosc", "Lista bez pustego stanu", "Dodaj galaz empty z komunikatem"),
            (9002, "Wydajnosc", "Zbyt wiele elementow w jednym ekranie", "Podziel ekran na mniejsze"),
            (9003, "Wydajnosc", "Magazyn bez limitu rozmiaru", "Rozwaz klase cache albo limit wpisow"),
        ]
    }

    pub fn find(code: u32) -> Option<&'static str> {
        Self::all()
            .into_iter()
            .find(|(c, _, _, _)| *c == code)
            .map(|(_, _, msg, _)| msg)
    }

    pub fn count() -> usize {
        Self::all().len()
    }
}
