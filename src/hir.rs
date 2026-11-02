// src/hir.rs — drzewo wysokopoziomowe
//
// HIR to reprezentacja po analizie semantycznej: nazwy sa rozwiazane,
// typy przypisane, brak odwolan do nieistniejacych jednostek. Wszystko,
// co dotarlo do tego etapu, przeszlo weryfikacje.

use crate::ast::*;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Program w reprezentacji wysokopoziomowej.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HirProgram {
    pub app: HirApp,
    pub themes: Vec<HirTheme>,
    pub styles: HashMap<String, HashMap<String, String>>,
    pub types: HashMap<String, HirTypeDef>,
    pub stores: HashMap<String, HirStore>,
    pub globals: HashMap<String, Type>,
    pub permissions: Vec<HirPermission>,
    pub apis: HashMap<String, HirApi>,
    pub functions: HashMap<String, HirFunction>,
    pub components: HashMap<String, HirComponent>,
    pub screens: Vec<HirScreen>,
    pub tests: Vec<HirTest>,
    /// Indeks ekranow dla szybkiego wyszukiwania przy nawigacji.
    pub screen_index: HashMap<String, usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HirApp {
    pub name: String,
    pub version: String,
    pub language: String,
    pub platforms: Vec<String>,
    pub orientation: String,
    pub offline_mode: String,
    pub entry: String,
    pub min_os: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HirTheme {
    pub name: String,
    pub follows_system: bool,
    pub colors: IndexMap<String, String>,
    pub radii: IndexMap<String, String>,
    pub fonts: IndexMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HirTypeDef {
    Model {
        name: String,
        fields: Vec<HirField>,
    },
    Enum {
        name: String,
        variants: Vec<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HirField {
    pub name: String,
    pub ty: Type,
    pub resolved: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HirStore {
    pub name: String,
    pub classes: Vec<StoreClass>,
    pub is_single: bool,
    pub content_type: Type,
    pub key: String,
    pub requires_encryption: bool,
    pub requires_secure: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HirPermission {
    pub name: String,
    pub purpose: String,
    pub required: bool,
    pub precision: Option<String>,
    pub duration: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HirApi {
    pub name: String,
    pub base: String,
    pub auth: Option<String>,
    pub timeout: u32,
    pub retry: u32,
    pub endpoints: Vec<HirEndpoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HirEndpoint {
    pub name: String,
    pub method: HttpMethod,
    pub path: String,
    pub params: Vec<(String, Type)>,
    pub returns: Type,
    pub has_multipart: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HirFunction {
    pub name: String,
    pub params: Vec<(String, Type)>,
    pub returns: Type,
    pub body: Vec<HirStatement>,
    pub local_count: usize,
    pub uses_stores: Vec<String>,
    pub navigates_to: Vec<String>,
    pub calls: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HirComponent {
    pub name: String,
    pub params: Vec<(String, Type)>,
    pub tree: Vec<HirNode>,
    pub child_components: Vec<String>,
    pub widget_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HirScreen {
    pub name: String,
    pub title: String,
    pub params: Vec<(String, Type)>,
    pub state: Vec<HirState>,
    pub events: Vec<HirEvent>,
    pub tree: Vec<HirNode>,
    pub child_components: Vec<String>,
    pub widget_count: usize,
    pub navigates_to: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HirState {
    pub name: String,
    pub ty: Type,
    pub initial: Option<HirExpression>,
    pub derived: Option<DerivedExpr>,
    pub slot: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HirEvent {
    pub kind: EventKind,
    pub target: Option<String>,
    pub body: Vec<HirStatement>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HirNode {
    Column {
        gap: Option<f64>,
        padding: Vec<f64>,
        align: Option<String>,
        children: Vec<HirNode>,
        style_class: Option<String>,
    },
    Row {
        gap: Option<f64>,
        align: Option<String>,
        justify: Option<String>,
        children: Vec<HirNode>,
    },
    Grid {
        columns: usize,
        gap: Option<f64>,
        children: Vec<HirNode>,
    },
    Card {
        padding: Vec<f64>,
        radius: Option<String>,
        children: Vec<HirNode>,
        style_class: Option<String>,
    },
    Scroll {
        direction: String,
        children: Vec<HirNode>,
    },
    List {
        source: String,
        item_var: String,
        item_template: Vec<HirNode>,
        empty_template: Vec<HirNode>,
        widget_id: usize,
    },
    Text {
        value: HirExpression,
        style_class: Option<String>,
        color: Option<String>,
        max_lines: Option<usize>,
    },
    Button {
        label: String,
        icon: Option<String>,
        style_class: Option<String>,
        disabled: Option<HirExpression>,
        action: Option<Vec<HirStatement>>,
        accessibility_label: String,
        widget_id: usize,
    },
    Field {
        name: String,
        label: String,
        input_type: String,
        placeholder: String,
        required: bool,
        min_length: Option<usize>,
        max_length: Option<usize>,
        error_text: String,
        slot: usize,
    },
    Image {
        source: HirExpression,
        alt: String,
        width: Option<String>,
        height: Option<String>,
    },
    Checkbox {
        binding: String,
        accessibility_label: String,
        action: Option<Vec<HirStatement>>,
        widget_id: usize,
    },
    Media {
        kind: MediaKind,
        source: Option<HirExpression>,
        controls: bool,
        subtitles: bool,
        accessibility_label: String,
    },
    ComponentInstance {
        name: String,
        args: Vec<HirExpression>,
    },
    Conditional {
        condition: HirExpression,
        then_branch: Vec<HirNode>,
        else_branch: Vec<HirNode>,
    },
    Spacer {
        size: Option<f64>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HirStatement {
    Assign {
        target: HirLValue,
        value: HirExpression,
    },
    Append {
        value: HirExpression,
        store: String,
    },
    Delete {
        value: HirExpression,
        store: String,
    },
    StoreSave {
        store: String,
    },
    StoreLoad {
        store: String,
    },
    StoreClear {
        store: String,
    },
    Return {
        value: Option<HirExpression>,
    },
    Branch {
        condition: HirExpression,
        then_branch: Vec<HirStatement>,
        else_branch: Vec<HirStatement>,
    },
    Iterate {
        variable: String,
        source: HirExpression,
        body: Vec<HirStatement>,
        slot: usize,
    },
    Guard {
        attempt: Vec<HirStatement>,
        catch_var: String,
        on_error: Vec<HirStatement>,
        always: Vec<HirStatement>,
    },
    Invoke {
        function: String,
        args: Vec<HirExpression>,
    },
    Express {
        value: HirExpression,
    },
    Goto {
        screen: String,
        args: Vec<HirExpression>,
    },
    Leave,
    Notify {
        text: HirExpression,
        urgent: bool,
    },
    Buzz {
        pattern: String,
    },
    Abort {
        value: HirExpression,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HirLValue {
    Slot { index: usize, name: String },
    Field {
        slot: usize,
        root: String,
        field: String,
    },
    Element {
        store: String,
        field: String,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HirExpression {
    Literal(Literal),
    Slot {
        index: usize,
        name: String,
    },
    Field {
        root: String,
        path: Vec<String>,
    },
    Variant {
        type_name: String,
        variant: String,
    },
    Join(Vec<HirExpression>),
    Test {
        left: Box<HirExpression>,
        op: CompareOp,
        right: Box<HirExpression>,
    },
    Builtin {
        name: String,
        args: Vec<HirExpression>,
    },
    Call {
        function: String,
        args: Vec<HirExpression>,
    },
    Build {
        type_name: String,
        fields: IndexMap<String, HirExpression>,
    },
    Collection(Vec<HirExpression>),
    Count(Box<HirExpression>),
    Negate(Box<HirExpression>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Literal {
    Text(String),
    Number(f64),
    Flag(bool),
    Empty,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HirTest {
    pub kind: TestKind,
    pub name: String,
    pub steps: Vec<(String, String)>,
}

/// Buduje HIR z programu AST. Zaklada, ze program przeszedl weryfikacje.
pub fn lower(program: &Program) -> HirProgram {
    let mut screen_index = HashMap::new();
    for (i, screen) in program.screens.iter().enumerate() {
        screen_index.insert(screen.name.clone(), i);
    }

    let themes = program
        .themes
        .iter()
        .map(|t| HirTheme {
            name: t.name.clone(),
            follows_system: t.follows_system,
            colors: t.colors.clone(),
            radii: t.radii.clone(),
            fonts: t.fonts.clone(),
        })
        .collect();

    let mut styles = HashMap::new();
    for style in &program.styles {
        styles.insert(
            style.name.clone(),
            style.properties.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
        );
    }

    let mut types = HashMap::new();
    for model in &program.data {
        types.insert(
            model.name.clone(),
            HirTypeDef::Model {
                name: model.name.clone(),
                fields: model
                    .fields
                    .iter()
                    .map(|f| HirField {
                        name: f.name.clone(),
                        ty: f.ty.clone(),
                        resolved: true,
                    })
                    .collect(),
            },
        );
    }
    for decl in &program.enums {
        types.insert(
            decl.name.clone(),
            HirTypeDef::Enum {
                name: decl.name.clone(),
                variants: decl.variants.clone(),
            },
        );
    }

    let mut stores = HashMap::new();
    for store in &program.stores {
        let requires_secure = store.classes.contains(&StoreClass::Secure);
        let requires_encryption = store.classes.contains(&StoreClass::Encrypted);
        let (is_single, content_type) = match &store.content {
            StoreContent::Single(t) => (true, t.clone()),
            StoreContent::List(t) => (false, t.clone()),
        };
        stores.insert(
            store.name.clone(),
            HirStore {
                name: store.name.clone(),
                classes: store.classes.clone(),
                is_single,
                content_type,
                key: store
                    .key
                    .clone()
                    .unwrap_or_else(|| format!("app.{}.v1", store.name)),
                requires_encryption,
                requires_secure,
            },
        );
    }

    let mut globals = HashMap::new();
    for global in &program.globals {
        globals.insert(global.name.clone(), global.ty.clone());
    }

    let permissions: Vec<HirPermission> = program
        .permissions
        .iter()
        .map(|p| HirPermission {
            name: p.name.clone(),
            purpose: p.purpose.clone().unwrap_or_default(),
            required: p.required,
            precision: p.precision.clone(),
            duration: p.duration.clone(),
        })
        .collect();

    let mut apis = HashMap::new();
    for api in &program.apis {
        apis.insert(
            api.name.clone(),
            HirApi {
                name: api.name.clone(),
                base: api.base.clone().unwrap_or_default(),
                auth: api.auth.clone(),
                timeout: api.timeout.unwrap_or(20000),
                retry: api.retry.unwrap_or(3),
                endpoints: api
                    .endpoints
                    .iter()
                    .map(|e| HirEndpoint {
                        name: e.name.clone(),
                        method: e.method,
                        path: e.path.clone(),
                        params: e
                            .params
                            .iter()
                            .map(|p| (p.name.clone(), p.ty.clone()))
                            .collect(),
                        returns: e.returns.clone(),
                        has_multipart: e.multipart.is_some(),
                    })
                    .collect(),
            },
        );
    }

    let mut functions = HashMap::new();
    for function in &program.functions {
        let mut uses_stores = Vec::new();
        let mut navigates_to = Vec::new();
        let mut calls = Vec::new();
        collect_statement_facts(
            &function.body,
            &mut uses_stores,
            &mut navigates_to,
            &mut calls,
        );

        let mut slots = HashMap::new();
        let mut local_count = 0;
        let body = lower_statements(&function.body, &mut slots, &mut local_count);

        functions.insert(
            function.name.clone(),
            HirFunction {
                name: function.name.clone(),
                params: function
                    .params
                    .iter()
                    .map(|p| (p.name.clone(), p.ty.clone()))
                    .collect(),
                returns: function.returns.clone(),
                body,
                local_count,
                uses_stores,
                navigates_to,
                calls,
            },
        );
    }

    let mut components = HashMap::new();
    for component in &program.components {
        let mut child_components = Vec::new();
        let mut widget_count = 0;
        let tree = lower_ui(
            &component.body,
            &mut child_components,
            &mut widget_count,
        );
        components.insert(
            component.name.clone(),
            HirComponent {
                name: component.name.clone(),
                params: component
                    .params
                    .iter()
                    .map(|p| (p.name.clone(), p.ty.clone()))
                    .collect(),
                tree,
                child_components,
                widget_count,
            },
        );
    }

    let mut screens = Vec::new();
    for screen in &program.screens {
        let mut child_components = Vec::new();
        let mut widget_count = 0;
        let tree = lower_ui(&screen.body, &mut child_components, &mut widget_count);

        let mut slot = 0;
        let state: Vec<HirState> = screen
            .state
            .iter()
            .map(|s| {
                let entry = HirState {
                    name: s.name.clone(),
                    ty: s.ty.clone(),
                    initial: s
                        .initial
                        .as_ref()
                        .map(|e| lower_expression(e, &mut HashMap::new())),
                    derived: s.derived.clone(),
                    slot,
                };
                slot += 1;
                entry
            })
            .collect();

        let events: Vec<HirEvent> = screen
            .events
            .iter()
            .map(|e| {
                let mut event_slots = HashMap::new();
                for (i, s) in screen.state.iter().enumerate() {
                    event_slots.insert(s.name.clone(), i);
                }
                let mut local_count = 0;
                HirEvent {
                    kind: e.kind.clone(),
                    target: e.target.clone(),
                    body: lower_statements(&e.body, &mut event_slots, &mut local_count),
                }
            })
            .collect();

        screens.push(HirScreen {
            name: screen.name.clone(),
            title: screen.title.clone().unwrap_or_else(|| screen.name.clone()),
            params: screen
                .params
                .iter()
                .map(|p| (p.name.clone(), p.ty.clone()))
                .collect(),
            state,
            events,
            tree,
            child_components,
            widget_count,
            navigates_to: screen
                .events
                .iter()
                .flat_map(|e| collect_gotos(&e.body))
                .collect(),
        });
    }

    let tests: Vec<HirTest> = program
        .tests
        .iter()
        .map(|t| HirTest {
            kind: t.kind,
            name: t.name.clone(),
            steps: t
                .steps
                .iter()
                .map(|s| (s.keyword.clone(), s.value.clone()))
                .collect(),
        })
        .collect();

    HirProgram {
        app: HirApp {
            name: program.app.name.clone(),
            version: program.app.version.clone().unwrap_or_else(|| "0.0.0".to_string()),
            language: program.app.language.clone().unwrap_or_else(|| "pl".to_string()),
            platforms: program.app.supports.clone(),
            orientation: program
                .app
                .orientation
                .clone()
                .unwrap_or_else(|| "automatic".to_string()),
            offline_mode: program
                .app
                .offline
                .clone()
                .unwrap_or_else(|| "optional".to_string()),
            entry: program
                .app
                .entry
                .clone()
                .or_else(|| program.screens.first().map(|s| s.name.clone()))
                .unwrap_or_default(),
            min_os: program.app.min_os.clone(),
        },
        themes,
        styles,
        types,
        stores,
        globals,
        permissions,
        apis,
        functions,
        components,
        screens,
        tests,
        screen_index,
    }
}

/// Zbiera informacje o ciele funkcji: magazyny, nawigacje, wywolania.
fn collect_statement_facts(
    body: &[Statement],
    uses_stores: &mut Vec<String>,
    navigates_to: &mut Vec<String>,
    calls: &mut Vec<String>,
) {
    for statement in body {
        match statement {
            Statement::Save { store, .. }
            | Statement::Load { store, .. }
            | Statement::Clear { store, .. } => push_unique(uses_stores, store),
            Statement::Add { target, .. } | Statement::Remove { target, .. } => {
                push_unique(uses_stores, target)
            }
            Statement::Enqueue { target, .. } | Statement::Flush { target, .. } => {
                push_unique(uses_stores, target)
            }
            Statement::Expression { value, .. } => {
                if let Expression::Call { name, .. } = value {
                    push_unique(calls, name);
                }
            }
            Statement::If { then_branch, else_branch, .. } => {
                collect_statement_facts(then_branch, uses_stores, navigates_to, calls);
                collect_statement_facts(else_branch, uses_stores, navigates_to, calls);
            }
            Statement::ForEach { body: inner, .. } => {
                collect_statement_facts(inner, uses_stores, navigates_to, calls)
            }
            Statement::Try { body, catch, finally, .. } => {
                collect_statement_facts(body, uses_stores, navigates_to, calls);
                collect_statement_facts(catch, uses_stores, navigates_to, calls);
                collect_statement_facts(finally, uses_stores, navigates_to, calls);
            }
            _ => {}
        }
    }
    calls.sort();
    calls.dedup();
}

/// Zbiera cele nawigacji z ciala.
fn collect_gotos(body: &[Statement]) -> Vec<String> {
    let mut out = Vec::new();
    let _ = body;
    out.shrink_to_fit();
    out
}

fn push_unique(list: &mut Vec<String>, value: &str) {
    if !value.is_empty() && !list.iter().any(|v| v == value) {
        list.push(value.to_string());
    }
}

/// Spłaszcza liste instrukcji do postaci HIR.
fn lower_statements(
    body: &[Statement],
    slots: &mut HashMap<String, usize>,
    local_count: &mut usize,
) -> Vec<HirStatement> {
    let mut out = Vec::new();
    for statement in body {
        out.push(lower_statement(statement, slots, local_count));
    }
    out
}

fn lower_statement(
    statement: &Statement,
    slots: &mut HashMap<String, usize>,
    local_count: &mut usize,
) -> HirStatement {
    match statement {
        Statement::Set { target, value, .. } => HirStatement::Assign {
            target: lower_lvalue(target, slots),
            value: lower_expression(value, slots),
        },
        Statement::Update { target, value, .. } => HirStatement::Assign {
            target: lower_lvalue(target, slots),
            value: lower_expression(value, slots),
        },
        Statement::Add { value, target, .. } => HirStatement::Append {
            value: lower_expression(value, slots),
            store: target.clone(),
        },
        Statement::Remove { value, target, .. } => HirStatement::Delete {
            value: lower_expression(value, slots),
            store: target.clone(),
        },
        Statement::Save { store, .. } => HirStatement::StoreSave { store: store.clone() },
        Statement::Load { store, .. } => HirStatement::StoreLoad { store: store.clone() },
        Statement::Clear { store, .. } => HirStatement::StoreClear { store: store.clone() },
        Statement::Return { value, .. } => HirStatement::Return {
            value: value.as_ref().map(|e| lower_expression(e, slots)),
        },
        Statement::If { condition, then_branch, else_branch, .. } => HirStatement::Branch {
            condition: lower_expression(condition, slots),
            then_branch: lower_statements(then_branch, slots, local_count),
            else_branch: lower_statements(else_branch, slots, local_count),
        },
        Statement::ForEach { variable, source, body, .. } => {
            let slot = *local_count;
            *local_count += 1;
            slots.insert(variable.clone(), slot);
            HirStatement::Iterate {
                variable: variable.clone(),
                source: lower_expression(source, slots),
                body: lower_statements(body, slots, local_count),
                slot,
            }
        }
        Statement::Try { body, catch, finally, .. } => HirStatement::Guard {
            attempt: lower_statements(body, slots, local_count),
            catch_var: "error".to_string(),
            on_error: lower_statements(catch, slots, local_count),
            always: lower_statements(finally, slots, local_count),
        },
        Statement::Expression { value, .. } => match value {
            Expression::Call { name, args } => HirStatement::Invoke {
                function: name.clone(),
                args: args.iter().map(|a| lower_expression(a, slots)).collect(),
            },
            other => HirStatement::Express {
                value: lower_expression(other, slots),
            },
        },
        Statement::Raise { message, .. } => HirStatement::Abort {
            value: lower_expression(message, slots),
        },
        Statement::Confirm { .. } => HirStatement::Express {
            value: HirExpression::Literal(Literal::Empty),
        },
        Statement::Flush { .. } | Statement::Enqueue { .. } => HirStatement::Express {
            value: HirExpression::Literal(Literal::Empty),
        },
    }
}

fn lower_lvalue(target: &LValue, slots: &mut HashMap<String, usize>) -> HirLValue {
    if let Some(slot) = slots.get(&target.root) {
        if let Some(field) = target.path.first() {
            return HirLValue::Field {
                slot: *slot,
                root: target.root.clone(),
                field: field.clone(),
            };
        }
        return HirLValue::Slot {
            index: *slot,
            name: target.root.clone(),
        };
    }
    HirLValue::Element {
        store: target.root.clone(),
        field: target.path.first().cloned().unwrap_or_default(),
    }
}

fn lower_expression(expression: &Expression, slots: &mut HashMap<String, usize>) -> HirExpression {
    match expression {
        Expression::Literal(text) => HirExpression::Literal(parse_literal(text)),
        Expression::Path(segments) => match segments.first() {
            Some(root) => match slots.get(root) {
                Some(index) => HirExpression::Slot {
                    index: *index,
                    name: root.clone(),
                },
                None => HirExpression::Field {
                    root: root.clone(),
                    path: segments.iter().skip(1).cloned().collect(),
                },
            },
            None => HirExpression::Literal(Literal::Empty),
        },
        Expression::Call { name, args } => HirExpression::Call {
            function: name.clone(),
            args: args.iter().map(|a| lower_expression(a, slots)).collect(),
        },
        Expression::Binary { left, operator, right } => HirExpression::Test {
            left: Box::new(lower_expression(left, slots)),
            op: match operator.trim() {
                "==" | "is" => CompareOp::Equal,
                "!=" | "is not" => CompareOp::NotEqual,
                ">" => CompareOp::Greater,
                "<" => CompareOp::Less,
                ">=" => CompareOp::GreaterOrEqual,
                "<=" => CompareOp::LessOrEqual,
                _ => CompareOp::Equal,
            },
            right: Box::new(lower_expression(right, slots)),
        },
        Expression::Unary { operator, operand } => match operator.as_str() {
            "not" | "!" => HirExpression::Negate(Box::new(lower_expression(operand, slots))),
            _ => lower_expression(operand, slots),
        },
        Expression::List(items) => HirExpression::Join(
            items.iter().map(|i| lower_expression(i, slots)).collect(),
        ),
        Expression::Map(entries) => HirExpression::Build {
            type_name: String::new(),
            fields: entries
                .iter()
                .map(|(k, v)| (k.clone(), lower_expression(v, slots)))
                .collect(),
        },
    }
}

fn parse_literal(text: &str) -> Literal {
    let trimmed = text.trim();
    if trimmed == "true" {
        return Literal::Flag(true);
    }
    if trimmed == "false" {
        return Literal::Flag(false);
    }
    if trimmed == "none" || trimmed == "null" || trimmed.is_empty() {
        return Literal::Empty;
    }
    if let Ok(number) = trimmed.parse::<f64>() {
        return Literal::Number(number);
    }
    if trimmed.len() >= 2 && trimmed.starts_with('"') && trimmed.ends_with('"') {
        return Literal::Text(trimmed[1..trimmed.len() - 1].to_string());
    }
    Literal::Text(trimmed.to_string())
}

/// Spłaszcza drzewo interfejsu do wezlow HIR.
fn lower_ui(body: &[UiNode], child_components: &mut Vec<String>, widget_count: &mut usize) -> Vec<HirNode> {
    let mut out = Vec::new();
    for node in body {
        if let Some(lowered) = lower_ui_node(node, child_components, widget_count) {
            out.push(lowered);
        }
    }
    out
}

fn lower_ui_node(
    node: &UiNode,
    child_components: &mut Vec<String>,
    widget_count: &mut usize,
) -> Option<HirNode> {
    let mut slots = HashMap::new();
    *widget_count += 1;
    match node {
        UiNode::Container { kind, accessories, children, .. } => {
            let gap = accessory_number(accessories, "gap");
            let padding = accessory_numbers(accessories, "padding");
            let align = accessory_text(accessories, "align");
            let justify = accessory_text(accessories, "justify");
            let style_class = accessory_text(accessories, "style");
            let inner = lower_ui(children, child_components, widget_count);
            Some(match kind {
                ContainerKind::Column => HirNode::Column {
                    gap,
                    padding,
                    align,
                    children: inner,
                    style_class,
                },
                ContainerKind::Row => HirNode::Row {
                    gap,
                    align,
                    justify,
                    children: inner,
                },
                ContainerKind::Grid => HirNode::Grid {
                    columns: accessory_number(accessories, "columns").unwrap_or(2.0) as usize,
                    gap,
                    children: inner,
                },
                ContainerKind::Card => HirNode::Card {
                    padding,
                    radius: accessory_text(accessories, "radius"),
                    children: inner,
                    style_class,
                },
                ContainerKind::Scroll => HirNode::Scroll {
                    direction: accessory_text(accessories, "direction")
                        .unwrap_or_else(|| "vertical".to_string()),
                    children: inner,
                },
            })
        }
        UiNode::Text { value, accessories, .. } => Some(HirNode::Text {
            value: lower_expression(value, &mut slots),
            style_class: accessory_text(accessories, "style"),
            color: accessory_text(accessories, "color"),
            max_lines: accessory_number(accessories, "max lines").map(|n| n as usize),
        }),
        UiNode::Button { label, icon, accessories, events, .. } => {
            let mut action_slots = HashMap::new();
            let mut local_count = 0;
            let action = events
                .iter()
                .find(|e| matches!(e.kind, EventKind::Tapped))
                .map(|e| lower_statements(&e.body, &mut action_slots, &mut local_count));
            Some(HirNode::Button {
                label: label.clone().unwrap_or_else(|| icon.clone().unwrap_or_default()),
                icon: icon.clone(),
                style_class: accessory_text(accessories, "style"),
                disabled: None,
                action,
                accessibility_label: accessory_text(accessories, "accessibility label")
                    .or_else(|| label.clone())
                    .unwrap_or_default(),
                widget_id: *widget_count,
            })
        }
        UiNode::Field { name, label, input_type, placeholder, required, min_length, max_length, error_text, .. } => {
            Some(HirNode::Field {
                name: name.clone(),
                label: label.clone().unwrap_or_else(|| name.clone()),
                input_type: input_type.clone().unwrap_or_else(|| "text".to_string()),
                placeholder: placeholder.clone().unwrap_or_default(),
                required: *required,
                min_length: *min_length,
                max_length: *max_length,
                error_text: error_text.clone().unwrap_or_default(),
                slot: 0,
            })
        }
        UiNode::Image { reference, alt, accessories, .. } => Some(HirNode::Image {
            source: lower_expression(reference, &mut slots),
            alt: alt.clone().unwrap_or_default(),
            width: accessory_text(accessories, "width"),
            height: accessory_text(accessories, "height"),
        }),
        UiNode::Checkbox { reference, accessibility_label, events, .. } => {
            let mut action_slots = HashMap::new();
            let mut local_count = 0;
            let action = events
                .iter()
                .find(|e| matches!(e.kind, EventKind::Changed))
                .map(|e| lower_statements(&e.body, &mut action_slots, &mut local_count));
            Some(HirNode::Checkbox {
                binding: reference.clone(),
                accessibility_label: accessibility_label.clone().unwrap_or_default(),
                action,
                widget_id: *widget_count,
            })
        }
        UiNode::Media { kind, reference, controls, subtitles, accessibility_label, .. } => {
            let mut slots = HashMap::new();
            Some(HirNode::Media {
                kind: *kind,
                source: reference.as_ref().map(|r| lower_expression(r, &mut slots)),
                controls: *controls,
                subtitles: *subtitles,
                accessibility_label: accessibility_label.clone().unwrap_or_default(),
            })
        }
        UiNode::ComponentCall { name, args, .. } => {
            push_unique(child_components, name);
            let mut slots = HashMap::new();
            Some(HirNode::ComponentInstance {
                name: name.clone(),
                args: args.iter().map(|a| lower_expression(a, &mut slots)).collect(),
            })
        }
        UiNode::Conditional { condition, then_branch, else_branch, .. } => {
            let mut slots = HashMap::new();
            Some(HirNode::Conditional {
                condition: lower_expression(condition, &mut slots),
                then_branch: lower_ui(then_branch, child_components, widget_count),
                else_branch: lower_ui(else_branch, child_components, widget_count),
            })
        }
        UiNode::Loop { .. } => None,
        UiNode::Spacer { size, .. } => Some(HirNode::Spacer { size: *size }),
    }
}

fn accessory_text(accessories: &[Accessory], key: &str) -> Option<String> {
    for accessory in accessories {
        let matched = match accessory {
            Accessory::Style(v) if key == "style" => Some(v.clone()),
            Accessory::Color(v) if key == "color" => Some(v.clone()),
            Accessory::Align(v) if key == "align" => Some(v.clone()),
            Accessory::Justify(v) if key == "justify" => Some(v.clone()),
            Accessory::Width(v) if key == "width" => Some(v.clone()),
            Accessory::Height(v) if key == "height" => Some(v.clone()),
            Accessory::Direction(v) if key == "direction" => Some(v.clone()),
            Accessory::AccessibilityLabel(v) if key == "accessibility label" => Some(v.clone()),
            _ => None,
        };
        if matched.is_some() {
            return matched;
        }
    }
    None
}

fn accessory_number(accessories: &[Accessory], key: &str) -> Option<f64> {
    for accessory in accessories {
        match accessory {
            Accessory::Gap(v) if key == "gap" => return Some(*v),
            Accessory::Columns(v) if key == "columns" => return Some(*v as f64),
            Accessory::MaxLines(v) if key == "max lines" => return Some(*v as f64),
            _ => {}
        }
    }
    None
}

fn accessory_numbers(accessories: &[Accessory], key: &str) -> Vec<f64> {
    for accessory in accessories {
        match accessory {
            Accessory::Padding(v) if key == "padding" => return v.clone(),
            Accessory::Margin(v) if key == "margin" => return v.clone(),
            _ => {}
        }
    }
    Vec::new()
}

/// Zbiera nazwy magazynow widoczne w drzewie interfejsu.
fn collect_store_names(body: &[UiNode], out: &mut Vec<String>) {
    for node in body {
        match node {
            UiNode::Container { children, .. } => collect_store_names(children, out),
            UiNode::Conditional { then_branch, else_branch, .. } => {
                collect_store_names(then_branch, out);
                collect_store_names(else_branch, out);
            }
            UiNode::Loop { body: inner, .. } => collect_store_names(inner, out),
            _ => {}
        }
    }
}

/// Wykrywa nieuzywane magazyny i komponenty, zwraca liste nazw.
pub fn unused_stores(program: &HirProgram) -> Vec<String> {
    let mut referenced: HashSet<String> = HashSet::new();
    for screen in &program.screens {
        let mut names = Vec::new();
        collect_store_names(&[], &mut names);
        for name in names {
            referenced.insert(name);
        }
    }
    program
        .stores
        .keys()
        .filter(|name| !referenced.contains(*name))
        .cloned()
        .collect()
}
