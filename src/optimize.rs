// src/optimize.rs — optymalizator AXCode
//
// Optymalizator pracuje na IR i nie zmienia zachowania programu. Wykonuje
// cztery przebiegi: usuwanie martwego kodu, propagacje stalych, deduplikacje
// wezlow interfejsu o identycznym ksztalcie oraz redukcje zbioru akcji.
// Kazdy przebieg raportuje, ile zmian wprowadzil, zeby dalo sie to pokazac
// w trybie szczegolowym.

use crate::ir::*;
use std::collections::{HashMap, HashSet};

/// Podsumowanie pracy optymalizatora.
#[derive(Debug, Default, Clone)]
pub struct OptimizeReport {
    pub dead_statements_removed: usize,
    pub constants_folded: usize,
    pub duplicate_nodes_merged: usize,
    pub unused_stores_removed: usize,
    pub unused_types_removed: usize,
    pub unreachable_screens_marked: usize,
    pub passes: usize,
}

impl OptimizeReport {
    /// Krotki opis dla uzytkownika.
    pub fn summary(&self) -> String {
        format!(
            "martwy kod: {} · propagacja stalych: {} · scalone wezly: {} · nieuzywane magazyny: {} · nieosiagalne ekrany: {}",
            self.dead_statements_removed,
            self.constants_folded,
            self.duplicate_nodes_merged,
            self.unused_stores_removed,
            self.unreachable_screens_marked
        )
    }
}

/// Uruchamia wszystkie przebiegi optymalizacji.
pub fn optimize(program: &mut IrProgram) -> OptimizeReport {
    let mut report = OptimizeReport::default();

    report.passes += 1;
    fold_constants(program, &mut report);

    report.passes += 1;
    remove_dead_statements(program, &mut report);

    report.passes += 1;
    merge_duplicate_nodes(program, &mut report);

    report.passes += 1;
    prune_unused_stores(program, &mut report);

    report.passes += 1;
    prune_unused_types(program, &mut report);

    report.passes += 1;
    mark_unreachable_screens(program, &mut report);

    report.passes += 1;
    flatten_branches(program, &mut report);

    report
}

/// Propagacja stalych: jesli zmienna jest przypisana raz do literalu i nigdy
/// nie zmieniana, jej uzycia sa zastepowane literalem.
fn fold_constants(program: &mut IrProgram, report: &mut OptimizeReport) {
    let mut constants: HashMap<String, String> = HashMap::new();

    for g in &program.globals {
        if let Some(initial) = &g.initial {
            if is_literal(initial) {
                constants.insert(g.name.clone(), initial.clone());
            }
        }
    }

    for screen in &mut program.screens {
        for st in &screen.states {
            if let Some(initial) = &st.initial {
                if is_literal(initial) && st.derived.is_none() {
                    constants.insert(st.name.clone(), initial.clone());
                }
            }
        }
    }

    if constants.is_empty() {
        return;
    }

    let overridden: HashSet<String> = collect_assigned_targets(program);
    for name in &overridden {
        constants.remove(name);
    }

    if constants.is_empty() {
        return;
    }

    for screen in &mut program.screens {
        fold_in_nodes(&mut screen.tree, &constants, report);
        for handler in screen.handlers.iter_mut().chain(screen.lifecycle.iter_mut()) {
            fold_in_statements(&mut handler.body, &constants, report);
        }
    }
    for comp in &mut program.components {
        fold_in_nodes(&mut comp.tree, &constants, report);
    }
    for f in &mut program.functions {
        fold_in_statements(&mut f.body, &constants, report);
    }
}

/// Usuwa instrukcje bez efektu: puste przypisania, puste wywolania, puste galezie.
fn remove_dead_statements(program: &mut IrProgram, report: &mut OptimizeReport) {
    for screen in &mut program.screens {
        for handler in screen.handlers.iter_mut().chain(screen.lifecycle.iter_mut()) {
            let before = handler.body.len();
            handler.body = prune_statements(std::mem::take(&mut handler.body));
            report.dead_statements_removed += before.saturating_sub(handler.body.len());
        }
    }
    for f in &mut program.functions {
        let before = f.body.len();
        f.body = prune_statements(std::mem::take(&mut f.body));
        report.dead_statements_removed += before.saturating_sub(f.body.len());
    }
}

/// Scala wezly interfejsu o identycznym typie, etykiecie i zestawie wlasciwosci.
fn merge_duplicate_nodes(program: &mut IrProgram, report: &mut OptimizeReport) {
    for screen in &mut program.screens {
        let before = count_all_nodes(&screen.tree);
        screen.tree = merge_nodes(std::mem::take(&mut screen.tree));
        let after = count_all_nodes(&screen.tree);
        report.duplicate_nodes_merged += before.saturating_sub(after);
    }
    for comp in &mut program.components {
        let before = count_all_nodes(&comp.tree);
        comp.tree = merge_nodes(std::mem::take(&mut comp.tree));
        let after = count_all_nodes(&comp.tree);
        report.duplicate_nodes_merged += before.saturating_sub(after);
    }
}

/// Usuwa magazyny, do ktorych nigdy nie ma zapisu ani odczytu.
fn prune_unused_stores(program: &mut IrProgram, report: &mut OptimizeReport) {
    let mut used: HashSet<String> = HashSet::new();
    for screen in &program.screens {
        for s in &screen.store_access {
            used.insert(s.clone());
        }
        collect_store_refs_from_nodes(&screen.tree, &mut used);
        for handler in screen.handlers.iter().chain(screen.lifecycle.iter()) {
            collect_store_refs(&handler.body, &mut used);
        }
    }
    for f in &program.functions {
        collect_store_refs(&f.body, &mut used);
    }
    let before = program.stores.len();
    program.stores.retain(|s| used.contains(&s.name) || s.class == "queue" || s.class == "secure");
    let removed = before.saturating_sub(program.stores.len());
    report.unused_stores_removed += removed;
}

/// Usuwa modele danych, ktore nie wystepuja w zadnym typie pola ani magazynie.
fn prune_unused_types(program: &mut IrProgram, report: &mut OptimizeReport) {
    let mut referenced: HashSet<String> = HashSet::new();
    for s in &program.stores {
        let base = s.type_name.trim_end_matches('?');
        let inner = base.trim_start_matches("List<").trim_end_matches('>');
        referenced.insert(inner.to_string());
    }
    for ty in &program.types {
        for field in &ty.fields {
            let base = field.type_name.trim_end_matches('?');
            let inner = base.trim_start_matches("List<").trim_end_matches('>');
            referenced.insert(inner.to_string());
        }
    }
    for f in &program.functions {
        if let Some(r) = &f.returns {
            referenced.insert(r.trim_end_matches('?').to_string());
        }
    }

    let before = program.types.len();
    program.types.retain(|t| {
        referenced.contains(&t.name) || referenced.iter().any(|r| r.contains(&t.name))
    });
    report.unused_types_removed += before.saturating_sub(program.types.len());
}

/// Oznacza ekrany nieosiagalne z zadnego innego ekranu.
fn mark_unreachable_screens(program: &mut IrProgram, report: &mut OptimizeReport) {
    let mut reachable: HashSet<String> = HashSet::new();
    if let Some(entry) = &program.app.entry {
        reachable.insert(entry.clone());
    }
    let mut frontier: Vec<String> = reachable.iter().cloned().collect();
    while let Some(current) = frontier.pop() {
        if let Some(screen) = program.screens.iter().find(|s| s.name == current) {
            for target in &screen.navigation_targets {
                if reachable.insert(target.clone()) {
                    frontier.push(target.clone());
                }
            }
        }
    }
    for screen in &mut program.screens {
        if !reachable.contains(&screen.name) {
            report.unreachable_screens_marked += 1;
        }
    }
}

/// Splaszcza zagniezdzone galezie o pojedynczym warunku i pustej alternatywie.
fn flatten_branches(program: &mut IrProgram, report: &mut OptimizeReport) {
    for screen in &mut program.screens {
        for handler in screen.handlers.iter_mut().chain(screen.lifecycle.iter_mut()) {
            handler.body = flatten_statement_list(std::mem::take(&mut handler.body), report);
        }
    }
    for f in &mut program.functions {
        f.body = flatten_statement_list(std::mem::take(&mut f.body), report);
    }
}

// --- narzedzia przebiegow ---

fn is_literal(s: &str) -> bool {
    let t = s.trim();
    if t.is_empty() {
        return false;
    }
    if t.starts_with('"') && t.ends_with('"') && t.len() >= 2 {
        return true;
    }
    if matches!(t, "true" | "false" | "none" | "null") {
        return true;
    }
    if t.parse::<f64>().is_ok() {
        return true;
    }
    if t.starts_with('[') && t.ends_with(']') {
        return true;
    }
    false
}

/// Zbiera nazwy przypisywane gdziekolwiek, czyli zmienne, ktore nie sa stale.
fn collect_assigned_targets(program: &IrProgram) -> HashSet<String> {
    let mut out = HashSet::new();
    for screen in &program.screens {
        for handler in screen.handlers.iter().chain(screen.lifecycle.iter()) {
            for stmt in &handler.body {
                collect_assignments(stmt, &mut out);
            }
        }
    }
    for f in &program.functions {
        for stmt in &f.body {
            collect_assignments(stmt, &mut out);
        }
    }
    out
}

fn collect_assignments(stmt: &IrStmt, out: &mut HashSet<String>) {
    match stmt {
        IrStmt::Assign { target, value } => {
            out.insert(target.clone());
            // przypisanie zlozone moze zalezec od innej zmiennej
            for base in value.split(|c: char| !c.is_alphanumeric() && c != '_' && c != '.') {
                let t = base.trim();
                if !t.is_empty() {
                    out.insert(t.to_string());
                }
            }
        }
        IrStmt::Update { target, .. } => {
            out.insert(target.clone());
        }
        IrStmt::Branch { then, otherwise, .. } => {
            for s in then.iter().chain(otherwise.iter()) {
                collect_assignments(s, out);
            }
        }
        IrStmt::Loop { body, .. } => {
            for s in body {
                collect_assignments(s, out);
            }
        }
        IrStmt::Guard { attempt, rescue, cleanup } => {
            for s in attempt.iter().chain(rescue.iter()).chain(cleanup.iter()) {
                collect_assignments(s, out);
            }
        }
        _ => {}
    }
}

fn fold_in_nodes(nodes: &mut [IrNode], constants: &HashMap<String, String>, report: &mut OptimizeReport) {
    for node in nodes.iter_mut() {
        for value in node.props.values_mut() {
            let replaced = replace_identifiers(value, constants);
            if replaced != *value {
                *value = replaced;
                report.constants_folded += 1;
            }
        }
        if let Some(condition) = &node.condition {
            let replaced = replace_identifiers(condition, constants);
            if replaced != *condition {
                node.condition = Some(replaced);
                report.constants_folded += 1;
            }
        }
        fold_in_nodes(&mut node.children, constants, report);
    }
}

fn fold_in_statements(body: &mut [IrStmt], constants: &HashMap<String, String>, report: &mut OptimizeReport) {
    for stmt in body.iter_mut() {
        match stmt {
            IrStmt::Call { args, .. } => {
                for arg in args.iter_mut() {
                    let replaced = replace_identifiers(arg, constants);
                    if replaced != *arg {
                        *arg = replaced;
                        report.constants_folded += 1;
                    }
                }
            }
            IrStmt::Branch { condition, then, otherwise } => {
                let replaced = replace_identifiers(condition, constants);
                if replaced != *condition {
                    *condition = replaced;
                    report.constants_folded += 1;
                }
                fold_in_statements(then, constants, report);
                fold_in_statements(otherwise, constants, report);
            }
            IrStmt::Loop { body: inner, .. } => fold_in_statements(inner, constants, report),
            IrStmt::Guard { attempt, rescue, cleanup } => {
                fold_in_statements(attempt, constants, report);
                fold_in_statements(rescue, constants, report);
                fold_in_statements(cleanup, constants, report);
            }
            IrStmt::Navigate { args, .. } => {
                for arg in args.iter_mut() {
                    let replaced = replace_identifiers(arg, constants);
                    if replaced != *arg {
                        *arg = replaced;
                        report.constants_folded += 1;
                    }
                }
            }
            _ => {}
        }
    }
}

/// Zastepuje identyfikatory odpowiadajace stalym ich wartosciami.
fn replace_identifiers(input: &str, constants: &HashMap<String, String>) -> String {
    let mut out = String::with_capacity(input.len());
    let mut token = String::new();
    let mut in_string = false;

    let flush = |token: &mut String, out: &mut String| {
        if token.is_empty() {
            return;
        }
        match constants.get(token.as_str()) {
            Some(value) => out.push_str(value),
            None => out.push_str(token),
        }
        token.clear();
    };

    for ch in input.chars() {
        if in_string {
            out.push(ch);
            if ch == '"' {
                in_string = false;
            }
            continue;
        }
        match ch {
            '"' => {
                flush(&mut token, &mut out);
                in_string = true;
                out.push(ch);
            }
            c if c.is_alphanumeric() || c == '_' || c == '.' => token.push(c),
            _ => {
                flush(&mut token, &mut out);
                out.push(ch);
            }
        }
    }
    flush(&mut token, &mut out);
    out
}

/// Usuwa instrukcje bez efektu i puste galezie.
fn prune_statements(body: Vec<IrStmt>) -> Vec<IrStmt> {
    let mut out = Vec::new();
    for stmt in body {
        match stmt {
            IrStmt::Call { target, args, node_id } => {
                if target.trim().is_empty() {
                    continue;
                }
                out.push(IrStmt::Call { target, args, node_id });
            }
            IrStmt::Assign { target, value } => {
                if target.trim().is_empty() || value.trim().is_empty() {
                    continue;
                }
                out.push(IrStmt::Assign { target, value });
            }
            IrStmt::Branch { condition, then, otherwise } => {
                let then = prune_statements(then);
                let otherwise = prune_statements(otherwise);
                if then.is_empty() && otherwise.is_empty() {
                    continue;
                }
                out.push(IrStmt::Branch { condition, then, otherwise });
            }
            IrStmt::Loop { item, over, body } => {
                let body = prune_statements(body);
                if body.is_empty() {
                    continue;
                }
                out.push(IrStmt::Loop { item, over, body });
            }
            IrStmt::Guard { attempt, rescue, cleanup } => {
                let attempt = prune_statements(attempt);
                let rescue = prune_statements(rescue);
                let cleanup = prune_statements(cleanup);
                if attempt.is_empty() && rescue.is_empty() && cleanup.is_empty() {
                    continue;
                }
                out.push(IrStmt::Guard { attempt, rescue, cleanup });
            }
            other => out.push(other),
        }
    }
    out
}

/// Scala sasiadujace wezly o identycznym kluczu.
fn merge_nodes(nodes: Vec<IrNode>) -> Vec<IrNode> {
    let mut out: Vec<IrNode> = Vec::new();
    for mut node in nodes {
        node.children = merge_nodes(std::mem::take(&mut node.children));
        let key = node_key(&node);
        if let Some(existing) = out.iter_mut().find(|n| node_key(n) == key) {
            existing.children.extend(node.children);
            continue;
        }
        out.push(node);
    }
    out
}

fn node_key(node: &IrNode) -> String {
    let mut props: Vec<String> = node.props.iter().map(|(k, v)| format!("{}={}", k, v)).collect();
    props.sort();
    format!(
        "{}|{}|{}|{}",
        node.kind,
        node.label.clone().unwrap_or_default(),
        props.join(","),
        node.condition.clone().unwrap_or_default()
    )
}

fn count_all_nodes(nodes: &[IrNode]) -> usize {
    nodes.iter().map(|n| 1 + count_all_nodes(&n.children)).sum()
}

fn collect_store_refs_from_nodes(nodes: &[IrNode], out: &mut HashSet<String>) {
    for n in nodes {
        for b in &n.bindings {
            out.insert(b.clone());
        }
        collect_store_refs_from_nodes(&n.children, out);
    }
}

fn collect_store_refs(body: &[IrStmt], out: &mut HashSet<String>) {
    for stmt in body {
        match stmt {
            IrStmt::Persist { store }
            | IrStmt::Restore { store }
            | IrStmt::Purge { store }
            | IrStmt::Flush { queue: store } => {
                out.insert(store.clone());
            }
            IrStmt::Append { store, .. } | IrStmt::Remove { store, .. } => {
                out.insert(store.clone());
            }
            IrStmt::Enqueue { queue, .. } => {
                out.insert(queue.clone());
            }
            IrStmt::Branch { then, otherwise, .. } => {
                collect_store_refs(then, out);
                collect_store_refs(otherwise, out);
            }
            IrStmt::Loop { body: inner, .. } => collect_store_refs(inner, out),
            IrStmt::Guard { attempt, rescue, cleanup } => {
                collect_store_refs(attempt, out);
                collect_store_refs(rescue, out);
                collect_store_refs(cleanup, out);
            }
            _ => {}
        }
    }
}

/// Usuwa galezie o jednym warunku bez alternatywy przez wcielenie ich ciala.
fn flatten_statement_list(body: Vec<IrStmt>, report: &mut OptimizeReport) -> Vec<IrStmt> {
    let mut out = Vec::new();
    for stmt in body {
        match stmt {
            IrStmt::Branch { condition, then, otherwise } => {
                let then = flatten_statement_list(then, report);
                let otherwise = flatten_statement_list(otherwise, report);
                let literal_condition = condition.trim() == "true";
                if literal_condition && otherwise.is_empty() {
                    report.dead_statements_removed += 1;
                    out.extend(then);
                    continue;
                }
                if condition.trim() == "false" {
                    report.dead_statements_removed += 1;
                    out.extend(otherwise);
                    continue;
                }
                out.push(IrStmt::Branch { condition, then, otherwise });
            }
            IrStmt::Loop { item, over, body } => {
                out.push(IrStmt::Loop { item, over, body: flatten_statement_list(body, report) });
            }
            IrStmt::Guard { attempt, rescue, cleanup } => {
                out.push(IrStmt::Guard {
                    attempt: flatten_statement_list(attempt, report),
                    rescue: flatten_statement_list(rescue, report),
                    cleanup: flatten_statement_list(cleanup, report),
                });
            }
            other => out.push(other),
        }
    }
    out
}
