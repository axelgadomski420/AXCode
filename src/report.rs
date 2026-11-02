// src/report.rs — raportowanie wynikow AXCode
//
// Raport zbiera diagnostyki, statystyki i wyniki optymalizacji i przedstawia
// je w trzech formatach: czytelnym dla czlowieka, zwartym dla terminala oraz
// maszynowym JSON. Format jest wybierany przelacznikiem --format.

use crate::diagnostics::{Diagnostic, Diagnostics, Severity};
use crate::ir::IrStats;
use crate::lexer::TokenCounts;
use crate::optimize::OptimizeReport;
use colored::*;
use serde::Serialize;
use std::fmt;
use std::str::FromStr;

/// Format raportu wybierany z wiersza polecen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ReportFormat {
    /// Czytelny raport z kolorami i kontekstem zrodla.
    Human,
    /// Jedna linia na diagnostyke.
    Short,
    /// Maszynowy JSON.
    Json,
}

impl Default for ReportFormat {
    fn default() -> Self {
        ReportFormat::Human
    }
}

impl FromStr for ReportFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_lowercase().as_str() {
            "human" | "text" | "pretty" => Ok(ReportFormat::Human),
            "short" | "compact" | "line" => Ok(ReportFormat::Short),
            "json" | "machine" => Ok(ReportFormat::Json),
            other => Err(format!(
                "nieznany format {}: uzyj human, short albo json",
                other
            )),
        }
    }
}

impl fmt::Display for ReportFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            ReportFormat::Human => "human",
            ReportFormat::Short => "short",
            ReportFormat::Json => "json",
        })
    }
}

/// Struktura raportu maszynowego.
#[derive(Debug, Clone, Serialize)]
pub struct MachineReport {
    pub file: String,
    pub ok: bool,
    pub errors: usize,
    pub warnings: usize,
    pub notes: usize,
    pub diagnostics: Vec<MachineDiagnostic>,
    pub stats: Option<IrStats>,
    pub tokens: Option<TokenCounts>,
    pub optimization: Option<OptimizeReport>,
    pub timings: Option<MachineTimings>,
}

/// Pojedyncza diagnostyka w formacie maszynowym.
#[derive(Debug, Clone, Serialize)]
pub struct MachineDiagnostic {
    pub code: String,
    pub category: String,
    pub severity: String,
    pub line: usize,
    pub col: usize,
    pub message: String,
    pub suggestion: Option<String>,
    pub snippet: Option<String>,
}

/// Czasy etapow kompilacji w formacie maszynowym.
#[derive(Debug, Clone, Serialize, Default)]
pub struct MachineTimings {
    pub read_ms: u128,
    pub lex_ms: u128,
    pub parse_ms: u128,
    pub verify_ms: u128,
    pub hir_ms: u128,
    pub ir_ms: u128,
    pub optimize_ms: u128,
    pub total_ms: u128,
}

/// Raport z kompilacji: diagnostyki plus opcjonalne statystyki.
#[derive(Debug, Clone, Default)]
pub struct Report {
    pub file: String,
    pub diagnostics: Diagnostics,
    pub stats: Option<IrStats>,
    pub tokens: Option<TokenCounts>,
    pub optimization: Option<OptimizeReport>,
    pub timings: Option<MachineTimings>,
    pub artifacts: Vec<String>,
    pub platform: Option<String>,
}

impl Report {
    /// Tworzy raport dla pliku.
    pub fn new(file: &str) -> Self {
        Self { file: file.to_string(), ..Default::default() }
    }

    /// Liczba bledow.
    pub fn error_count(&self) -> usize {
        self.diagnostics.count_errors()
    }

    /// Liczba ostrzezen.
    pub fn warning_count(&self) -> usize {
        self.diagnostics.count_warnings()
    }

    /// Czy kompilacja mogla sie dokonczyc.
    pub fn is_ok(&self) -> bool {
        !self.diagnostics.has_errors()
    }

    /// Buduje reprezentacje maszynowa.
    pub fn to_machine(&self) -> MachineReport {
        MachineReport {
            file: self.file.clone(),
            ok: self.is_ok(),
            errors: self.error_count(),
            warnings: self.warning_count(),
            notes: self.diagnostics.count_notes(),
            diagnostics: self
                .diagnostics
                .items
                .iter()
                .map(|d| MachineDiagnostic {
                    code: d.code.code().to_string(),
                    category: d.code.category().to_string(),
                    severity: severity_name(d.severity).to_string(),
                    line: d.line,
                    col: d.col,
                    message: d.message.clone(),
                    suggestion: d.suggestion.clone(),
                    snippet: d.snippet.clone(),
                })
                .collect(),
            stats: self.stats.clone(),
            tokens: self.tokens.clone(),
            optimization: self.optimization.clone(),
            timings: self.timings.clone(),
        }
    }

    /// Wypisuje raport w wybranym formacie.
    pub fn print(&self, format: ReportFormat) {
        match format {
            ReportFormat::Human => self.print_human(),
            ReportFormat::Short => self.print_short(),
            ReportFormat::Json => self.print_json(),
        }
    }

    /// Wypisuje raport czytelny dla czlowieka.
    pub fn print_human(&self) {
        println!("{}", "AXCode Compiler".bright_cyan().bold());
        println!("{} {}", "plik:".dimmed(), self.file.bright_white());
        if let Some(platform) = &self.platform {
            println!("{} {}", "platforma:".dimmed(), platform.bright_white());
        }
        println!();

        if self.diagnostics.items.is_empty() {
            println!("{} {}", "✓".green().bold(), "brak uwag — plik jest poprawny".green());
        } else {
            for diagnostic in &self.diagnostics.items {
                self.print_diagnostic(diagnostic);
            }
        }

        self.print_summary();

        if let Some(stats) = &self.stats {
            self.print_stats(stats);
        }
        if let Some(tokens) = &self.tokens {
            self.print_tokens(tokens);
        }
        if let Some(opt) = &self.optimization {
            println!("{} {}", "optymalizacja:".bold(), opt.summary().bright_white());
            println!();
        }
        if !self.artifacts.is_empty() {
            println!("{}", "wygenerowane pliki:".bold());
            for artifact in &self.artifacts {
                println!("  {}", artifact.bright_white());
            }
            println!();
        }
        if let Some(t) = &self.timings {
            println!(
                "{} odczyt {}ms · lekser {}ms · parser {}ms · weryfikacja {}ms · IR {}ms · optymalizacja {}ms · razem {}ms",
                "czasy:".dimmed(),
                t.read_ms,
                t.lex_ms,
                t.parse_ms,
                t.verify_ms,
                t.ir_ms,
                t.optimize_ms,
                t.total_ms
            );
        }
    }

    /// Wypisuje pojedyncza diagnostyke z kontekstem zrodla.
    fn print_diagnostic(&self, diagnostic: &Diagnostic) {
        let (marker, color_label) = match diagnostic.severity {
            Severity::Error => ("error", "error".red().bold()),
            Severity::Warning => ("warning", "warning".yellow().bold()),
            Severity::Note => ("note", "note".blue().bold()),
        };
        let _ = marker;
        let code = diagnostic.code.code();
        let location = format!("{}:{}:{}", self.file, diagnostic.line, diagnostic.col);

        println!(
            "{} {} {} {}",
            color_label,
            format!("[{}]", code).bright_white().bold(),
            diagnostic.message.bright_white(),
            location.dimmed()
        );
        println!("  {} {}", "kategoria:".dimmed(), diagnostic.code.category().bright_white());

        if let Some(snippet) = &diagnostic.snippet {
            println!();
            for line in snippet.lines() {
                println!("  {}", line.bright_black());
            }
        }

        if let Some(suggestion) = &diagnostic.suggestion {
            println!();
            println!("  {} {}", "sugestia:".green().bold(), suggestion.green());
        }
        println!();
    }

    /// Wypisuje podsumowanie liczbowe.
    fn print_summary(&self) {
        let errors = self.error_count();
        let warnings = self.warning_count();
        let notes = self.diagnostics.count_notes();
        let total = self.diagnostics.items.len();

        let errors_text = if errors > 0 {
            format!("{} bledow", errors).red().bold().to_string()
        } else {
            format!("{} bledow", errors).green().to_string()
        };
        let warnings_text = if warnings > 0 {
            format!("{} ostrzezen", warnings).yellow().bold().to_string()
        } else {
            format!("{} ostrzezen", warnings).green().to_string()
        };

        println!("{} {} · {} · {} uwag łącznie", "podsumowanie:".bold(), errors_text, warnings_text, total);
        let _ = notes;
        println!();
    }

    /// Wypisuje statystyki aplikacji.
    fn print_stats(&self, stats: &IrStats) {
        println!("{}", "struktura aplikacji:".bold());
        let rows = [
            ("ekrany", stats.screens),
            ("komponenty", stats.components),
            ("funkcje", stats.functions),
            ("modele danych", stats.types),
            ("magazyny", stats.stores),
            ("warstwy API", stats.apis),
            ("testy", stats.tests),
            ("wezly interfejsu", stats.nodes),
            ("etykiety dostepnosci", stats.a11y_labels),
        ];
        for (label, value) in rows {
            println!("  {:<22} {}", label.dimmed(), value.to_string().bright_white().bold());
        }
        if stats.nodes > 0 {
            let coverage = (stats.a11y_labels as f64 / stats.nodes as f64 * 100.0).round();
            println!();
            let coverage_text = if coverage >= 100.0 {
                format!("{}%", coverage).green().bold().to_string()
            } else if coverage >= 70.0 {
                format!("{}%", coverage).yellow().bold().to_string()
            } else {
                format!("{}%", coverage).red().bold().to_string()
            };
            println!("  {} {}", "pokrycie etykiet dostepnosci:".bold(), coverage_text);
        }
        println!();
    }

    /// Wypisuje statystyki tokenow.
    fn print_tokens(&self, tokens: &TokenCounts) {
        println!("{}", "leksyka:".bold());
        println!("  {:<22} {}", "tokeny".dimmed(), tokens.total.to_string().bright_white().bold());
        println!("  {:<22} {}", "identyfikatory".dimmed(), tokens.identifiers.to_string().bright_white());
        println!("  {:<22} {}", "literaly tekstowe".dimmed(), tokens.strings.to_string().bright_white());
        println!("  {:<22} {}", "literaly liczbowe".dimmed(), tokens.numbers.to_string().bright_white());
        println!("  {:<22} {}", "slowa kluczowe".dimmed(), tokens.keywords.to_string().bright_white());
        println!("  {:<22} {}", "komentarze".dimmed(), tokens.comments.to_string().bright_white());
        println!();
    }

    /// Wypisuje raport zwarty: jedna linia na uwage.
    pub fn print_short(&self) {
        for diagnostic in &self.diagnostics.items {
            let marker = match diagnostic.severity {
                Severity::Error => "E",
                Severity::Warning => "W",
                Severity::Note => "N",
            };
            println!(
                "{}:{}:{}: {} [{}] {}",
                self.file,
                diagnostic.line,
                diagnostic.col,
                marker,
                diagnostic.code.code(),
                diagnostic.message
            );
        }
        println!(
            "{} bledow, {} ostrzezen",
            self.error_count(),
            self.warning_count()
        );
    }

    /// Wypisuje raport maszynowy.
    pub fn print_json(&self) {
        match serde_json::to_string_pretty(&self.to_machine()) {
            Ok(json) => println!("{}", json),
            Err(error) => eprintln!("nie mozna zbudowac raportu JSON: {}", error),
        }
    }

    /// Zwraca raport jako napis w podanym formacie.
    pub fn render(&self, format: ReportFormat) -> String {
        match format {
            ReportFormat::Json => serde_json::to_string_pretty(&self.to_machine())
                .unwrap_or_else(|_| "{}".to_string()),
            ReportFormat::Short => {
                let mut out = String::new();
                for d in &self.diagnostics.items {
                    out.push_str(&format!(
                        "{}:{}:{}: [{}] {}\n",
                        self.file,
                        d.line,
                        d.col,
                        d.code.code(),
                        d.message
                    ));
                }
                out
            }
            ReportFormat::Human => {
                let mut out = String::new();
                out.push_str(&format!("AXCode Compiler — {}\n\n", self.file));
                for d in &self.diagnostics.items {
                    out.push_str(&format!(
                        "[{}] {} ({}:{})\n",
                        d.code.code(),
                        d.message,
                        d.line,
                        d.col
                    ));
                    if let Some(s) = &d.suggestion {
                        out.push_str(&format!("   sugestia: {}\n", s));
                    }
                }
                out.push_str(&format!(
                    "\nbledy: {} · ostrzezenia: {}\n",
                    self.error_count(),
                    self.warning_count()
                ));
                out
            }
        }
    }
}

fn severity_name(severity: Severity) -> &'static str {
    match severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Note => "note",
    }
}
