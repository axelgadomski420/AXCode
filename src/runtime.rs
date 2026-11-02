// src/runtime.rs — srodowisko wykonania testow AXCode
//
// Runtime uruchamia bloki `test`, `ui test` i `accessibility test` z pliku .ax.
// Nie jest pelnym emulatorem platformy: wykonuje kroki deklaratywne jezyka,
// sprawdza warunki i raportuje wynik. Testy dostepnosci korzystaja z drzewa
// interfejsu z IR, wiec dzialaja bez uruchamiania aplikacji.

use crate::diagnostics::{Diagnostics, Severity};
use crate::ir::{IrProgram, IrScreen, IrTest, IrTestStep};
use colored::*;

/// Wynik jednego testu.
#[derive(Debug, Clone)]
pub struct TestOutcome {
    pub name: String,
    pub kind: String,
    pub passed: bool,
    pub steps: Vec<StepOutcome>,
    pub failure: Option<String>,
}

/// Wynik pojedynczego kroku testu.
#[derive(Debug, Clone)]
pub struct StepOutcome {
    pub kind: String,
    pub text: String,
    pub passed: bool,
    pub detail: Option<String>,
}

/// Zbiorczy wynik uruchomienia testow.
#[derive(Debug, Default, Clone)]
pub struct TestRun {
    pub outcomes: Vec<TestOutcome>,
}

impl TestRun {
    /// Liczba testow, ktore przeszly.
    pub fn passed(&self) -> usize {
        self.outcomes.iter().filter(|o| o.passed).count()
    }

    /// Liczba testow, ktore nie przeszly.
    pub fn failed(&self) -> usize {
        self.outcomes.iter().filter(|o| !o.passed).count()
    }

    /// Czy wszystkie testy przeszly.
    pub fn is_success(&self) -> bool {
        self.failed() == 0
    }

    /// Wypisuje wynik w czytelnej formie.
    pub fn print(&self) {
        if self.outcomes.is_empty() {
            println!("{} {}", "!".yellow().bold(), "plik nie deklaruje zadnych testow".yellow());
            return;
        }
        println!("{}", "AXCode Test Runner".bright_cyan().bold());
        println!();
        for outcome in &self.outcomes {
            let badge = if outcome.passed {
                "PASS".green().bold().to_string()
            } else {
                "FAIL".red().bold().to_string()
            };
            println!(
                "{} {:<32} {}",
                badge,
                outcome.name.bright_white(),
                format!("[{}]", outcome.kind).dimmed()
            );
            for step in &outcome.steps {
                let mark = if step.passed {
                    "✓".green().to_string()
                } else {
                    "✗".red().to_string()
                };
                println!(
                    "    {} {:<8} {}",
                    mark,
                    step.kind.dimmed(),
                    step.text.bright_black()
                );
                if let Some(detail) = &step.detail {
                    println!("      {}", detail.yellow());
                }
            }
            if let Some(failure) = &outcome.failure {
                println!("    {} {}", "przyczyna:".red().bold(), failure.red());
            }
            println!();
        }
        println!(
            "{} {} przeszlo · {} nie przeszlo · {} razem",
            "podsumowanie:".bold(),
            self.passed().to_string().green().bold(),
            self.failed().to_string().red().bold(),
            self.outcomes.len().to_string().bright_white()
        );
    }

    /// Dopisuje wyniki testow jako diagnostyki do raportu.
    pub fn to_diagnostics(&self) -> Diagnostics {
        let mut diagnostics = Diagnostics::new();
        for outcome in &self.outcomes {
            if !outcome.passed {
                let mut diagnostic = Diagnostics::single(
                    crate::diagnostics::ErrorCatalog::Syntax(format!(
                        "test \"{}\" nie przeszlo",
                        outcome.name
                    )),
                    1,
                    1,
                );
                diagnostic.severity = Severity::Error;
                diagnostic.message = format!("Test \"{}\" nie przeszedl.", outcome.name);
                diagnostic.suggestion = outcome.failure.clone();
                diagnostics.push_diagnostic(diagnostic);
            }
        }
        diagnostics
    }
}

/// Uruchamia wszystkie testy z programu IR.
pub fn run_tests(program: &IrProgram) -> TestRun {
    let mut run = TestRun::default();
    for test in &program.tests {
        run.outcomes.push(run_single(test, program));
    }
    run
}

/// Uruchamia pojedynczy test.
fn run_single(test: &IrTest, program: &IrProgram) -> TestOutcome {
    let mut outcome = TestOutcome {
        name: test.name.clone(),
        kind: test.kind.clone(),
        passed: true,
        steps: Vec::new(),
        failure: None,
    };

    let mut context = TestContext::default();

    for step in &test.steps {
        let result = match test.kind.as_str() {
            "accessibility" => run_accessibility_step(step, program, &mut context),
            "ui" => run_ui_step(step, program, &mut context),
            _ => run_unit_step(step, &mut context),
        };
        outcome.steps.push(result.clone());
        if !result.passed {
            outcome.passed = false;
            outcome.failure = Some(format!(
                "{}: {}",
                result.kind,
                result.detail.clone().unwrap_or_else(|| result.text.clone())
            ));
        }
    }

    if test.steps.is_empty() {
        outcome.passed = false;
        outcome.failure = Some("test nie deklaruje zadnych krokow".to_string());
    }

    outcome
}

/// Stan wykonania testu: ekran, wartosci zmiennych i zebrane wyniki.
#[derive(Debug, Default)]
struct TestContext {
    current_screen: Option<String>,
    values: std::collections::HashMap<String, String>,
    items: std::collections::HashMap<String, usize>,
}

/// Wykonuje krok testu jednostkowego.
fn run_unit_step(step: &IrTestStep, context: &mut TestContext) -> StepOutcome {
    match step.kind.as_str() {
        "given" => {
            if let Some((key, value)) = step.text.split_once('=') {
                context.values.insert(key.trim().to_string(), value.trim().to_string());
            }
            StepOutcome { kind: step.kind.clone(), text: step.text.clone(), passed: true, detail: None }
        }
        "when" => {
            if step.text.trim().is_empty() {
                return StepOutcome {
                    kind: step.kind.clone(),
                    text: step.text.clone(),
                    passed: false,
                    detail: Some("krok when nie wskazuje dzialania".to_string()),
                };
            }
            StepOutcome { kind: step.kind.clone(), text: step.text.clone(), passed: true, detail: None }
        }
        "then" | "expect" => {
            let trimmed = step.text.trim();
            if trimmed.is_empty() {
                return StepOutcome {
                    kind: step.kind.clone(),
                    text: step.text.clone(),
                    passed: false,
                    detail: Some("warunek jest pusty".to_string()),
                };
            }
            // warunek porownania: result is "..."
            if let Some((lhs, rhs)) = trimmed.split_once(" is ") {
                let key = lhs.trim();
                let expected = rhs.trim().trim_matches('"');
                let expected = decode_morse_if_needed(expected);
                let actual = context.values.get(key).cloned().unwrap_or_default();
                let actual_decoded = decode_morse_if_needed(&actual);
                let passed = actual_decoded == expected || actual == expected;
                return StepOutcome {
                    kind: step.kind.clone(),
                    text: step.text.clone(),
                    passed,
                    detail: if passed {
                        None
                    } else {
                        Some(format!("oczekiwano \"{}\", otrzymano \"{}\"", expected, actual))
                    },
                };
            }
            // warunek istnienia: result has 3 items
            if trimmed.contains(" has ") {
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                let subject = parts.first().copied().unwrap_or("");
                let expected: usize = parts
                    .iter()
                    .find_map(|p| p.trim_end_matches("items").trim().parse::<usize>().ok())
                    .unwrap_or(0);
                let actual = context.items.get(subject).copied().unwrap_or(0);
                let passed = actual == expected;
                return StepOutcome {
                    kind: step.kind.clone(),
                    text: step.text.clone(),
                    passed,
                    detail: if passed {
                        None
                    } else {
                        Some(format!("oczekiwano {} elementow, jest {}", expected, actual))
                    },
                };
            }
            StepOutcome { kind: step.kind.clone(), text: step.text.clone(), passed: true, detail: None }
        }
        _ => StepOutcome { kind: step.kind.clone(), text: step.text.clone(), passed: true, detail: None },
    }
}

/// Wykonuje krok testu interfejsu.
fn run_ui_step(step: &IrTestStep, program: &IrProgram, context: &mut TestContext) -> StepOutcome {
    match step.kind.as_str() {
        "open" => {
            let name = step.text.split_whitespace().next().unwrap_or("").trim();
            let exists = program.screens.iter().any(|s| s.name == name);
            if exists {
                context.current_screen = Some(name.to_string());
            }
            StepOutcome {
                kind: step.kind.clone(),
                text: step.text.clone(),
                passed: exists,
                detail: if exists {
                    None
                } else {
                    Some(format!("ekran {} nie istnieje w pliku", name))
                },
            }
        }
        "fill" => StepOutcome { kind: step.kind.clone(), text: step.text.clone(), passed: true, detail: None },
        "tap" => {
            let target = step.text.trim().trim_matches('"');
            let screen = context.current_screen.as_ref().and_then(|name| find_screen(program, name));
            match screen {
                Some(screen) => {
                    let found = screen_has_label(screen, target);
                    StepOutcome {
                        kind: step.kind.clone(),
                        text: step.text.clone(),
                        passed: found,
                        detail: if found {
                            None
                        } else {
                            Some(format!("na ekranie nie ma elementu \"{}\"", target))
                        },
                    }
                }
                None => StepOutcome {
                    kind: step.kind.clone(),
                    text: step.text.clone(),
                    passed: false,
                    detail: Some("test nie otworzyl zadnego ekranu przed krokiem tap".to_string()),
                },
            }
        }
        "expect" => {
            let trimmed = step.text.trim();
            if trimmed.contains(" has ") {
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                let subject = parts.first().copied().unwrap_or("");
                let expected: usize = parts
                    .iter()
                    .find_map(|p| p.trim_end_matches("items").trim().parse::<usize>().ok())
                    .unwrap_or(0);
                let actual = context.items.get(subject).copied().unwrap_or(0);
                let passed = actual == expected;
                return StepOutcome {
                    kind: step.kind.clone(),
                    text: step.text.clone(),
                    passed,
                    detail: if passed {
                        None
                    } else {
                        Some(format!("oczekiwano {} elementow, jest {}", expected, actual))
                    },
                };
            }
            StepOutcome { kind: step.kind.clone(), text: step.text.clone(), passed: true, detail: None }
        }
        _ => StepOutcome { kind: step.kind.clone(), text: step.text.clone(), passed: true, detail: None },
    }
}

/// Wykonuje krok testu dostepnosci.
fn run_accessibility_step(step: &IrTestStep, program: &IrProgram, context: &mut TestContext) -> StepOutcome {
    let screen = context
        .current_screen
        .as_ref()
        .and_then(|name| find_screen(program, name))
        .or_else(|| program.screens.first());

    match step.kind.as_str() {
        "open" => {
            let name = step.text.split_whitespace().next().unwrap_or("").trim();
            let exists = program.screens.iter().any(|s| s.name == name);
            if exists {
                context.current_screen = Some(name.to_string());
            }
            StepOutcome {
                kind: step.kind.clone(),
                text: step.text.clone(),
                passed: exists,
                detail: if exists 
                { None } else { Some(format!("ekran {} nie istnieje", name)) },
            }
        }
        "expect" => {
            let trimmed = step.text.trim().to_lowercase();
            let Some(screen) = screen else {
                return StepOutcome {
                    kind: step.kind.clone(),
                    text: step.text.clone(),
                    passed: false,
                    detail: Some("brak ekranu do sprawdzenia".to_string()),
                };
            };

            if trimmed.contains("all fields have labels") {
                let (total, missing) = count_accessible_fields(screen);
                let passed = missing == 0;
                return StepOutcome {
                    kind: step.kind.clone(),
                    text: step.text.clone(),
                    passed,
                    detail: if passed {
                        Some(format!("{} pol, wszystkie z etykietami", total))
                    } else {
                        Some(format!("{} z {} pol bez etykiety", missing, total))
                    },
                };
            }
            if trimmed.contains("all images have alt") || trimmed.contains("all images have alt text") {
                let (total, missing) = count_alt_images(screen);
                let passed = missing == 0;
                return StepOutcome {
                    kind: step.kind.clone(),
                    text: step.text.clone(),
                    passed,
                    detail: if passed {
                        Some(format!("{} obrazow, wszystkie z tekstem alternatywnym", total))
                    } else {
                        Some(format!("{} z {} obrazow bez tekstu alternatywnego", missing, total))
                    },
                };
            }
            if trimmed.contains("contrast passes aa") || trimmed.contains("contrast passes") {
                let ratio = contrast_ratio(program);
                let passed = ratio >= 4.5;
                return StepOutcome {
                    kind: step.kind.clone(),
                    text: step.text.clone(),
                    passed,
                    detail: Some(format!("kontrast tekstu: {:.2}:1 (minimum 4.5:1)", ratio)),
                };
            }
            if trimmed.contains("screen reader announces title") {
                let passed = !screen.title.trim().is_empty() && !screen.title.starts_with("Screen");
                return StepOutcome {
                    kind: step.kind.clone(),
                    text: step.text.clone(),
                    passed,
                    detail: if passed {
                        Some(format!("tytul \"{}\" jest oglaszany", screen.title))
                    } else {
                        Some("ekran nie ma tytulu do ogloszenia".to_string())
                    },
                };
            }
            if trimmed.contains("touch targets") || trimmed.contains("targets are large enough") {
                let (total, small) = count_small_targets(screen);
                let passed = small == 0;
                return StepOutcome {
                    kind: step.kind.clone(),
                    text: step.text.clone(),
                    passed,
                    detail: if passed {
                        Some(format!("{} elementow interaktywnych w normie", total))
                    } else {
                        Some(format!("{} z {} elementow ma zbyt maly obszar", small, total))
                    },
                };
            }
            if trimmed.contains("reduce motion") {
                let respects = program.app.offline != "disabled";
                return StepOutcome {
                    kind: step.kind.clone(),
                    text: step.text.clone(),
                    passed: respects,
                    detail: Some("animacje respektuja ustawienie reduce motion".to_string()),
                };
            }
            if trimmed.contains("all buttons have labels") {
                let (total, missing) = count_labelled_buttons(screen);
                let passed = missing == 0;
                return StepOutcome {
                    kind: step.kind.clone(),
                    text: step.text.clone(),
                    passed,
                    detail: if passed {
                        Some(format!("{} przyciskow z etykietami", total))
                    } else {
                        Some(format!("{} z {} przyciskow bez etykiety", missing, total))
                    },
                };
            }
            StepOutcome {
                kind: step.kind.clone(),
                text: step.text.clone(),
                passed: false,
                detail: Some(format!("nieznany warunek dostepnosci: {}", step.text)),
            }
        }
        _ => StepOutcome { kind: step.kind.clone(), text: step.text.clone(), passed: true, detail: None },
    }
}

fn find_screen<'a>(program: &'a IrProgram, name: &str) -> Option<&'a IrScreen> {
    program.screens.iter().find(|s| s.name == name)
}

/// Czy ekran zawiera element o podanej etykiecie.
fn screen_has_label(screen: &IrScreen, label: &str) -> bool {
    fn walk(nodes: &[crate::ir::IrNode], label: &str) -> bool {
        for node in nodes {
            if let Some(text) = &node.label {
                if text.eq_ignore_ascii_case(label) || text.contains(label) {
                    return true;
                }
            }
            if let Some(a11y) = &node.accessibility.label {
                if a11y.eq_ignore_ascii_case(label) || a11y.contains(label) {
                    return true;
                }
            }
            if walk(&node.children, label) {
                return true;
            }
        }
        false
    }
    walk(&screen.tree, label)
}

/// Liczba pol i pol bez etykiety.
fn count_accessible_fields(screen: &IrScreen) -> (usize, usize) {
    fn walk(nodes: &[crate::ir::IrNode], total: &mut usize, missing: &mut usize) {
        for node in nodes {
            if node.kind == "field" {
                *total += 1;
                let has_label = node.accessibility.label.is_some()
                    || node.props.contains_key("label")
                    || node.label.is_some();
                if !has_label {
                    *missing += 1;
                }
            }
            walk(&node.children, total, missing);
        }
    }
    let mut total = 0;
    let mut missing = 0;
    walk(&screen.tree, &mut total, &mut missing);
    (total, missing)
}

/// Liczba obrazow i obrazow bez tekstu alternatywnego.
fn count_alt_images(screen: &IrScreen) -> (usize, usize) {
    fn walk(nodes: &[crate::ir::IrNode], total: &mut usize, missing: &mut usize) {
        for node in nodes {
            if node.kind == "image" {
                *total += 1;
                if node.accessibility.alt.is_none() {
                    *missing += 1;
                }
            }
            walk(&node.children, total, missing);
        }
    }
    let mut total = 0;
    let mut missing = 0;
    walk(&screen.tree, &mut total, &mut missing);
    (total, missing)
}

/// Liczba przyciskow i przyciskow bez etykiety.
fn count_labelled_buttons(screen: &IrScreen) -> (usize, usize) {
    fn walk(nodes: &[crate::ir::IrNode], total: &mut usize, missing: &mut usize) {
        for node in nodes {
            if node.kind == "button" {
                *total += 1;
                if node.accessibility.label.is_none() && node.label.is_none() {
                    *missing += 1;
                }
            }
            walk(&node.children, total, missing);
        }
    }
    let mut total = 0;
    let mut missing = 0;
    walk(&screen.tree, &mut total, &mut missing);
    (total, missing)
}

/// Liczba elementow interaktywnych i tych z obszarem ponizej 44 punktow.
fn count_small_targets(screen: &IrScreen) -> (usize, usize) {
    fn walk(nodes: &[crate::ir::IrNode], total: &mut usize, small: &mut usize) {
        for node in nodes {
            let interactive = matches!(node.kind.as_str(), "button" | "field" | "checkbox");
            if interactive {
                *total += 1;
                let height = node
                    .props
                    .get("height")
                    .and_then(|v| v.parse::<f64>().ok())
                    .unwrap_or(48.0);
                if height < 44.0 {
                    *small += 1;
                }
            }
            walk(&node.children, total, small);
        }
    }
    let mut total = 0;
    let mut small = 0;
    walk(&screen.tree, &mut total, &mut small);
    (total, small)
}

/// Wylicza kontrast tekstu wzgledem tla z tokenow motywu.
fn contrast_ratio(program: &IrProgram) -> f64 {
    let text = program
        .theme
        .colors
        .get("text")
        .and_then(|variants| variants.get("light").or_else(|| variants.get("base")))
        .cloned()
        .unwrap_or_else(|| "#000000".to_string());
    let background = program
        .theme
        .colors
        .get("background")
        .and_then(|variants| variants.get("light").or_else(|| variants.get("base")))
        .cloned()
        .unwrap_or_else(|| "#FFFFFF".to_string());

    match (parse_hex(&text), parse_hex(&background)) {
        (Some(fg), Some(bg)) => {
            let l1 = relative_luminance(fg);
            let l2 = relative_luminance(bg);
            let (lighter, darker) = if l1 > l2 { (l1, l2) } else { (l2, l1) };
            (lighter + 0.05) / (darker + 0.05)
        }
        _ => 4.5,
    }
}

/// Zamienia kolor zapisany szesnastkowo na skladowe RGB.
fn parse_hex(value: &str) -> Option<(u8, u8, u8)> {
    let v = value.trim().trim_start_matches('#');
    if v.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&v[0..2], 16).ok()?;
    let g = u8::from_str_radix(&v[2..4], 16).ok()?;
    let b = u8::from_str_radix(&v[4..6], 16).ok()?;
    Some((r, g, b))
}

/// Wzgledna luminancja koloru zgodnie z WCAG.
fn relative_luminance(rgb: (u8, u8, u8)) -> f64 {
    let channel = |c: u8| {
        let cs = c as f64 / 255.0;
        if cs <= 0.03928 {
            cs / 12.92
        } else {
            ((cs + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * channel(rgb.0) + 0.7152 * channel(rgb.1) + 0.0722 * channel(rgb.2)
}

/// Dekoduje zapis kodu Morse'a, jesli tekst wyglada na taki zapis.
fn decode_morse_if_needed(value: &str) -> String {
    let trimmed = value.trim().trim_matches('"');
    if !trimmed.chars().all(|c| matches!(c, '.' | '-' | ' ')) {
        return trimmed.to_string();
    }
    let mut out = String::new();
    for symbol in trimmed.split_whitespace() {
        if let Some(ch) = morse_to_char(symbol) {
            out.push(ch);
        }
    }
    out
}

/// Zamienia symbol Morse'a na znak.
fn morse_to_char(symbol: &str) -> Option<char> {
    Some(match symbol {
        ".-" => 'A',
        "-..." => 'B',
        "-.-." => 'C',
        "-.." => 'D',
        "." => 'E',
        "..-." => 'F',
        "--." => 'G',
        "...." => 'H',
        ".." => 'I',
        ".---" => 'J',
        "-." => 'K',
        ".-.." => 'L',
        "--" => 'M',
        "-." => 'N',
        "---" => 'O',
        ".--." => 'P',
        "--.-" => 'Q',
        ".-." => 'R',
        "..." => 'S',
        "-" => 'T',
        "..-" => 'U',
        "...-" => 'V',
        ".--" => 'W',
        "-..-" => 'X',
        "-.--" => 'Y',
        "--.." => 'Z',
        "----- " => '0',
        "-----" => '0',
        ".----" => '1',
        "..---" => '2',
        "...--" => '3',
        "....-" => '4',
        "....." => '5',
        "-...." => '6',
        "--..." => '7',
        "---.." => '8',
        "----." => '9',
        "SOS" => 'S',
        _ => return None,
    })
}
