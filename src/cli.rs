// src/cli.rs — obsluga polecen wiersza polecenia
//
// Kazda komenda czyta plik, przepuszcza go przez potok kompilacji i wypisuje
// wynik w wybranym formacie. Komendy diagnostyczne (lex, parse, ast, hir, ir)
// pozwalaja zatrzymac sie na wybranym etapie i obejrzec reprezentacje.

use crate::ast::Program;
use crate::backend::{self, Target};
use crate::diagnostics::{Diagnostic, Diagnostics, ErrorCatalog, Severity};
use crate::hir::{self, HirProgram};
use crate::ir::{self, IrProgram};
use crate::lexer::{self, TokenCounts};
use crate::optimize;
use crate::parser::Parser;
use crate::report::{Report, ReportFormat};
use crate::runtime;
use crate::verifier;
use anyhow::{Context, Result};
use colored::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

/// Wynik pelnego potoku kompilacji.
pub struct PipelineResult {
    pub program: Option<Program>,
    pub hir: Option<HirProgram>,
    pub ir: Option<IrProgram>,
    pub diagnostics: Diagnostics,
    pub timings: Timings,
}

#[derive(Default, Debug, Clone)]
pub struct Timings {
    pub read_ms: u128,
    pub lex_ms: u128,
    pub parse_ms: u128,
    pub verify_ms: u128,
    pub hir_ms: u128,
    pub ir_ms: u128,
    pub optimize_ms: u128,
    pub total_ms: u128,
}

impl Timings {
    pub fn report(&self) -> String {
        format!(
            "odczyt {}ms · lekser {}ms · parser {}ms · weryfikacja {}ms · HIR {}ms · IR {}ms · optymalizacja {}ms\n\
             wszystko razem: {}ms",
            self.read_ms,
            self.lex_ms,
            self.parse_ms,
            self.verify_ms,
            self.hir_ms,
            self.ir_ms,
            self.optimize_ms,
            self.total_ms
        )
    }
}

/// Uruchamia pelny potok kompilacji na pliku.
pub fn compile_file(path: &Path, with_ir: bool) -> Result<PipelineResult> {
    let start = Instant::now();
    let mut timings = Timings::default();

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
        diagnostics.push(item);
    }
    crate::types::check_types(&program, &mut diagnostics);
    timings.verify_ms = verify_start.elapsed().as_millis();

    diagnostics.attach_snippets(&source);

    if diagnostics.has_errors() {
        timings.total_ms = start.elapsed().as_millis();
        return Ok(PipelineResult {
            program: Some(program),
            hir: None,
            ir: None,
            diagnostics,
            tim
