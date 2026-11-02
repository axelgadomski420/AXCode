// src/lexer.rs — analiza leksykalna
//
// Zamienia tekst zrodlowy na liste tokenow. Kazdy token niesie rodzaj,
// numer linii, poziom wciecia i tresc. Wciecie jest znaczace: jezyk nie
// uzywa nawiasow klamrowych, wiec struktura bloku wynika wylacznie z niego.
// Tabulator jest normalizowany do dwoch spacji, dzieki czemu plik napisany
// tabulatorami i plik napisany spacjami daja identyczne drzewo.

use crate::ast::Span;
use crate::diagnostics::{Diagnostic, Diagnostics};
use std::fmt;

/// Rodzaj tokenu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    /// Linia pusta.
    Blank,
    /// Komentarz rozpoczety dwoma ukosnikami.
    Comment,
    /// Deklaracja bloku: app, screen, store, api, function.
    Declaration,
    /// Wywolanie funkcji albo komponentu.
    Call,
    /// Instrukcja, pole, akcesorium.
    Statement,
}

impl fmt::Display for TokenKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenKind::Blank => write!(f, "linia pusta"),
            TokenKind::Comment => write!(f, "komentarz"),
            TokenKind::Declaration => write!(f, "deklaracja"),
            TokenKind::Call => write!(f, "wywolanie"),
            TokenKind::Statement => write!(f, "instrukcja"),
        }
    }
}

/// Token z pelna informacja o pozycji.
#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub line: usize,
    pub column: usize,
    pub indent: usize,
    pub offset: usize,
    /// Pierwsze slowo linii po wcieciu. Puste dla linii pustej.
    pub head: String,
    /// Reszta linii po pierwszym slowie, bez wiodacego bialego znaku.
    pub rest: String,
    /// Cala linia po usunieciu wciecia.
    pub raw: String,
    /// Tekst komentarza bez dwoch ukosnikow.
    pub comment: String,
    /// Argumenty wywolania, jesli token jest wywolaniem.
    pub args: String,
}

impl Token {
    pub fn span(&self) -> Span {
        Span::new(self.line, self.column, self.offset, self.raw.len())
    }

    pub fn is_blank(&self) -> bool {
        self.kind == TokenKind::Blank
    }

    pub fn is_comment(&self) -> bool {
        self.kind == TokenKind::Comment
    }

    pub fn is_declaration(&self) -> bool {
        self.kind == TokenKind::Declaration
    }

    pub fn is_call(&self) -> bool {
        self.kind == TokenKind::Call
    }

    pub fn is_statement(&self) -> bool {
        self.kind == TokenKind::Statement
    }

    /// Czy token deklaruje blok o podanym slowie kluczowym.
    pub fn declares(&self, keyword: &str) -> bool {
        self.kind == TokenKind::Declaration && self.head == keyword
    }

    /// Czy token jest instrukcja zaczynajaca sie od podanego slowa.
    pub fn starts_with(&self, prefix: &str) -> bool {
        self.raw.starts_with(prefix)
    }
}

/// Slowa, ktore rozpoczynaja deklaracje bloku na poziomie zerowego wciecia.
pub const BLOCK_DECLARATORS: &[&str] = &[
    "app",
    "version",
    "language",
    "supports",
    "orientation",
    "offline",
    "entry",
    "minOs",
    "theme",
    "style",
    "enum",
    "data",
    "store",
    "global",
    "permission",
    "api",
    "function",
    "component",
    "screen",
    "test",
    "ui",
    "accessibility",
];

/// Slowa kluczowe jezyka. Uzywane przez formater i podswietlanie skladni.
pub const KEYWORDS: &[&str] = &[
    "app", "version", "language", "supports", "orientation", "offline", "entry",
    "minOs", "theme", "style", "enum", "data", "store", "global", "state",
    "permission", "api", "function", "component", "screen", "test", "ui",
    "accessibility", "returns", "single", "list", "local", "synced", "secure",
    "encrypted", "queue", "cache", "follows", "system", "true", "false", "if",
    "else", "for", "in", "return", "set", "add", "remove", "update", "save",
    "load", "clear", "when", "tapped", "changed", "appears", "disappears",
    "launches", "valid", "invalid", "error", "online", "granted", "denied",
    "go", "to", "back", "open", "close", "show", "use", "try", "catch",
    "finally", "and", "or", "is", "not", "none", "where", "order", "by",
    "ascending", "descending", "empty", "item", "column", "row", "grid",
    "card", "scroll", "spacer", "text", "button", "field", "image", "icon",
    "checkbox", "audioPlayer", "videoPlayer", "cameraPreview", "value", "then",
    "raise", "confirm", "ask", "multipart", "body", "get", "post", "put",
    "patch", "delete", "base", "auth", "bearer", "timeout", "retry", "purpose",
    "required", "precision", "duration", "limited", "approximate", "label",
    "placeholder", "type", "min", "max", "length", "email", "password",
    "number", "disabled", "color", "alt", "role", "padding", "margin", "gap",
    "align", "justify", "width", "height", "overflow", "columns", "direction",
    "vertical", "horizontal", "fill", "center", "start", "end", "stretch",
    "spaceBetween", "full", "auto", "half", "visible", "hidden", "controls",
    "subtitles", "facing", "user", "announce", "vibrate", "short", "medium",
    "long", "round", "derive", "count", "sum", "min", "max", "average",
];

/// Kontenery interfejsu rozpoznawane w drzewie.
pub const CONTAINERS: &[&str] = &[
    "column", "row", "grid", "card", "scroll", "spacer", "list", "item", "empty",
];

/// Slowa rozpoczynajace instrukcje logiczne.
pub const STATEMENT_KEYWORDS: &[&str] = &[
    "set", "add", "remove", "update", "save", "load", "clear", "enqueue",
    "flush", "return", "if", "else", "for", "try", "catch", "finally",
    "go", "open", "close", "announce", "announceUrgent", "vibrate", "raise",
    "confirm", "ask", "when",
];

/// Akcesoria, ktore moga wystapic pod wezlem interfejsu.
pub const ACCESSORIES: &[&str] = &[
    "style", "color", "gap", "padding", "margin", "align", "justify", "width",
    "height", "columns", "direction", "max lines", "overflow",
    "accessibility label", "role", "state", "disabled", "controls",
    "subtitles", "facing", "alt", "label", "placeholder", "type", "required",
    "min length", "max length", "error", "animate",
];

/// Lekser: przeksztalca tekst na liste tokenow.
pub struct Lexer<'a> {
    source: &'a str,
    diagnostics: &'a mut Diagnostics,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str, diagnostics: &'a mut Diagnostics) -> Self {
        Lexer { source, diagnostics }
    }

    /// Glowne wejscie: tokenizuje caly tekst.
    pub fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        let mut offset = 0usize;

        for (index, raw_line) in self.source.split('\n').enumerate() {
            let line_number = index + 1;
            let line = raw_line.strip_suffix('\r').unwrap_or(raw_line);

            let (indent, body) = self.measure_indent(line);

            if body.trim().is_empty() {
                tokens.push(Token {
                    kind: TokenKind::Blank,
                    line: line_number,
                    column: 0,
                    indent,
                    offset,
                    head: String::new(),
                    rest: String::new(),
                    raw: String::new(),
                    comment: String::new(),
                    args: String::new(),
                });
                offset += line.len() + 1;
                continue;
            }

            if let Some(comment) = body.strip_prefix("//") {
                tokens.push(Token {
                    kind: TokenKind::Comment,
                    line: line_number,
                    column: indent,
                    indent,
                    offset,
                    head: String::new(),
                    rest: String::new(),
                    raw: body.to_string(),
                    comment: comment.trim().to_string(),
                    args: String::new(),
                });
                offset += line.len() + 1;
                continue;
            }

            tokens.push(self.classify(body, line_number, indent, offset));
            offset += line.len() + 1;
        }

        tokens
    }

    /// Mierzy wciecie i zwraca tresc linii bez wiodacego bialego znaku.
    /// Tabulator liczy sie jako dwie spacje.
    fn measure_indent(&self, line: &str) -> (usize, &str) {
        let mut width = 0usize;
        let mut byte_index = 0usize;

        for (i, ch) in line.char_indices() {
            match ch {
                ' ' => width += 1,
                '\t' => width += 2,
                _ => {
                    byte_index = i;
                    break;
                }
            }
            byte_index = i + ch.len_utf8();
        }

        (width, &line[byte_index..])
    }

    /// Klasyfikuje linie na deklaracje, wywolanie albo instrukcje.
    fn classify(&mut self, body: &str, line: usize, indent: usize, offset: usize) -> Token {
        let mut parts = body.splitn(2, char::is_whitespace);
        let head = parts.next().unwrap_or("").to_string();
        let rest = parts.next().unwrap_or("").trim().to_string();

        // wywolanie funkcji albo komponentu: nazwa(argumenty)
        if let Some((name, args)) = split_call(body) {
            if name.chars().next().map(|c| c.is_alphanumeric()).unwrap_or(false) {
                return Token {
                    kind: TokenKind::Call,
                    line,
                    column: indent,
                    indent,
                    offset,
                    head: name.to_string(),
                    rest: args.to_string(),
                    raw: body.to_string(),
                    comment: String::new(),
                    args: args.to_string(),
                };
            }
        }

        let kind = if indent == 0 && BLOCK_DECLARATORS.contains(&head.as_str()) {
            TokenKind::Declaration
        } else if indent > 0 && BLOCK_DECLARATORS.contains(&head.as_str()) {
            TokenKind::Declaration
        } else {
            TokenKind::Statement
        };

        Token {
            kind,
            line,
            column: indent,
            indent,
            offset,
            head: head.clone(),
            rest,
            raw: body.to_string(),
            comment: String::new(),
            args: String::new(),
        }
    }
}

/// Rozpoznaje wzorzec nazwa(argumenty) i zwraca obie czesci.
/// Zwraca None, jesli linia nie konczy sie nawiasem zamykajacym.
pub fn split_call(text: &str) -> Option<(&str, &str)> {
    let trimmed = text.trim();
    if !trimmed.ends_with(')') {
        return None;
    }
    let open = trimmed.find('(')?;
    let name = trimmed[..open].trim();
    if name.is_empty() {
        return None;
    }
    if !name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '.') {
        return None;
    }
    let args = &trimmed[open + 1..trimmed.len() - 1];
    Some((name, args))
}

/// Rozbija liste argumentow po przecinkach, z pominieciem wnetrza nawiasow.
pub fn split_args(text: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut depth = 0i32;
    let mut in_string = false;
    let mut current = String::new();

    for ch in text.chars() {
        if ch == '"' {
            in_string = !in_string;
        }
        if !in_string {
            match ch {
                '(' | '[' | '{' => depth += 1,
                ')' | ']' | '}' => depth -= 1,
                ',' if depth == 0 => {
                    parts.push(current.trim().to_string());
                    current.clear();
                    continue;
                }
                _ => {}
            }
        }
        current.push(ch);
    }

    if !current.trim().is_empty() {
        parts.push(current.trim().to_string());
    }

    parts
}

/// Rozpoznaje przypisanie postaci pole = wartosc i zwraca obie czesci.
pub fn split_assignment(text: &str) -> Option<(&str, &str)> {
    let mut depth = 0i32;
    let mut in_string = false;

    for (i, ch) in text.char_indices() {
        if ch == '"' {
            in_string = !in_string;
        }
        if !in_string {
            match ch {
                '(' | '[' | '{' => depth += 1,
                ')' | ']' | '}' => depth -= 1,
                '=' if depth == 0 => {
                    let left = text[..i].trim();
                    let right = text[i + 1..].trim();
                    if left.is_empty() || right.is_empty() {
                        return None;
                    }
                    return Some((left, right));
                }
                _ => {}
            }
        }
    }
    None
}

/// Usuwa cudzyslowy z poczatku i konca tekstu.
pub fn strip_quotes(text: &str) -> String {
    let trimmed = text.trim();
    if trimmed.len() >= 2 && trimmed.starts_with('"') && trimmed.ends_with('"') {
        trimmed[1..trimmed.len() - 1].to_string()
    } else {
        trimmed.to_string()
    }
}

/// Sprawdza, czy tekst jest literatem tekstowym w cudzyslowach.
pub fn is_quoted(text: &str) -> bool {
    let t = text.trim();
    t.len() >= 2 && t.starts_with('"') && t.ends_with('"')
}

/// Pomija linie puste i komentarze, zwracajac pierwszy token tresci.
pub fn first_meaningful(tokens: &[Token], start: usize) -> Option<usize> {
    let mut i = start;
    while i < tokens.len() {
        if !tokens[i].is_blank() && !tokens[i].is_comment() {
            return Some(i);
        }
        i += 1;
    }
    None
}

/// Zlicza tokeny wedlug rodzaju. Uzywane przez polecenie stats.
pub struct TokenCounts {
    pub blank: usize,
    pub comment: usize,
    pub declaration: usize,
    pub call: usize,
    pub statement: usize,
}

impl TokenCounts {
    pub fn from(tokens: &[Token]) -> Self {
        let mut c = TokenCounts {
            blank: 0,
            comment: 0,
            declaration: 0,
            call: 0,
            statement: 0,
        };
        for t in tokens {
            match t.kind {
                TokenKind::Blank => c.blank += 1,
                TokenKind::Comment => c.comment += 1,
                TokenKind::Declaration => c.declaration += 1,
                TokenKind::Call => c.call += 1,
                TokenKind::Statement => c.statement += 1,
            }
        }
        c
    }

    pub fn total(&self) -> usize {
        self.blank + self.comment + self.declaration + self.call + self.statement
    }

    pub fn meaningful(&self) -> usize {
        self.total() - self.blank - self.comment
    }
}

/// Wykrywa niekonsekwencje wciec i raportuje je jako ostrzezenia.
pub fn check_indentation(tokens: &[Token], diagnostics: &mut Diagnostics) {
    let mut previous_indent: Option<usize> = None;

    for token in tokens {
        if token.is_blank() || token.is_comment() {
            continue;
        }
        if let Some(prev) = previous_indent {
            // wciecie musi byc wielokrotnoscia dwoch spacji
            if token.indent % 2 != 0 {
                diagnostics.push(Diagnostic::warning(
                    1002,
                    format!(
                        "Wciecie w linii {} wynosi {} znakow, a powinno byc wielokrotnoscia dwoch.",
                        token.line, token.indent
                    ),
                    Some(format!("Popraw wciecie w linii {}.", token.line)),
                    Some(token.span()),
                ));
            }
            // skok wciecia o wiecej niz dwa poziomy jest podejrzany
            if token.indent > prev + 4 {
                diagnostics.push(Diagnostic::warning(
                    1003,
                    format!(
                        "Wciecie w linii {} wzroslo o {} znakow, co moze oznaczac blad formatowania.",
                        token.line,
                        token.indent - prev
                    ),
                    Some("Zmniejsz wciecie albo sprawdz, czy nie brakuje poziomu posredniego.".to_string()),
                    Some(token.span()),
                ));
            }
        }
        previous_indent = Some(token.indent);
    }
}
