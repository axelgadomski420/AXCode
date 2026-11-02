// src/hir.rs — reprezentacja wysokopoziomowa (HIR)
//
// HIR to drzewo po rozwiazaniu nazw: kazde odwolanie do ekranu, funkcji,
// magazynu i typu jest zamienione na rozwiazany symbol. Ten etap nie zmienia
// semantyki programu — przygotowuje dane dla IR i dla diagnostyki nazw.

use crate::ast::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Program w reprezentacji wysokopoziomowej.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HirProgram {
    pub app_name: String,
    pub screens: Vec<HirScreen>,
    pub components: Vec<HirComponent>,
    pub functions: Vec<HirFunction>,
    pub stores: Vec<HirStore>,
    pub type_table: HashMap<String, String>,
    pub enum_table: HashMap<String, Vec<String>>,
    pub symbol_count: usize,
}

/// Ekran z rozwiazanymi odwolaniami.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HirScreen {
    pub id: String,
    pub name: String,
    pub title: String,
    pub state: Vec<HirState>,
    pub nodes: Vec<HirNode>,
    pub resolved_refs: Vec<String>,
    pub navigation: Vec<String>,
    pub store_access: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HirState {
    pub name: String,
    pub type_name: String,
    pub initial: Option<String>,
    pub derived: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HirNode {
    pub kind: String,
    pub label: Option<String>,
    pub source: Option<String>,
    pub has_accessibility_label: bool,
    pub has_alt: bool,
    pub children: Vec<HirNode>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HirComponent {
    pub id: String,
    pub name: String,
    pub signature: String,
    pub nodes: Vec<HirNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HirFunction {
    pub id: String,
    pub name: String,
    pub signature: String,
    pub calls: Vec<String>,
    pub effects: Vec<String>,
    pub statement_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HirStore {
    pub name: String,
    pub class: String,
    pub encrypted: bool,
    pub element_type: String,
    pub key: String,
}

/// Zamienia drzewo skladniowe na HIR.
pub fn lower(program: &Program) -> HirProgram {
    let mut type_table: HashMap<String, String> = HashMap::new();
    for data in &program.data {
        let fields: Vec<String> = data
            .fields
            .iter()
            .map(|f| format!("{}: {}", f.name, f.ty.display()))
            .collect();
        type_table.insert(data.name.clone(), fields.join(", "));
    }

    let mut enum_table: HashMap<String, Vec<String>> = HashMap::new();
    for decl in &program.enums {
        enum_table.insert(decl.name.clone(), decl.variants.clone());
    }

    let screens = program
        .screens
        .iter()
        .map(|screen| {
            let mut refs: Vec<String> = Vec::new();
            collect_ui_refs(&screen.body, &mut refs);
            HirScreen {
                id: format!("screen.{}", screen.name),
                name: screen.name.clone(),
                title: screen.title.clone().unwrap_or_else(|| screen.name.clone()),
                state: screen
                    .state
                    .iter()
                    .map(|st| HirState {
                        name: st.name.clone(),
                        type_name: st.ty.display(),
                        initial: st.initial.clone(),
                        derived: st.derived.is_some(),
                    })
                    .collect(),
                nodes: screen.body.iter().map(lower_ui_node).collect(),
                resolved_refs: refs,
                navigation: collect_navigation(&screen.body),
                store_access: collect_store_access(&screen.body),
            }
        })
        .collect();

    let components = program
        .components
        .iter()
        .map(|comp| HirComponent {
            id: format!("component.{}", comp.name),
            name: comp.name.clone(),
            signature: format!(
                "{}({})",
                comp.name,
                comp.params
                    .iter()
                    .map(|p| format!("{} {}", p.name, p.ty.display()))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            nodes: comp.body.iter().map(lower_ui_node).collect(),
        })
        .collect();

    let functions = program
        .functions
        .iter()
        .map(|function| HirFunction {
            id: format!("fn.{}", function.name),
            name: function.name.clone(),
            signature: format!(
                "{}({}) -> {}",
                function.name,
                function
                    .params
                    .iter()
                    .map(|p| format!("{} {}", p.name, p.ty.display()))
                    .collect::<Vec<_>>()
                    .join(", "),
                function.returns.display()
            ),
            calls: collect_calls(&function.body),
            effects: collect_effects(&function.body),
            statement_count: function.body.len(),
        })
        .collect();

    let stores = program
        .stores
        .iter()
        .map(|store| {
            let element_type = match &store.content {
                StoreContent::Single(ty) => ty.display(),
                StoreContent::List(ty) => ty.display(),
            };
            HirStore {
                name: store.name.clone(),
                class: store
                    .classes
                    .iter()
                    .map(|c| c.name().to_string())
                    .collect::<Vec<_>>()
                    .join(" "),
                encrypted: store.classes.iter().any(|c| c.is_protected()),
                element_type,
                key: store.key.clone().unwrap_or_default(),
            }
        })
        .collect();

    HirProgram {
        app_name: program.app.name.clone(),
        screens,
        components,
        functions,
        stores,
        symbol_count: type_table.len() + enum_table.len(),
        type_table,
        enum_table,
    }
}

/// Analiza HIR: dopisuje odwolania do magazynow widoczne w ekranach.
pub fn analyze(program: &mut HirProgram) {
    let known_stores: Vec<String> = program.stores.iter().map(|s| s.name.clone()).collect();
    for screen in &mut program.screens {
        for store in &known_stores {
            if screen.resolved_refs.iter().any(|r| r.contains(store))
                && !screen.store_access.contains(store)
            {
                screen.store_access.push(store.clone());
            }
        }
    }
}

fn lower_ui_node(node: &UiNode) -> HirNode {
    match node {
        UiNode::Container { kind, source, children, accessories, .. } => HirNode {
            kind: format!("{:?}", kind).to_lowercase(),
            label: source.clone(),
            source: source.clone(),
            has_accessibility_label: accessories
                .iter()
                .any(|a| matches!(a, Accessory::AccessibilityLabel(_))),
            has_alt: false,
            children: children.iter().map(lower_ui_node).collect(),
        },
        UiNode::Text { accessories, .. } => HirNode {
            kind: "text".to_string(),
            label: None,
            source: None,
            has_accessibility_label: accessories
                .iter()
                .any(|a| matches!(a, Accessory::AccessibilityLabel(_))),
            has_alt: false,
            children: Vec::new(),
        },
        UiNode::Button { label, accessories, .. } => HirNode {
            kind: "button".to_string(),
            label: label.clone(),
            source: None,
            has_accessibility_label: accessories
                .iter()
                .any(|a| matches!(a, Accessory::AccessibilityLabel(_))),
            has_alt: false,
            children: Vec::new(),
        },
        UiNode::Field { name, label, .. } => HirNode {
            kind: "field".to_string(),
            label: label.clone().or_else(|| Some(name.clone())),
            source: Some(name.clone()),
            has_accessibility_label: label.is_some(),
            has_alt: false,
            children: Vec::new(),
        },
        UiNode::Image { alt, accessories, .. } => HirNode {
            kind: "image".to_string(),
            label: alt.clone(),
            source: None,
            has_accessibility_label: accessories
                .iter()
                .any(|a| matches!(a, Accessory::AccessibilityLabel(_))),
            has_alt: alt.is_some(),
            children: Vec::new(),
        },
        UiNode::Checkbox { reference, accessibility_label, .. } => HirNode {
            kind: "checkbox".to_string(),
            label: accessibility_label.clone(),
            source: Some(reference.clone()),
            has_accessibility_label: accessibility_label.is_some(),
            has_alt: false,
            children: Vec::new(),
        },
        UiNode::Media { kind, accessibility_label, .. } => HirNode {
            kind: format!("{:?}", kind).to_lowercase(),
            label: accessibility_label.clone(),
            source: None,
            has_accessibility_label: accessibility_label.is_some(),
            has_alt: false,
            children: Vec::new(),
        },
        UiNode::ComponentCall { name, .. } => HirNode {
            kind: "call".to_string(),
            label: Some(name.clone()),
            source: Some(name.clone()),
            has_accessibility_label: false,
            has_alt: false,
            children: Vec::new(),
        },
        UiNode::Conditional { then_branch, else_branch, .. } => HirNode {
            kind: "conditional".to_string(),
            label: None,
            source: None,
            has_accessibility_label: false,
            has_alt: false,
            children: then_branch
                .iter()
                .map(lower_ui_node)
                .chain(else_branch.iter().map(lower_ui_node))
                .collect(),
        },
        UiNode::Loop { variable, body, .. } => HirNode {
            kind: "loop".to_string(),
            label: Some(variable.clone()),
            source: Some(variable.clone()),
            has_accessibility_label: false,
            has_alt: false,
            children: body.iter().map(lower_ui_node).collect(),
        },
        UiNode::Spacer { .. } => HirNode {
            kind: "spacer".to_string(),
            label: None,
            source: None,
            has_accessibility_label: false,
            has_alt: false,
            children: Vec::new(),
        },
    }
}

/// Zbiera odwolania do symboli widoczne w drzewie interfejsu.
fn collect_ui_refs(nodes: &[UiNode], out: &mut Vec<String>) {
    for node in nodes {
        match node {
            UiNode::Container { children, source, .. } => {
                if let Some(name) = source {
                    push_unique(out, name);
                }
                collect_ui_refs(children, out);
            }
            UiNode::ComponentCall { name, .. } => push_unique(out, name),
            UiNode::Conditional { then_branch, else_branch, .. } => {
                collect_ui_refs(then_branch, out);
                collect_ui_refs(else_branch, out);
            }
            UiNode::Loop { body, .. } => collect_ui_refs(body, out),
            UiNode::Checkbox { reference, .. } => {
                let root = reference.split('.').next().unwrap_or("").to_string();
                push_unique(out, &root);
            }
            _ => {}
        }
    }
}

fn collect_navigation(nodes: &[UiNode]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for node in nodes {
        match node {
            UiNode::Container { children, .. }
            | UiNode::Conditional { then_branch: children, .. } => {
                for target in collect_navigation(children) {
                    push_unique(&mut out, &target);
                }
            }
            UiNode::Loop { body, .. } => {
                for target in collect_navigation(body) {
                    push_unique(&mut out, &target);
                }
            }
            _ => {}
        }
    }
    out
}

fn collect_store_access(nodes: &[UiNode]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for node in nodes {
        if let UiNode::Container { children, .. } = node {
            for name in collect_store_access(children) {
                push_unique(&mut out, &name);
            }
        }
    }
    out
}

fn collect_calls(body: &[Statement]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for statement in body {
        if let Statement::Expression { value, .. } = statement {
            let text = expression_text(value);
            if let Some(open) = text.find('(') {
                push_unique(&mut out, text[..open].trim());
            }
        }
        for nested in nested_statements(statement) {
            for call in collect_calls(nested) {
                push_unique(&mut out, &call);
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

fn collect_effects(body: &[Statement]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for statement in body {
        match statement {
            Statement::Save { .. }
            | Statement::Load { .. }
            | Statement::Clear { .. }
            | Statement::Add { .. }
            | Statement::Remove { .. }
            | Statement::Update { .. }
            | Statement::Enqueue { .. }
            | Statement::Flush { .. } => push_unique(&mut out, "store"),
            Statement::Confirm { .. } => push_unique(&mut out, "dialog"),
            Statement::Raise { .. } => push_unique(&mut out, "raise"),
            Statement::Expression { value, .. } => {
                let text = expression_text(value);
                if text.starts_with("AI.") {
                    push_unique(&mut out, "ai");
                } else if text.contains('.') {
                    push_unique(&mut out, "network");
                } else if text.starts_with("announce") {
                    push_unique(&mut out, "announcement");
                } else if text.starts_with("vibrate") {
                    push_unique(&mut out, "haptics");
                }
            }
            _ => {}
        }
        for nested in nested_statements(statement) {
            for effect in collect_effects(nested) {
                push_unique(&mut out, &effect);
            }
        }
    }
    out
}

fn nested_statements(statement: &Statement) -> &[Statement] {
    match statement {
        Statement::If { then_branch, else_branch, .. } => then_branch,
        Statement::ForEach { body, .. } => body,
        Statement::Try { body, .. } => body,
        _ => &[],
    }
}

fn push_unique(list: &mut Vec<String>, value: &str) {
    if !value.is_empty() && !list.iter().any(|v| v == value) {
        list.push(value.to_string());
    }
}

fn expression_text(expression: &Expression) -> String {
    format!("{:?}", expression)
}
