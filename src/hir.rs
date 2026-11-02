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
                    initial: s.initial.as_ref().map(|e| lower_expression(e, &mut HashMap::new())),
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

        let mut navigates_to = Vec::new();
        for event in &screen.events {
            collect_navigation(&event.body, &mut navigates_to);
        }

        screens.push(HirScreen {
            name: screen.name.clone(),
            title: screen
                .title
                .clone()
                .unwrap_or_else(|| screen.name.clone()),
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
            navigates_to,
        });
    }

    let tests = program
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
            version: program.app.version.clone().unwrap_or_else(|| "1.0.0".into()),
            language: program.app.language.clone().unwrap_or_else(|| "pl".into()),
            platforms: program.app.supports.clone(),
            orientation: program
                .app
                .orientation
                .clone()
                .unwrap_or_else(|| "automatic".into()),
            offline_mode: program.app.offline.clone().unwrap_or_else(|| "optional".into()),
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

fn collect_statement_facts(
    statements: &[Statement],
    stores: &mut Vec<String>,
    navigation: &mut Vec<String>,
    calls: &mut Vec<String>,
) {
    for stmt in statements {
        match stmt {
            Statement::Save { store, .. }
            | Statement::Load { store, .. }
            | Statement::Clear { store, .. } => stores.push(store.clone()),
            Statement::Add { target, .. } | Statement::Remove { target, .. } => {
                stores.push(target.clone())
            }
            Statement::Enqueue { target, .. } => stores.push(target.clone()),
            Statement::Navigate { screen, .. } => navigation.push(screen.clone()),
            Statement::Expression {
                value: Expression::Call { name, .. },
                ..
            } => calls.push(name.clone()),
            Statement::If {
                then_branch,
                else_branch,
                ..
            } => {
                collect_statement_facts(then_branch, stores, navigation, calls);
                collect_statement_facts(else_branch, stores, navigation, calls);
            }
            Statement::For { body, .. } => {
                collect_statement_facts(body, stores, navigation, calls)
            }
            Statement::TryCatch {
                try_body,
                catch_body,
                finally_body,
                ..
            } => {
                collect_statement_facts(try_body, stores, navigation, calls);
                collect_statement_facts(catch_body, stores, navigation, calls);
                collect_statement_facts(finally_body, stores, navigation, calls);
            }
            _ => {}
        }
    }
}

fn collect_navigation(statements: &[Statement], navigation: &mut Vec<String>) {
    let mut stores = Vec::new();
    let mut calls = Vec::new();
    collect_statement_facts(statements, &mut stores, navigation, &mut calls);
}

fn lower_statements(
    statements: &[Statement],
    slots: &mut HashMap<String, usize>,
    local_count: &mut usize,
) -> Vec<HirStatement> {
    let mut out = Vec::new();

    for stmt in statements {
        match stmt {
            Statement::Set { target, value, .. } => out.push(HirStatement::Assign {
                target: lower_lvalue(target, slots, local_count),
                value: lower_expression(value, slots),
            }),
            Statement::Add { value, target, .. } => out.push(HirStatement::Append {
                value: lower_expression(value, slots),
                store: target.clone(),
            }),
            Statement::Remove { value, target, .. } => out.push(HirStatement::Delete {
                value: lower_expression(value, slots),
                store: target.clone(),
            }),
            Statement::Update { target, value, .. } => out.push(HirStatement::Assign {
                target: lower_lvalue(target, slots, local_count),
                value: lower_expression(value, slots),
            }),
            Statement::Save { store, .. } => out.push(HirStatement::StoreSave {
                store: store.clone(),
            }),
            Statement::Load { store, .. } => out.push(HirStatement::StoreLoad {
                store: store.clone(),
            }),
            Statement::Clear { store, .. } => out.push(HirStatement::StoreClear {
                store: store.clone(),
            }),
            Statement::Return { value, .. } => out.push(HirStatement::Return {
                value: value.as_ref().map(|v| lower_expression(v, slots)),
            }),
            Statement::If {
                condition,
                then_branch,
                else_branch,
                ..
            } => out.push(HirStatement::Branch {
                condition: lower_expression(condition, slots),
                then_branch: lower_statements(then_branch, slots, local_count),
                else_branch: lower_statements(else_branch, slots, local_count),
            }),
            Statement::For {
                variable,
                source,
                body,
                ..
            } => {
                let slot = *local_count;
                *local_count += 1;
                slots.insert(variable.clone(), slot);
                out.push(HirStatement::Iterate {
                    variable: variable.clone(),
                    source: lower_expression(source, slots),
                    body: lower_statements(body, slots, local_count),
                    slot,
                });
            }
            Statement::TryCatch {
                try_body,
                catch_var,
                catch_body,
                finally_body,
                ..
            } => {
                let var = catch_var.clone().unwrap_or_else(|| "error".into());
                let slot = *local_count;
                *local_count += 1;
                slots.insert(var.clone(), slot);
                out.push(HirStatement::Guard {
                    attempt: lower_statements(try_body, slots, local_count),
                    catch_var: var,
                    on_error: lower_statements(catch_body, slots, local_count),
                    always: lower_statements(finally_body, slots, local_count),
                });
            }
            Statement::Expression {
                value: Expression::Call { name, args },
                ..
            } => out.push(HirStatement::Invoke {
                function: name.clone(),
                args: args.iter().map(|a| lower_expression(a, slots)).collect(),
            }),
            Statement::Expression { value, .. } => out.push(HirStatement::Express {
                value: lower_expression(value, slots),
            }),
            Statement::Navigate { screen, args, .. } => out.push(HirStatement::Goto {
                screen: screen.clone(),
                args: args.iter().map(|a| lower_expression(a, slots)).collect(),
            }),
            Statement::GoBack { .. } => out.push(HirStatement::Leave),
            Statement::Announce { value, urgent, .. } => out.push(HirStatement::Notify {
                text: lower_expression(value, slots),
                urgent: *urgent,
            }),
            Statement::Vibrate { pattern, .. } => out.push(HirStatement::Buzz {
                pattern: pattern.clone(),
            }),
            Statement::Raise { value, .. } => out.push(HirStatement::Abort {
                value: lower_expression(value, slots),
            }),
        }
    }

    out
}

fn lower_lvalue(
    target: &LValue,
    slots: &mut HashMap<String, usize>,
    local_count: &mut usize,
) -> HirLValue {
    let root = target.root_name();
    let slot = match slots.get(root) {
        Some(s) => *s,
        None => {
            let s = *local_count;
            *local_count += 1;
            slots.insert(root.to_string(), s);
            s
        }
    };

    match target {
        LValue::Variable(name) => HirLValue::Slot {
            index: slot,
            name: name.clone(),
        },
        LValue::Field { object, field, .. } => HirLValue::Field {
            slot,
            root: object.clone(),
            field: field.clone(),
        },
        LValue::Indexed { name, .. } => HirLValue::Slot {
            index: slot,
            name: name.clone(),
        },
    }
}

fn lower_expression(expr: &Expression, slots: &mut HashMap<String, usize>) -> HirExpression {
    match expr {
        Expression::Text(t) => HirExpression::Literal(Literal::Text(t.clone())),
        Expression::Number(n) => HirExpression::Literal(Literal::Number(*n)),
        Expression::Boolean(b) => HirExpression::Literal(Literal::Flag(*b)),
        Expression::None => HirExpression::Literal(Literal::Empty),
        Expression::Identifier(name) => {
            let index = match slots.get(name) {
                Some(i) => *i,
                None => {
                    let i = slots.len();
                    slots.insert(name.clone(), i);
                    i
                }
            };
            HirExpression::Slot {
                index,
                name: name.clone(),
            }
        }
        Expression::Path { root, fields } => HirExpression::Field {
            root: root.clone(),
            path: fields.clone(),
        },
        Expression::EnumValue {
            enum_name,
            variant,
        } => HirExpression::Variant {
            type_name: enum_name.clone(),
            variant: variant.clone(),
        },
        Expression::Concatenate(parts) => HirExpression::Join(
            parts.iter().map(|p| lower_expression(p, slots)).collect(),
        ),
        Expression::Compare { left, op, right } => HirExpression::Test {
            left: Box::new(lower_expression(left, slots)),
            op: *op,
            right: Box::new(lower_expression(right, slots)),
        },
        Expression::Call { name, args } => HirExpression::Call {
            function: name.clone(),
            args: args.iter().map(|a| lower_expression(a, slots)).collect(),
        },
        Expression::Constructor { type_name, fields } => HirExpression::Build {
            type_name: type_name.clone(),
            fields: fields
                .iter()
                .map(|(k, v)| (k.clone(), lower_expression(v, slots)))
                .collect(),
        },
        Expression::List(items) => HirExpression::Collection(
            items.iter().map(|i| lower_expression(i, slots)).collect(),
        ),
        Expression::Object(fields) => HirExpression::Build {
            type_name: "Json".into(),
            fields: fields
                .iter()
                .map(|(k, v)| (k.clone(), lower_expression(v, slots)))
                .collect(),
        },
        Expression::Length(inner) => {
            HirExpression::Count(Box::new(lower_expression(inner, slots)))
        }
        Expression::Negate(inner) => {
            HirExpression::Negate(Box::new(lower_expression(inner, slots)))
        }
    }
}

fn lower_ui(
    nodes: &[UiNode],
    child_components: &mut Vec<String>,
    widget_count: &mut usize,
) -> Vec<HirNode> {
    let mut out = Vec::new();

    for node in nodes {
        match node {
            UiNode::Container {
                kind,
                source,
                accessories,
                children,
                ..
            } => {
                let inner = lower_ui(children, child_components, widget_count);

                if let Some(src) = source {
                    // kontener z zrodlem jest lista; rozdzielamy szablon elementu
                    // i szablon pustego stanu
                    let mut item_template = Vec::new();
                    let mut empty_template = Vec::new();
                    for child in &inner {
                        match child {
                            HirNode::Card { children: cc, .. } if cc.is_empty() => {
                                empty_template.push(child.clone())
                            }
                            _ => item_template.push(child.clone()),
                        }
                    }
                    *widget_count += 1;
                    out.push(HirNode::List {
                        source: src.clone(),
                        item_var: src.clone(),
                        item_template,
                        empty_template,
                        widget_id: *widget_count,
                    });
                    continue;
                }

                match kind {
                    ContainerKind::Column => {
                        let gap = find_gap(accessories);
                        let padding = find_padding(accessories);
                        let style_class = find_style(accessories);
                        out.push(HirNode::Column {
                            gap,
                            padding,
                            align: find_align(accessories),
                            children: inner,
                            style_class,
                        });
                    }
                    ContainerKind::Row => {
                        out.push(HirNode::Row {
                            gap: find_gap(accessories),
                            align: find_align(accessories),
                            justify: find_justify(accessories),
                            children: inner,
                        });
                    }
                    ContainerKind::Grid => {
                        out.push(HirNode::Grid {
                            columns: find_columns(accessories).unwrap_or(2),
                            gap: find_gap(accessories),
                            children: inner,
                        });
                    }
                    ContainerKind::Card => {
                        out.push(HirNode::Card {
                            padding: find_padding(accessories),
                            radius: None,
                            children: inner,
                            style_class: find_style(accessories),
                        });
                    }
                    ContainerKind::Scroll => {
                        out.push(HirNode::Scroll {
                            direction: find_direction(accessories),
                            children: inner,
                        });
                    }
                }
            }
            UiNode::Text {
                value,
                accessories,
                ..
            } => out.push(HirNode::Text {
                value: lower_expression(value, &mut HashMap::new()),
                style_class: find_style(accessories),
                color: find_color(accessories),
                max_lines: find_max_lines(accessories),
            }),
            UiNode::Button {
                label,
                icon,
                accessories,
                events,
                ..
            } => {
                *widget_count += 1;
                let action = events
                    .iter()
                    .find(|e| e.kind == EventKind::Tapped)
                    .map(|e| lower_statements(&e.body, &mut HashMap::new(), &mut 0));

                let accessibility_label = accessories
                    .iter()
                    .find_map(|a| match a {
                        Accessory::AccessibilityLabel(l) => Some(l.clone()),
                        _ => None,
                    })
                    .unwrap_or_else(|| label.clone().unwrap_or_default());

                out.push(HirNode::Button {
                    label: label.clone().unwrap_or_default(),
                    icon: icon.clone(),
                    style_class: find_style(accessories),
                    disabled: accessories.iter().find_map(|a| match a {
                        Accessory::Disabled(e) => Some(lower_expression(e, &mut HashMap::new())),
                        _ => None,
                    }),
                    action,
                    accessibility_label,
                    widget_id: *widget_count,
                });
            }
            UiNode::Field {
                name,
                label,
                input_type,
                placeholder,
                required,
                min_length,
                max_length,
                error_text,
                ..
            } => {
                *widget_count += 1;
                out.push(HirNode::Field {
                    name: name.clone(),
                    label: label.clone().unwrap_or_else(|| name.clone()),
                    input_type: input_type.clone().unwrap_or_else(|| "text".into()),
                    placeholder: placeholder.clone().unwrap_or_default(),
                    required: *required,
                    min_length: *min_length,
                    max_length: *max_length,
                    error_text: error_text.clone().unwrap_or_default(),
                    slot: *widget_count,
                });
            }
            UiNode::Image {
                reference,
                alt,
                accessories,
                ..
            } => out.push(HirNode::Image {
                source: lower_expression(reference, &mut HashMap::new()),
                alt: alt.clone().unwrap_or_default(),
                width: accessories.iter().find_map(|a| match a {
                    Accessory::Width(w) => Some(w.clone()),
                    _ => None,
                }),
                height: accessories.iter().find_map(|a| match a {
                    Accessory::Height(h) => Some(h.clone()),
                    _ => None,
                }),
            }),
            UiNode::Checkbox {
                reference,
                accessibility_label,
                events,
                ..
            } => {
                *widget_count += 1;
                out.push(HirNode::Checkbox {
                    binding: reference.clone(),
                    accessibility_label: accessibility_label.clone().unwrap_or_default(),
                    action: events
                        .iter()
                        .find(|e| e.kind == EventKind::Changed)
                        .map(|e| lower_statements(&e.body, &mut HashMap::new(), &mut 0)),
                    widget_id: *widget_count,
                });
            }
            UiNode::Media {
                kind,
                reference,
                controls,
                subtitles,
                accessibility_label,
                ..
            } => out.push(HirNode::Media {
                kind: *kind,
                source: reference
                    .as_ref()
                    .map(|r| lower_expression(r, &mut HashMap::new())),
                controls: *controls,
                subtitles: *subtitles,
                accessibility_label: accessibility_label.clone().unwrap_or_default(),
            }),
            UiNode::ComponentCall { name, args, .. } => {
                child_components.push(name.clone());
                out.push(HirNode::ComponentInstance {
                    name: name.clone(),
                    args: args
                        .iter()
                        .map(|a| lower_expression(a, &mut HashMap::new()))
                        .collect(),
                });
            }
            UiNode::Conditional {
                condition,
                then_branch,
                else_branch,
                ..
            } => out.push(HirNode::Conditional {
                condition: lower_expression(condition, &mut HashMap::new()),
                then_branch: lower_ui(then_branch, child_components, widget_count),
                else_branch: lower_ui(else_branch, child_components, widget_count),
            }),
            UiNode::Loop {
                variable: _,
                source: _,
                body,
                ..
            } => {
                out.extend(lower_ui(body, child_components, widget_count));
            }
            UiNode::Spacer { size, .. } => out.push(HirNode::Spacer { size: *size }),
        }
    }

    out
}

fn find_style(accessories: &[Accessory]) -> Option<String> {
    accessories.iter().find_map(|a| match a {
        Accessory::Style(s) => Some(s.clone()),
        _ => None,
    })
}

fn find_color(accessories: &[Accessory]) -> Option<String> {
    accessories.iter().find_map(|a| match a {
        Accessory::Color(c) => Some(c.clone()),
        _ => None,
    })
}

fn find_gap(accessories: &[Accessory]) -> Option<f64> {
    accessories.iter().find_map(|a| match a {
        Accessory::Gap(g) => Some(*g),
        _ => None,
    })
}

fn find_padding(accessories: &[Accessory]) -> Vec<f64> {
    accessories
        .iter()
        .find_map(|a| match a {
            Accessory::Padding(p) => Some(p.clone()),
            _ => None,
        })
        .unwrap_or_default()
}

fn find_align(accessories: &[Accessory]) -> Option<String> {
    accessories.iter().find_map(|a| match a {
        Accessory::Align(a) => Some(a.clone()),
        _ => None,
    })
}

fn find_justify(accessories: &[Accessory]) -> Option<String> {
    accessories.iter().find_map(|a| match a {
        Accessory::Justify(j) => Some(j.clone()),
        _ => None,
    })
}

fn find_columns(accessories: &[Accessory]) -> Option<usize> {
    accessories.iter().find_map(|a| match a {
        Accessory::Columns(c) => Some(*c),
        _ => None,
    })
}

fn find_direction(accessories: &[Accessory]) -> String {
    accessories
        .iter()
        .find_map(|a| match a {
            Accessory::Direction(d) => Some(d.clone()),
            _ => None,
        })
        .unwrap_or_else(|| "vertical".into())
}

fn find_max_lines(accessories: &[Accessory]) -> Option<usize> {
    accessories.iter().find_map(|a| match a {
        Accessory::MaxLines(n) => Some(*n),
        _ => None,
    })
}

/// Zwraca liczbe wezlow interfejsu w calym programie.
pub fn count_widgets(hir: &HirProgram) -> usize {
    let screens: usize = hir.screens.iter().map(|s| s.widget_count).sum();
    let components: usize = hir.components.values().map(|c| c.widget_count).sum();
    screens + components
}

/// Zbiera wszystkie magazyny uzywane w programie.
pub fn collect_used_stores(hir: &HirProgram) -> HashSet<String> {
    let mut used = HashSet::new();
    for function in hir.functions.values() {
        for store in &function.uses_stores {
            used.insert(store.clone());
        }
    }
    used
}

/// Zwraca funkcje, ktore nie sa wywolywane z nigadnego miejsca.
pub fn find_unused_functions(hir: &HirProgram) -> Vec<String> {
    let mut called: HashSet<String> = HashSet::new();

    for function in hir.functions.values() {
        for name in &function.calls {
            called.insert(name.clone());
        }
    }

    for screen in &hir.screens {
        for event in &screen.events {
            collect_calls_from_statements(&event.body, &mut called);
        }
    }

    hir.functions
        .keys()
        .filter(|name| !called.contains(*name))
        .cloned()
        .collect()
}

fn collect_calls_from_statements(statements: &[HirStatement], called: &mut HashSet<String>) {
    for stmt in statements {
        match stmt {
            HirStatement::Invoke { function, .. } => {
                called.insert(function.clone());
            }
            HirStatement::Branch {
                then_branch,
                else_branch,
                ..
            } => {
                collect_calls_from_statements(then_branch, called);
                collect_calls_from_statements(else_branch, called);
            }
            HirStatement::Iterate { body, .. } => collect_calls_from_statements(body, called),
            HirStatement::Guard {
                attempt,
                on_error,
                always,
                ..
            } => {
                collect_calls_from_statements(attempt, called);
                collect_calls_from_statements(on_error, called);
                collect_calls_from_statements(always, called);
            }
            _ => {}
        }
    }
}
