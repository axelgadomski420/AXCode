// src/ir.rs — reprezentacja posrednia AXCode
//
// IR jest niezalezny od platformy: opisuje aplikacje jako graf jednostek,
// bez odwolan do DOM, UIKit czy Jetpack Compose. Backendy tlumacza ten graf
// na konkretny jezyk docelowy. IR jest tez formatem wymiany dla podgladu
// na zywo i dla testow.

use crate::ast::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Jednostka IR: stabilny identyfikator i dane.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrProgram {
    pub app: IrApp,
    pub theme: IrTheme,
    pub styles: Vec<IrStyle>,
    pub types: Vec<IrType>,
    pub enums: Vec<IrEnum>,
    pub stores: Vec<IrStore>,
    pub globals: Vec<IrGlobal>,
    pub permissions: Vec<IrPermission>,
    pub apis: Vec<IrApi>,
    pub functions: Vec<IrFunction>,
    pub components: Vec<IrComponent>,
    pub screens: Vec<IrScreen>,
    pub tests: Vec<IrTest>,
    pub stats: IrStats,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrApp {
    pub id: String,
    pub name: String,
    pub version: String,
    pub language: String,
    pub targets: Vec<String>,
    pub orientation: String,
    pub offline: String,
    pub entry: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrTheme {
    pub name: String,
    pub follows_system: bool,
    pub colors: HashMap<String, HashMap<String, String>>,
    pub radius: HashMap<String, f64>,
    pub fonts: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrStyle {
    pub id: String,
    pub name: String,
    pub properties: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrType {
    pub id: String,
    pub name: String,
    pub fields: Vec<IrField>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrField {
    pub name: String,
    pub type_name: String,
    pub optional: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrEnum {
    pub id: String,
    pub name: String,
    pub variants: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrStore {
    pub id: String,
    pub name: String,
    pub class: String,
    pub encrypted: bool,
    pub shape: String,
    pub type_name: String,
    pub key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrGlobal {
    pub name: String,
    pub type_name: String,
    pub initial: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrPermission {
    pub kind: String,
    pub purpose: String,
    pub required: bool,
    pub declaration: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrApi {
    pub id: String,
    pub name: String,
    pub base: String,
    pub auth: Option<String>,
    pub timeout_ms: u64,
    pub retry: u32,
    pub functions: Vec<IrApiFunction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrApiFunction {
    pub name: String,
    pub method: String,
    pub route: String,
    pub params: Vec<String>,
    pub returns: Option<String>,
    pub cacheable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrFunction {
    pub id: String,
    pub name: String,
    pub params: Vec<String>,
    pub returns: Option<String>,
    pub body: Vec<IrStmt>,
    pub calls: Vec<String>,
    pub side_effects: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrComponent {
    pub id: String,
    pub name: String,
    pub params: Vec<String>,
    pub tree: Vec<IrNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrScreen {
    pub id: String,
    pub name: String,
    pub title: String,
    pub params: Vec<String>,
    pub states: Vec<IrState>,
    pub tree: Vec<IrNode>,
    pub handlers: Vec<IrHandler>,
    pub lifecycle: Vec<IrHandler>,
    pub store_access: Vec<String>,
    pub navigation_targets: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrState {
    pub name: String,
    pub type_name: String,
    pub initial: Option<String>,
    pub derived: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrNode {
    pub id: String,
    pub kind: String,
    pub label: Option<String>,
    pub props: HashMap<String, String>,
    pub accessibility: IrAccessibility,
    pub children: Vec<IrNode>,
    pub condition: Option<String>,
    pub bindings: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IrAccessibility {
    pub label: Option<String>,
    pub alt: Option<String>,
    pub role: Option<String>,
    pub hint: Option<String>,
    pub live_region: Option<String>,
    pub hidden: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrHandler {
    pub event: String,
    pub node_id: Option<String>,
    pub body: Vec<IrStmt>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IrStmt {
    Assign { target: String, value: String },
    Call { target: String, args: Vec<String>, node_id: String },
    Append { value: String, store: String },
    Remove { value: String, store: String },
    Update { target: String, value: String },
    Persist { store: String },
    Restore { store: String },
    Purge { store: String },
    Enqueue { item: String, queue: String },
    Flush { queue: String },
    Navigate { mode: String, screen: Option<String>, args: Vec<String> },
    Present { target: String, mode: String },
    Return { value: Option<String> },
    Announce { text: String, urgent: bool },
    RequestPermission { kind: String },
    Vibrate { pattern: String },
    Branch { condition: String, then: Vec<IrStmt>, otherwise: Vec<IrStmt> },
    Loop { item: String, over: String, body: Vec<IrStmt> },
    Guard { attempt: Vec<IrStmt>, rescue: Vec<IrStmt>, cleanup: Vec<IrStmt> },
    Derive { target: String, expression: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrTest {
    pub name: String,
    pub kind: String,
    pub steps: Vec<IrTestStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrTestStep {
    pub kind: String,
    pub text: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IrStats {
    pub screens: usize,
    pub components: usize,
    pub functions: usize,
    pub types: usize,
    pub stores: usize,
    pub apis: usize,
    pub tests: usize,
    pub nodes: usize,
    pub a11y_labels: usize,
}

/// Konwersja drzewa skladniowego na IR.
pub fn lower(program: &Program) -> IrProgram {
    let mut id_counter = 0usize;

    let app = IrApp {
        id: "app#1".to_string(),
        name: program.app.name.clone().unwrap_or_else(|| "AXCode App".to_string()),
        version: program.app.version.clone().unwrap_or_else(|| "0.0.0".to_string()),
        language: program.app.language.clone().unwrap_or_else(|| "pl".to_string()),
        targets: program.app.supports.clone(),
        orientation: program.app.orientation.clone().unwrap_or_else(|| "automatic".to_string()),
        offline: program.app.offline.clone().unwrap_or_else(|| "optional".to_string()),
        entry: program.app.entry.clone().or_else(|| program.screens.first().map(|s| s.name.clone())),
    };

    let theme = program
        .themes
        .first()
        .map(|t| IrTheme {
            name: t.name.clone(),
            follows_system: t.follows_system,
            colors: t.colors.clone(),
            radius: t.radius.clone(),
            fonts: t.fonts.clone(),
        })
        .unwrap_or(IrTheme {
            name: "default".to_string(),
            follows_system: true,
            colors: HashMap::new(),
            radius: HashMap::new(),
            fonts: HashMap::new(),
        });

    let styles = program
        .styles
        .iter()
        .map(|s| {
            let mut properties = HashMap::new();
            for p in &s.props {
                properties.insert(p.key.clone(), p.value.clone());
            }
            IrStyle { id: format!("style.{}", s.name), name: s.name.clone(), properties }
        })
        .collect();

    let types = program
        .data
        .iter()
        .map(|d| IrType {
            id: format!("type.{}", d.name),
            name: d.name.clone(),
            fields: d
                .fields
                .iter()
                .map(|f| IrField {
                    name: f.name.clone(),
                    type_name: type_name(&f.ty),
                    optional: matches!(f.ty, TypeRef::Optional(_)),
                })
                .collect(),
        })
        .collect();

    let enums = program
        .enums
        .iter()
        .map(|e| IrEnum {
            id: format!("enum.{}", e.name),
            name: e.name.clone(),
            variants: e.variants.iter().map(|v| v.name.clone()).collect(),
        })
        .collect();

    let stores = program
        .stores
        .iter()
        .map(|s| IrStore {
            id: format!("store.{}", s.name),
            name: s.name.clone(),
            class: store_class_name(s.class).to_string(),
            encrypted: s.encryption,
            shape: match s.shape {
                StoreShape::Single(_) => "single",
                StoreShape::List(_) => "list",
                StoreShape::Map(_, _) => "map",
            }
            .to_string(),
            type_name: type_name(&s.shape.inner_type()),
            key: s.key.clone().unwrap_or_else(|| format!("app.{}", s.name)),
        })
        .collect();

    let globals = program
        .globals
        .iter()
        .map(|g| IrGlobal {
            name: g.name.clone(),
            type_name: type_name(&g.ty),
            initial: g.initial.clone(),
        })
        .collect();

    let permissions = program
        .permissions
        .iter()
        .map(|p| IrPermission {
            kind: p.kind.clone(),
            purpose: p.purpose.clone(),
            required: p.required,
            declaration: permission_declaration(&p.kind).to_string(),
        })
        .collect();

    let apis = program
        .apis
        .iter()
        .map(|a| IrApi {
            id: format!("api.{}", a.name),
            name: a.name.clone(),
            base: a.base.clone().unwrap_or_default(),
            auth: a.auth.clone(),
            timeout_ms: a.timeout_ms.unwrap_or(15000),
            retry: a.retry.unwrap_or(2),
            functions: a
                .functions
                .iter()
                .map(|f| IrApiFunction {
                    name: f.name.clone(),
                    method: f.method.clone().unwrap_or_else(|| "GET".to_string()),
                    route: f.route.clone().unwrap_or_default(),
                    params: f.params.iter().map(|p| p.name.clone()).collect(),
                    returns: f.returns.as_ref().map(type_name),
                    cacheable: f.method.as_deref() == Some("GET"),
                })
                .collect(),
        })
        .collect();

    let functions = program
        .functions
        .iter()
        .map(|f| IrFunction {
            id: format!("fn.{}", f.name),
            name: f.name.clone(),
            params: f.params.iter().map(|p| format!("{} {}", p.name, type_name(&p.ty))).collect(),
            returns: f.returns.as_ref().map(type_name),
            body: lower_statements(&f.body, &mut id_counter),
            calls: collect_calls(&f.body),
            side_effects: collect_effects(&f.body),
        })
        .collect();

    let components = program
        .components
        .iter()
        .map(|c| IrComponent {
            id: format!("component.{}", c.name),
            name: c.name.clone(),
            params: c.params.iter().map(|p| format!("{} {}", p.name, type_name(&p.ty))).collect(),
            tree: lower_nodes(&c.body, &mut id_counter),
        })
        .collect();

    let screens = program
        .screens
        .iter()
        .map(|s| {
            let mut handlers = Vec::new();
            let mut lifecycle = Vec::new();
            collect_handlers(&s.body, None, &mut handlers, &mut lifecycle, &mut id_counter);
            IrScreen {
                id: format!("screen.{}", s.name),
                name: s.name.clone(),
                title: s.title.clone().unwrap_or_else(|| s.name.clone()),
                params: s.params.iter().map(|p| format!("{} {}", p.name, type_name(&p.ty))).collect(),
                states: s
                    .states
                    .iter()
                    .map(|st| IrState {
                        name: st.name.clone(),
                        type_name: type_name(&st.ty),
                        initial: st.initial.clone(),
                        derived: st.derive.clone(),
                    })
                    .collect(),
                tree: lower_nodes(&s.body, &mut id_counter),
                handlers,
                lifecycle,
                store_access: collect_store_access(&s.body),
                navigation_targets: collect_navigation(&s.body),
            }
        })
        .collect::<Vec<_>>();

    let tests = program
        .tests
        .iter()
        .map(|t| IrTest {
            name: t.name.clone(),
            kind: match t.kind {
                TestKind::Unit => "unit",
                TestKind::Ui => "ui",
                TestKind::Accessibility => "accessibility",
            }
            .to_string(),
            steps: t
                .steps
                .iter()
                .map(|s| IrTestStep {
                    kind: match s.kind {
                        StepKind::Given => "given",
                        StepKind::When => "when",
                        StepKind::Then => "then",
                        StepKind::Expect => "expect",
                        StepKind::Open => "open",
                        StepKind::Fill => "fill",
                        StepKind::Tap => "tap",
                        StepKind::Type => "type",
                        StepKind::Raw => "raw",
                    }
                    .to_string(),
                    text: s.text.clone(),
                })
                .collect(),
        })
        .collect::<Vec<_>>();

    let mut stats = IrStats {
        screens: screens.len(),
        components: components.len(),
        functions: functions.len(),
        types: types.len(),
        stores: stores.len(),
        apis: apis.len(),
        tests: tests.len(),
        nodes: 0,
        a11y_labels: 0,
    };
    for s in &screens {
        count_nodes(&s.tree, &mut stats);
    }
    for c in &components {
        count_nodes(&c.tree, &mut stats);
    }

    IrProgram { app, theme, styles, types, enums, stores, globals, permissions, apis, functions, components, screens, tests, stats }
}

fn count_nodes(nodes: &[IrNode], stats: &mut IrStats) {
    for n in nodes {
        stats.nodes += 1;
        if n.accessibility.label.is_some() || n.accessibility.alt.is_some() {
            stats.a11y_labels += 1;
        }
        count_nodes(&n.children, stats);
    }
}

/// Buduje wezly drzewa interfejsu z instrukcji.
fn lower_nodes(body: &[Stmt], counter: &mut usize) -> Vec<IrNode> {
    let mut out = Vec::new();
    let mut pending_props: HashMap<String, String> = HashMap::new();
    let mut pending_a11y = IrAccessibility::default();

    for stmt in body {
        match stmt {
            Stmt::Property { key, .. } => {
                let (k, v) = match key.split_once(' ') {
                    Some((k, v)) => (k.to_string(), v.to_string()),
                    None => (key.clone(), String::new()),
                };
                match k.as_str() {
                    "alt" => pending_a11y.alt = Some(v.trim_matches('"').to_string()),
                    "label" => pending_a11y.label = Some(v.trim_matches('"').to_string()),
                    "role" => pending_a11y.role = Some(v.to_string()),
                    "accessibility" => {
                        if let Some(rest) = v.strip_prefix("label ") {
                            pending_a11y.label = Some(rest.trim_matches('"').to_string());
                        } else if let Some(rest) = v.strip_prefix("hint ") {
                            pending_a11y.hint = Some(rest.trim_matches('"').to_string());
                        } else {
                            pending_a11y.label = Some(v.trim_matches('"').to_string());
                        }
                    }
                    _ => {
                        pending_props.insert(k, v);
                    }
                }
            }
            Stmt::Widget { kind, label, props, .. } => {
                let mut merged = pending_props.clone();
                let mut a11y = pending_a11y.clone();
                for p in props {
                    merged.insert(p.key.clone(), p.value.clone());
                }
                *counter += 1;
                let mut node = IrNode {
                    id: format!("{}#{}", kind, counter),
                    kind: kind.clone(),
                    label: if label.is_empty() { None } else { Some(label.clone()) },
                    props: merged,
                    accessibility: a11y.clone(),
                    children: Vec::new(),
                    condition: None,
                    bindings: Vec::new(),
                };
                if node.accessibility.label.is_none() && kind == "button" {
                    node.accessibility.label = node.label.clone();
                }
                if node.accessibility.alt.is_none() && kind == "image" {
                    node.accessibility.alt = node.label.clone();
                }
                out.push(node);
                pending_props.clear();
                pending_a11y = IrAccessibility::default();
            }
            Stmt::Container { kind, target, props, body: inner, .. } => {
                let mut merged = pending_props.clone();
                for p in props {
                    merged.insert(p.key.clone(), p.value.clone());
                }
                let children = lower_nodes(inner, counter);
                let bindings = if target.trim().is_empty() {
                    Vec::new()
                } else {
                    vec![target.trim().to_string()]
                };
                *counter += 1;
                out.push(IrNode {
                    id: format!("{}#{}", kind, counter),
                    kind: kind.clone(),
                    label: if target.trim().is_empty() { None } else { Some(target.trim().to_string()) },
                    props: merged,
                    accessibility: pending_a11y.clone(),
                    children,
                    condition: None,
                    bindings,
                });
                pending_props.clear();
                pending_a11y = IrAccessibility::default();
            }
            Stmt::Block { header, body: inner, .. } => {
                if header.starts_with("if ") {
                    let condition = header.trim_start_matches("if ").to_string();
                    let children = lower_nodes(inner, counter);
                    *counter += 1;
                    out.push(IrNode {
                        id: format!("conditional#{}", counter),
                        kind: "conditional".to_string(),
                        label: None,
                        props: HashMap::new(),
                        accessibility: IrAccessibility::default(),
                        children,
                        condition: Some(condition),
                        bindings: Vec::new(),
                    });
                } else {
                    out.extend(lower_nodes(inner, counter));
                }
            }
            _ => {}
        }
    }
    out
}

/// Zbiera obsluge zdarzen z drzewa instrukcji.
fn collect_handlers(
    body: &[Stmt],
    node_id: Option<String>,
    handlers: &mut Vec<IrHandler>,
    lifecycle: &mut Vec<IrHandler>,
    counter: &mut usize,
) {
    let mut current_node = node_id.clone();
    for stmt in body {
        match stmt {
            Stmt::Widget { kind, line, .. } => {
                current_node = Some(format!("{}@{}", kind, line));
            }
            Stmt::Container { kind, body: inner, line, .. } => {
                collect_handlers(inner, Some(format!("{}@{}", kind, line)), handlers, lifecycle, counter);
            }
            Stmt::When { event, body: inner, .. } => {
                let lowered = lower_statements(inner, counter);
                let handler = IrHandler { event: event.clone(), node_id: current_node.clone(), body: lowered };
                let head = event.split_whitespace().next().unwrap_or("");
                if matches!(head, "screen" | "app") {
                    lifecycle.push(handler);
                } else {
                    handlers.push(handler);
                }
            }
            Stmt::Block { body: inner, .. } => {
                collect_handlers(inner, current_node.clone(), handlers, lifecycle, counter);
            }
            _ => {}
        }
    }
}

/// Splaszcza liste instrukcji do IR.
fn lower_statements(body: &[Stmt], counter: &mut usize) -> Vec<IrStmt> {
    let mut out = Vec::new();
    let mut index = 0usize;
    while index < body.len() {
        let stmt = &body[index];
        match stmt {
            Stmt::Set { expr, .. } => {
                if let Some((l, r)) = expr.split_once('=') {
                    out.push(IrStmt::Assign { target: l.trim().to_string(), value: r.trim().to_string() });
                }
            }
            Stmt::Call { name, args, line } => {
                out.push(IrStmt::Call {
                    target: name.clone(),
                    args: split_args(args),
                    node_id: format!("call@{}", line),
                });
            }
            Stmt::Add { expr, .. } => {
                if let Some((v, s)) = expr.split_once(" to ") {
                    out.push(IrStmt::Append { value: v.trim().to_string(), store: s.trim().to_string() });
                }
            }
            Stmt::Remove { expr, .. } => {
                if let Some((v, s)) = expr.split_once(" from ") {
                    out.push(IrStmt::Remove { value: v.trim().to_string(), store: s.trim().to_string() });
                }
            }
            Stmt::Update { expr, .. } => {
                if let Some((t, v)) = expr.split_once('=') {
                    out.push(IrStmt::Update { target: t.trim().to_string(), value: v.trim().to_string() });
                }
            }
            Stmt::Save { target, .. } => out.push(IrStmt::Persist { store: target.trim().to_string() }),
            Stmt::Load { target, .. } => out.push(IrStmt::Restore { store: target.trim().to_string() }),
            Stmt::Clear { target, .. } => out.push(IrStmt::Purge { store: target.trim().to_string() }),
            Stmt::Enqueue { expr, .. } => {
                let (item, queue) = match expr.split_once(" to ") {
                    Some((i, q)) => (i.trim().to_string(), q.trim().to_string()),
                    None => (expr.trim().to_string(), "outbox".to_string()),
                };
                out.push(IrStmt::Enqueue { item, queue });
            }
            Stmt::Flush { target, .. } => out.push(IrStmt::Flush { queue: target.trim().to_string() }),
            Stmt::Go { target, .. } => {
                let (screen, args) = parse_navigation_target(target);
                out.push(IrStmt::Navigate { mode: "push".to_string(), screen, args });
            }
            Stmt::Open { target, .. } => {
                let (screen, args) = parse_navigation_target(target);
                out.push(IrStmt::Navigate { mode: "modal".to_string(), screen, args });
            }
            Stmt::Close { .. } => out.push(IrStmt::Navigate { mode: "close".to_string(), screen: None, args: Vec::new() }),
            Stmt::Show { target, .. } => out.push(IrStmt::Present {
                target: target.trim().to_string(),
                mode: "replace".to_string(),
            }),
            Stmt::Return { expr, .. } => out.push(IrStmt::Return {
                value: if expr.trim().is_empty() { None } else { Some(expr.trim().to_string()) },
            }),
            Stmt::Announce { text, urgent, .. } => out.push(IrStmt::Announce {
                text: text.trim_matches('"').to_string(),
                urgent: *urgent,
            }),
            Stmt::PermissionRequest { kind, .. } => {
                out.push(IrStmt::RequestPermission { kind: kind.clone() })
            }
            Stmt::Vibrate { pattern, .. } => out.push(IrStmt::Vibrate { pattern: pattern.trim().to_string() }),
            Stmt::Derive { body: inner, .. } => {
                let expression = inner
                    .iter()
                    .map(|s| match s {
                        Stmt::Expr { text, .. } => text.clone(),
                        Stmt::Call { name, args, .. } => format!("{}({})", name, args),
                        _ => String::new(),
                    })
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<_>>()
                    .join(" ");
                *counter += 1;
                out.push(IrStmt::Derive { target: format!("derived#{}", counter), expression });
            }
            Stmt::Block { header, body: inner, .. } => {
                if let Some(cond) = header.strip_prefix("if ") {
                    let then_branch = lower_statements(inner, counter);
                    let mut otherwise = Vec::new();
                    if let Some(Stmt::Block { header: else_header, body: else_body, .. }) = body.get(index + 1) {
                        if else_header == "else" {
                            otherwise = lower_statements(else_body, counter);
                            index += 1;
                        }
                    }
                    out.push(IrStmt::Branch { condition: cond.trim().to_string(), then: then_branch, otherwise });
                } else if let Some(cond) = header.strip_prefix("else if ") {
                    let then_branch = lower_statements(inner, counter);
                    let otherwise = Vec::new();
                    out.push(IrStmt::Branch { condition: cond.trim().to_string(), then: then_branch, otherwise });
                } else if let Some(head) = header.strip_prefix("for ") {
                    let (item, over) = match head.split_once(" in ") {
                        Some((i, o)) => (i.trim().to_string(), o.trim().to_string()),
                        None => ("item".to_string(), head.trim().to_string()),
                    };
                    out.push(IrStmt::Loop { item, over, body: lower_statements(inner, counter) });
                }
            }
            Stmt::Try { body: attempt, handler, finally, .. } => {
                out.push(IrStmt::Guard {
                    attempt: lower_statements(attempt, counter),
                    rescue: lower_statements(handler, counter),
                    cleanup: lower_statements(finally, counter),
                });
            }
            Stmt::Expr { text, .. } => {
                *counter += 1;
                out.push(IrStmt::Call {
                    target: text.trim().to_string(),
                    args: Vec::new(),
                    node_id: format!("expr#{}", counter),
                });
            }
            _ => {}
        }
        index += 1;
    }
    out
}

/// Dzieli cel nawigacji na nazwe ekranu i argumenty.
fn parse_navigation_target(target: &str) -> (Option<String>, Vec<String>) {
    let t = target.trim();
    if t == "back" || t == "modal" {
        return (None, Vec::new());
    }
    match t.split_once('(') {
        Some((name, rest)) => {
            let args = rest.trim_end_matches(')');
            (Some(name.trim().to_string()), split_args(args))
        }
        None => (Some(t.to_string()), Vec::new()),
    }
}

/// Zbiera nazwy magazynow uzywanych na ekranie.
fn collect_store_access(body: &[Stmt]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for stmt in body {
        match stmt {
            Stmt::Save { target, .. }
            | Stmt::Load { target, .. }
            | Stmt::Clear { target, .. }
            | Stmt::Flush { target, .. } => {
                let name = target.split_whitespace().next().unwrap_or("").to_string();
                if !name.is_empty() && !out.contains(&name) {
                    out.push(name);
                }
            }
            Stmt::Add { expr, .. } | Stmt::Remove { expr, .. } => {
                let name = expr
                    .split_once(" to ")
                    .map(|(_, t)| t)
                    .or_else(|| expr.split_once(" from ").map(|(_, t)| t))
                    .unwrap_or("")
                    .trim()
                    .to_string();
                if !name.is_empty() && !out.contains(&name) {
                    out.push(name);
                }
            }
            Stmt::Container { body: inner, .. }
            | Stmt::Block { body: inner, .. }
            | Stmt::When { body: inner, .. }
            | Stmt::Derive { body: inner, .. } => {
                for n in collect_store_access(inner) {
                    if !out.contains(&n) {
                        out.push(n);
                    }
                }
            }
            Stmt::Try { body: inner, handler, finally, .. } => {
                for group in [inner, handler, finally] {
                    for n in collect_store_access(group) {
                        if !out.contains(&n) {
                            out.push(n);
                        }
                    }
                }
            }
            _ => {}
        }
    }
    out
}

/// Zbiera cele nawigacji z ekranu.
fn collect_navigation(body: &[Stmt]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for stmt in body {
        match stmt {
            Stmt::Go { target, .. } | Stmt::Open { target, .. } => {
                let name = target.split('(').next().unwrap_or("").trim().to_string();
                if !name.is_empty() && name != "back" && name != "modal" && !out.contains(&name) {
                    out.push(name);
                }
            }
            Stmt::Container { body: inner, .. }
            | Stmt::Block { body: inner, .. }
            | Stmt::When { body: inner, .. }
            | Stmt::Derive { body: inner, .. } => {
                for n in collect_navigation(inner) {
                    if !out.contains(&n) {
                        out.push(n);
                    }
                }
            }
            Stmt::Try { body: inner, handler, finally, .. } => {
                for group in [inner, handler, finally] {
                    for n in collect_navigation(group) {
                        if !out.contains(&n) {
                            out.push(n);
                        }
                    }
                }
            }
            _ => {}
        }
    }
    out
}

/// Zbiera wywolania funkcji z ciala.
fn collect_calls(body: &[Stmt]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for stmt in body {
        match stmt {
            Stmt::Call { name, .. } => {
                if !out.contains(name) {
                    out.push(name.clone());
                }
            }
            Stmt::Container { body: inner, .. }
            | Stmt::Block { body: inner, .. }
            | Stmt::When { body: inner, .. }
            | Stmt::Derive { body: inner, .. } => out.extend(collect_calls(inner)),
            Stmt::Try { body: inner, handler, finally, .. } => {
                out.extend(collect_calls(inner));
                out.extend(collect_calls(handler));
                out.extend(collect_calls(finally));
            }
            _ => {}
        }
    }
    out.sort();
    out.dedup();
    out
}

/// Zbiera efekty uboczne funkcji: magazyny, siec, AI, uprawnienia.
fn collect_effects(body: &[Stmt]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for stmt in body {
        match stmt {
            Stmt::Save { .. }
            | Stmt::Load { .. }
            | Stmt::Clear { .. }
            | Stmt::Add { .. }
            | Stmt::Remove { .. }
            | Stmt::Update { .. }
            | Stmt::Enqueue { .. }
            | Stmt::Flush { .. } => push_unique(&mut out, "store"),
            Stmt::Call { name, .. } => {
                if name.starts_with("AI.") {
                    push_unique(&mut out, "ai");
                } else if name.contains('.') {
                    push_unique(&mut out, "network");
                }
            }
            Stmt::PermissionRequest { .. } => push_unique(&mut out, "permission"),
            Stmt::Vibrate { .. } => push_unique(&mut out, "haptics"),
            Stmt::Announce { .. } => push_unique(&mut out, "announcement"),
            Stmt::Container { body: inner, .. }
            | Stmt::Block { body: inner, .. }
            | Stmt::When { body: inner, .. }
            | Stmt::Derive { body: inner, .. } => {
                for e in collect_effects(inner) {
                    push_unique(&mut out, &e);
                }
            }
            Stmt::Try { body: inner, handler, finally, .. } => {
                for group in [inner, handler, finally] {
                    for e in collect_effects(group) {
                        push_unique(&mut out, &e);
                    }
                }
            }
            _ => {}
        }
    }
    out
}

fn push_unique(list: &mut Vec<String>, value: &str) {
    if !list.iter().any(|v| v == value) {
        list.push(value.to_string());
    }
}

/// Dzieli liste argumentow zapisana jako tekst.
fn split_args(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut in_string = false;
    let mut current = String::new();
    for ch in s.chars() {
        match ch {
            '"' => {
                in_string = !in_string;
                current.push(ch);
            }
            '(' | '[' | '{' if !in_string => {
                depth += 1;
                current.push(ch);
            }
            ')' | ']' | '}' if !in_string => {
                depth -= 1;
                current.push(ch);
            }
            ',' if !in_string && depth == 0 => {
                let t = current.trim().to_string();
                if !t.is_empty() {
                    out.push(t);
                }
                current.clear();
            }
            _ => current.push(ch),
        }
    }
    let t = current.trim().to_string();
    if !t.is_empty() {
        out.push(t);
    }
    out
}

/// Nazwa typu w postaci tekstowej, niezalezna od platformy.
pub fn type_name(ty: &TypeRef) -> String {
    match ty {
        TypeRef::Text => "Text".to_string(),
        TypeRef::Number => "Number".to_string(),
        TypeRef::Boolean => "Boolean".to_string(),
        TypeRef::Date => "Date".to_string(),
        TypeRef::Duration => "Duration".to_string(),
        TypeRef::Json => "Json".to_string(),
        TypeRef::File => "File".to_string(),
        TypeRef::Image => "Image".to_string(),
        TypeRef::Video => "Video".to_string(),
        TypeRef::Audio => "Audio".to_string(),
        TypeRef::Location => "Location".to_string(),
        TypeRef::List(inner) => format!("List<{}>", type_name(inner)),
        TypeRef::Map(k, v) => format!("Map<{}, {}>", type_name(k), type_name(v)),
        TypeRef::Optional(inner) => format!("{}?", type_name(inner)),
        TypeRef::Named(name) => name.clone(),
    }
}

/// Nazwa klasy magazynu w postaci tekstowej.
fn store_class_name(class: StoreClass) -> &'static str {
    match class {
        StoreClass::Local => "local",
        StoreClass::Synced => "synced",
        StoreClass::Secure => "secure",
        StoreClass::Cache => "cache",
        StoreClass::Queue => "queue",
    }
}

/// Nazwa wpisu w manifescie platformy dla danego uprawnienia.
pub fn permission_declaration(kind: &str) -> &'static str {
    match kind {
        "camera" => "NSCameraUsageDescription",
        "microphone" => "NSMicrophoneUsageDescription",
        "location" => "NSLocationWhenInUseUsageDescription",
        "photos" => "NSPhotoLibraryUsageDescription",
        "notifications" => "POST_NOTIFICATIONS",
        "contacts" => "READ_CONTACTS",
        "bluetooth" => "BLUETOOTH_CONNECT",
        _ => "BIND_PERMISSION",
    }
}
