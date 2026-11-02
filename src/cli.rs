// src/cli.rs — potok kompilacji
//
// Jedno miejsce, w ktorym plik .ax przechodzi przez wszystkie etapy:
// odczyt, lekser, kontrola wciec, parser, weryfikacja semantyczna,
// analiza typow i zebranie diagnostyk. HIR, IR i optymalizacja
// uruchamiane sa w main.rs, gdy plik jest poprawny.

use crate::ast::Program;
use crate::diagnostics::Diagnostics;
use crate::lexer::{self, TokenCounts};
use crate::optimize::OptimizeReport;
use crate::parser::Parser;
use crate::report::MachineTimings;
use crate::types;
use crate::verifier;
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;
use std::time::Instant;

/// Wynik potoku kompilacji: drzewo, opcjonalne reprezentacje i diagnostyki.
pub struct PipelineResult {
    pub program: Option<Program>,
    pub hir: Option<crate::hir::HirProgram>,
    pub ir: Option<crate::ir::IrProgram>,
    pub diagnostics: Diagnostics,
    pub timings: MachineTimings,
    pub tokens: Option<TokenCounts>,
    pub stats: Option<crate::ir::IrStats>,
    pub optimization: Option<OptimizeReport>,
}

impl PipelineResult {
    /// Wynik pusty, uzywany gdy odczyt pliku sie nie powiodl.
    pub fn empty() -> Self {
        Self {
            program: None,
            hir: None,
            ir: None,
            diagnostics: Diagnostics::new(),
            timings: MachineTimings::default(),
            tokens: None,
            stats: None,
            optimization: None,
        }
    }
}

/// Uruchamia pelny potok kompilacji na pliku.
pub fn compile_file(path: &Path, collect_tokens: bool) -> Result<PipelineResult> {
    let start = Instant::now();
    let mut timings = MachineTimings::default();

    let read_start = Instant::now();
    let source = fs::read_to_string(path)
        .with_context(|| format!("nie mozna odczytac pliku {}", path.display()))?;
    timings.read_ms = read_start.elapsed().as_millis();

    let mut diagnostics = Diagnostics::new();

    // etap leksykalny
    let lex_start = Instant::now();
    let tokens = {
        let mut lx = lexer::Lexer::new(&source, &mut diagnostics);
        lx.tokenize()
    };
    timings.lex_ms = lex_start.elapsed().as_millis();

    let token_counts = if collect_tokens {
        Some(lexer::count(&source, &tokens))
    } else {
        None
    };

    // kontrola wciec
    lexer::check_indentation(&tokens, &mut diagnostics);

    // etap skladniowy
    let parse_start = Instant::now();
    let program = {
        let mut parser = Parser::new(&source, &mut diagnostics);
        parser.parse()
    };
    timings.parse_ms = parse_start.elapsed().as_millis();

    // etap semantyczny
    let verify_start = Instant::now();
    let verify_result = verifier::verify(&program);
    for item in verify_result.diagnostics.items {
        diagnostics.push_diagnostic(item);
    }
    types::check_types(&program, &mut diagnostics);
    timings.verify_ms = verify_start.elapsed().as_millis();

    diagnostics.attach_snippets(&source);
    timings.total_ms = start.elapsed().as_millis();

    Ok(PipelineResult {
        program: Some(program),
        hir: None,
        ir: None,
        diagnostics,
        timings,
        tokens: token_counts,
        stats: None,
        optimization: None,
    })
}
