// src/fmt.rs — formatter pliku .ax
//
// Formatter porzadkuje plik bez zmiany semantyki: usuwa ciagle spacje na
// koncu linii, normalizuje wciecia do wielokrotnosci dwoch spacji wedlug
// struktury blokow, usuwa nadmiar pustych linii i pilnuje jednej pustej
// linii miedzy deklaracjami najwyzszego poziomu.
//
// Formatter nie zmienia tresci literalow ani komentarzy. Jesli natrafi na
// blad skladni, raportuje go i zwraca plik wejsciowy bez zmian.

use crate::diagnostics::{Diagnostics, ErrorCatalog};

/// Jednostki jezyka, ktore zawsze rozpoczynaja nowy blok najwyzszego poziomu.
const BLOCK_UNITS: &[&str] = &[
    "app", "theme", "style", "enum", "data", "store", "global", "permission", "api", "function",
    "component", "screen", "test",
];

/// Deklaracje pol najwyzszego poziomu nalezace do bloku app.
const APP_FIELDS: &[&str] = &[
    "version", "language", "supports", "orientation", "offline", "entry", "minOs",
];

/// Slowa rozpoczynajace zagniezdzony blok wewnatrz ekranu lub funkcji.
const INNER_BLOCKS: &[&str] = &["when", "if", "else", "for", "try", "catch", "finally", "derive"];

/// Formatuje plik zrodlowy i zwraca nowa tresc.
pub fn format_source(source: &str, diagnostics: &mut Diagnostics) -> String {
    let lines: Vec<&str> = source.lines().collect();
    let mut out: Vec<String> = Vec::new();
    let mut indent_stack: Vec<usize> = vec![0];
    let mut previous_was_top_level = false;
    let mut pending_blank = false;

    for (index, raw) in lines.iter().enumerate() {
        let line_no = index + 1;
        let trimmed_end = raw.trim_end();
        let stripped = trimmed_end.trim_start();

        // pusta linia: zapamietujemy, ale nie wypisujemy od razu
        if stripped.is_empty() {
            pending_blank = true;
            continue;
        }

        let raw_indent = measure_indent(trimmed_end);
        let keyword = first_word(stripped);
        let rest = stripped[keyword.len()..].trim();

        // "ui test" i "accessibility test" sa jednostkami najwyzszego poziomu
        let mut is_declaration = BLOCK_UNITS.contains(&keyword);
        if (keyword == "ui" || keyword == "accessibility") && rest.starts_with("test") {
            is_declaration = true;
        }
        if keyword == "global" && rest.starts_with("state") {
            is_declaration = true;
        }

        let is_app_field = APP_FIELDS.contains(&keyword);
        let is_inner_block = INNER_BLOCKS.contains(&keyword);

        if is_declaration && raw_indent == 0 {
            indent_stack.clear();
            indent_stack.push(0);
        }

        let target_indent = if is_declaration && raw_indent == 0 {
            0
        } else {
            align_to_stack(raw_indent, &indent_stack)
        };

        // pusta linia przed nowa deklaracja najwyzszego poziomu
        if target_indent == 0 && !out.is_empty() && (pending_blank || previous_was_top_level) {
            if !out.last().map(|l| l.is_empty()).unwrap_or(true) {
                out.push(String::new());
            }
        }

        let normalized = normalize_body(stripped);
        out.push(format!("{}{}", " ".repeat(target_indent), normalized));

        // aktualizacja stosu wciec
        if is_declaration || is_inner_block || is_app_field {
            push_level(&mut indent_stack, raw_indent, target_indent + 2);
        } else if raw_indent == 0 {
            indent_stack.clear();
            indent_stack.push(0);
        }

        previous_was_top_level = target_indent == 0;
        pending_blank = false;

        // ostrzezenie o mieszaniu spacji i tabulatorow
        if raw.starts_with('\t') && raw.contains(' ') {
            let mut diagnostic = Diagnostics::single(
                ErrorCatalog::Syntax("mieszane wciecie: tabulatory i spacje".to_string()),
                line_no,
                1,
            );
            diagnostic.message = format!(
                "Linia {} miesza tabulatory ze spacjami. Formatter wyrownuje do spacji.",
                line_no
            );
            diagnostic.suggestion = Some("ustaw edytor na wciecia dwoma spacjami".to_string());
            diagnostics.push_diagnostic(diagnostic);
        }
    }

    // jeden znak nowej linii na koncu
    while out.last().map(|l| l.is_empty()).unwrap_or(false) {
        out.pop();
    }
    let mut result = out.join("\n");
    result.push('\n');
    result
}

/// Mierzy wciecie linii, licząc tabulator jako dwie spacje.
fn measure_indent(line: &str) -> usize {
    let mut width = 0;
    for ch in line.chars() {
        match ch {
            ' ' => width += 1,
            '\t' => width += 2,
            _ => break,
        }
    }
    width
}

/// Dopasowuje wciecie zrodlowe do najblizszego poziomu ze stosu.
fn align_to_stack(raw_indent: usize, stack: &[usize]) -> usize {
    let mut best = *stack.last().unwrap_or(&0);
    for level in stack {
        if *level <= raw_indent {
            best = *level;
        }
    }
    if best % 2 == 1 {
        best + 1
    } else {
        best
    }
}

/// Dodaje nowy poziom wciecia, jesli glebszy niz poprzedni.
fn push_level(stack: &mut Vec<usize>, raw_indent: usize, next: usize) {
    while stack.len() > 1 {
        match stack.last() {
            Some(last) if *last >= raw_indent => {
                stack.pop();
            }
            _ => break,
        }
    }
    if stack.last().map(|l| *l < next).unwrap_or(true) {
        stack.push(next);
    }
}

/// Normalizuje bialy znak wewnatrz linii: pojedyncze spacje, brak koncowych.
fn normalize_body(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut in_string = false;
    let mut pending_space = false;

    for ch in line.chars() {
        if in_string {
            out.push(ch);
            if ch == '"' {
                in_string = false;
            }
            continue;
        }
        match ch {
            '"' => {
                if pending_space && !out.is_empty() {
                    out.push(' ');
                    pending_space = false;
                }
                in_string = true;
                out.push(ch);
            }
            ' ' | '\t' => {
                pending_space = true;
            }
            _ => {
                if pending_space && !out.is_empty() {
                    out.push(' ');
                }
                pending_space = false;
                out.push(ch);
            }
        }
    }
    out.trim_end().to_string()
}

fn first_word(line: &str) -> &str {
    line.split_whitespace().next().unwrap_or("")
}
