// src/main.rs — punkt wejscia kompilatora AXCode
use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use colored::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

mod ast;
mod lexer;
mod parser;
mod verifier;
mod types;
mod diagnostics;
mod emitter;
mod hir;
mod optimize;
mod runtime;
mod cli;
mod report;
mod ir;
mod backend;

use diagnostics::{Diagnostic, Diagnostics, Severity};
use report::{Report, ReportFormat};

#[derive(Parser)]
#[command(
    name = "axc",
    version = "1.0.0",
    about = "AXCode Compiler — kompilator deklaratywnego jezyka aplikacji",
    long_about = "AXC kompiluje pliki .ax do reprezentacji posredniej, wykonuje pelna analize \
                  statyczna (dostepnosc, bezpieczenstwo, typy, magazyny, nawigacja) i generuje \
                  kod dla platform docelowych.\n\n\
                  Kolejnosc etapow: lekser → parser → analiza semantyczna → HIR → IR → backend.\n\
                  Wszystkie bledy maja kody AX1000-AX9999 wedlug specyfikacji jezyka.",
    author = "Axel G"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Sciezka do pliku .ax
    #[arg(value_name = "PLIK")]
    file: Option<PathBuf>,

    /// Tryb analizy statycznej bez emisji kodu
    #[arg(long, short)]
    check: bool,

    /// Format raportu
    #[arg(long, short, value_enum, default_value = "human")]
    format: ReportFormat,

    /// Poziom szczegolowosci
    #[arg(long, short, action = clap::ArgAction::Count)]
    verbose: u8,

    /// Zapis HIR do pliku JSON
    #[arg(long)]
    emit_hir: bool,

    /// Zapis IR do pliku JSON
    #[arg(long)]
    emit_ir: bool,

    /// Katalog wyjscia
    #[arg(long, short, default_value = ".")]
    output: PathBuf,
}

#[derive(Subcommand)]
enum Commands {
    /// Analiza statyczna pliku bez generowania kodu
    Check {
        #[arg(value_name = "PLIK")]
        file: PathBuf,
        #[arg(long, short, value_enum, default_value = "human")]
        format: ReportFormat,
    },

    /// Wypis drzewa skladniowego
    Parse {
        #[arg(value_name = "PLIK")]
        file: PathBuf,
        /// Poziom glebokosci drzewa
        #[arg(long, default_value = "10")]
        depth: usize,
    },

    /// Wypis tokenow z pozycjami
    Lex {
        #[arg(value_name = "PLIK")]
        file: PathBuf,
    },

    /// Wypis reprezentacji posredniej HIR
    Hir {
        #[arg(value_name = "PLIK")]
        file: PathBuf,
        #[arg(long)]
        json: bool,
    },

    /// Wypis reprezentacji IR po optymalizacjach
    Ir {
        #[arg(value_name = "PLIK")]
        file: PathBuf,
        #[arg(long)]
        json: bool,
    },

    /// Wypis rozszerzonego drzewa AST jako JSON
    Ast {
        #[arg(value_name = "PLIK")]
        file: PathBuf,
    },

    /// Wykonuje testy zadeklarowane w pliku
    Test {
        #[arg(value_name = "PLIK")]
        file: PathBuf,
    },

    /// Generuje kod dla platformy docelowej
    Build {
        #[arg(value_name = "PLIK")]
        file: PathBuf,
        /// Platforma docelowa
        #[arg(long, short, default_value = "web")]
        target: String,
        #[arg(long, short, default_value = ".")]
        output: PathBuf,
    },

    /// Wypis statystyk pliku
    Stats {
        #[arg(value_name = "PLIK")]
        file: PathBuf,
    },

    /// Formatuje plik wedlug konwencji jezyka
    Format {
        #[arg(value_name = "PLIK")]
        file: PathBuf,
        #[arg(long)]
        write: bool,
    },

    /// Informacje o wersji i mozliwosciach
    Info,
}

fn main() {
    if let Err(e) = run() {
        eprintln!("{} {}", "blad:".red().bold(), e);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Info) => {
            print_info();
            Ok(())
        }
        Some(Commands::Lex { file }) => cmd_lex(&file, cli.verbose),
        Some(Commands::Parse { file, depth }) => cmd_parse(&file, depth, cli.verbose),
        Some(Commands::Ast { file }) => cmd_ast(&file),
        Some(Commands::Hir { file, json }) => cmd_hir(&file, json),
        Some(Commands::Ir { file, json }) => cmd_ir(&file, json),
        Some(Commands::Check { file, format }) => cmd_check(&file, format, cli.verbose),
        Some(Commands::Test { file }) => cmd_test(&file, cli.verbose),
        Some(Commands::Stats { file }) => cmd_stats(&file),
        Some(Commands::Format { file, write }) => cmd_format(&file, write),
        Some(Commands::Build { file, target, output }) => cmd_build(&file, &target, &output),
        None => {
            if let Some(file) = cli.file {
                if cli.check {
                    cmd_check(&file, cli.format, cli.verbose)
                } else {
                    cmd_build(&file, "web", &cli.output)
                }
            } else {
                print_info();
                Ok(())
            }
        }
    }
}

fn read_source(path: &Path) -> Result<String> {
    fs::read_to_string(path)
        .with_context(|| format!("nie mozna odczytac pliku {}", path.display()))
}

fn print_info() {
    println!("{}", "AXCode Compiler v1.0.0".bright_cyan().bold());
    println!("{}", "deklaratywny jezyk aplikacji — kompilator w Rust".dimmed());
    println!();
    println!("{}", "Etapy kompilacji:".bold());
    println!("  1. Lekser              — tekst zrodlowy na tokeny");
    println!("  2. Parser              — tokeny na drzewo skladniowe");
    println!("  3. Analiza semantyczna — typy, magazyny, dostepnosc, bezpieczenstwo");
    println!("  4. HIR                 — drzewo wysokopoziomowe z rozwiazanymi nazwami");
    println!("  5. IR                  — reprezentacja posrednia niezalezna od platformy");
    println!("  6. Optymalizator       — usuwanie martwego kodu, propagacja stalych");
    println!("  7. Backend             — emisja kodu dla platformy docelowej");
    println!();
    println!("{}", "Jednostki jezyka:".bold());
    let units = [
        ("app", "deklaracja aplikacji"),
        ("theme", "tokeny designu"),
        ("style", "styl nazwany"),
        ("enum", "typ wyliczeniowy"),
        ("data", "model danych"),
        ("store", "trwaly magazyn"),
        ("global state", "stan globalny"),
        ("permission", "uprawnienie systemowe"),
        ("api", "warstwa API"),
        ("function", "funkcja"),
        ("component", "komponent interfejsu"),
        ("screen", "ekran"),
        ("test", "test jednostkowy, interfejsu lub dostepnosci"),
    ];
    for (name, desc) in units {
        println!("  {:<14} {}", name.bright_yellow(), desc);
    }
    println!();
    println!("{}", "Kody bledow:".bold());
    println!("  AX1000-AX1999  skladnia");
    println!("  AX2000-AX2999  typy");
    println!("  AX3000-AX3999  interfejs i layout");
    println!("  AX4000-AX4999  stan i magazyny");
    println!("  AX5000-AX5999  API i siec");
    println!("  AX6000-AX6999  bezpieczenstwo");
    println!("  AX7000-AX7999  dostepnosc");
    println!("  AX8000-AX8999  platforma i nawigacja");
    println!("  AX9000-AX9999  wydajnosc");
    println!();
    println!("{}", "Uzycie: axc <PLIK> [--check] [--format human|json|short]".dimmed());
}
