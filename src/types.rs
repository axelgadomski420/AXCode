// src/types.rs — analiza typow i tablica symboli AXCode
//
// Sprawdza spojnosc typow w wyrazeniach: przypisania stanu, wywolania funkcji,
// operacje na listach, pola modeli danych w srodku item, oraz zwracane wartosci.
// Pracuje na AST po parsowaniu i dopisuje diagnostyki do wspolnego zbioru.

use crate::ast::*;
use crate::diagnostics::{Diagnostics, ErrorCatalog};
use std::collections::HashMap;

/// Pojedynczy symbol widoczny w zasiegu.
#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub ty: TypeRef,
    pub mutable: bool,
}

/// Tablica symboli podzielona na zasiegi.
#[derive(Debug, Default, Clone)]
pub struct Scope {
    pub symbols: HashMap<String, Symbol>,
    pub parent_index: Option<usize>,
}

/// Caly kontekst typowania.
pub struct TypeContext {
    pub scopes: Vec<Scope>,
    pub data_types: HashMap<String, Vec<FieldDecl>>,
    pub data_field_types: HashMap<String, HashMap<String, TypeRef>>,
    pub enums: HashMap<String, Vec<String>>,
    pub functions: HashMap<String, FunctionSignature>,
    pub apis: HashMap<String, HashMap<String, FunctionSignature>>,
    pub stores: HashMap<String, TypeRef>,
    pub store_is_list: HashMap<String, bool>,
    pub params: HashMap<String, TypeRef>,
}

/// Sygnatura funkcji lub metody API.
#[derive(Debug, Clone, Default)]
pub struct FunctionSignature {
    pub params: Vec<TypeRef>,
    pub returns: Option<TypeRef>,
    pub is_api: bool,
    pub api_name: Option<String>,
}

impl TypeContext {
    /// Buduje kontekst na podstawie zadeklarowanych typow.
    pub fn build(program: &Program) -> Self {
        let mut ctx = TypeContext {
            scopes: vec![Scope::default()],
            data_types: HashMap::new(),
            data_field_types: HashMap::new(),
            enums: HashMap::new(),
            functions: HashMap::new(),
            apis: HashMap::new(),
            stores: HashMap::new(),
            store_is_list: HashMap::new(),
            params: HashMap::new(),
        };

        for data in &program.data {
            ctx.data_types.insert(data.name.clone(), data.fields.clone());
            let mut fields = HashMap::new();
            for f in &data.fields {
                fields.insert(f.name.clone(), f.ty.clone());
            }
            ctx.data_field_types.insert(data.name.clone(), fields);
        }

        for e in &program.enums {
            ctx.enums.insert(e.name.clone(), e.variants.iter().map(|v| v.name.clone()).collect());
        }

        for f in &program.functions {
            ctx.functions.insert(
                f.name.clone(),
                FunctionSignature {
                    params: f.params.iter().map(|p| p.ty.clone()).collect(),
                    returns: f.returns.clone(),
                    is_api: false,
                    api_name: None,
                },
            );
        }

        for api in &program.apis {
            let mut methods = HashMap::new();
            for f in &api.functions {
                methods.insert(
                    f.name.clone(),
                    FunctionSignature {
                        params: f.params.iter().map(|p| p.ty.clone()).collect(),
                        returns: f.returns.clone(),
                        is_api: true,
                        api_name: Some(api.name.clone()),
                    },
                );
            }
            ctx.apis.insert(api.name.clone(), methods);
        }

        for store in &program.stores {
            let is_list = matches!(store.shape, StoreShape::List(_));
            ctx.store_is_list.insert(store.name.clone(), is_list);
            ctx.stores.insert(store.name.clone(), store.shape.inner_type());
        }

        for g in &program.globals {
            ctx.declare_global(&g.name, g.ty.clone());
        }
        for p in &program.permissions {
            let _ = p;
        }

        ctx
    }

    fn declare_global(&mut self, name: &str, ty: TypeRef) {
        if let Some(scope) = self.scopes.first_mut() {
            scope.symbols.insert(
                name.to_string(),
                Symbol { name: name.to_string(), ty, mutable: true },
            );
        }
    }

    fn push_scope(&mut self) {
        let parent = Some(self.scopes.len() - 1);
        self.scopes.push(Scope { symbols: HashMap::new(), parent_index: parent });
    }

    fn pop_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    fn declare(&mut self, name: &str, ty: TypeRef, mutable: bool) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.symbols.insert(
                name.to_string(),
                Symbol { name: name.to_string(), ty, mutable },
            );
        }
    }

    /// Szuka symbolu po nazwie, idac w gore stosu zasiegow.
    pub fn lookup(&self, name: &str) -> Option<&Symbol> {
        let mut idx = Some(self.scopes.len() - 1);
        while let Some(i) = idx {
            if let Some(sym) = self.scopes[i].symbols.get(name) {
                return Some(sym);
            }
            idx = self.scopes[i].parent_index;
        }
        None
    }

    /// Typ pola w modelu, np. message.body.
    fn field_type(&self, model: &str, field: &str) -> Option<TypeRef> {
        self.data_field_types
            .get(model)
            .and_then(|m| m.get(field))
            .cloned()
    }

    /// Rozwiazuje typ wyrazenia po nazwie sciezki.
    pub fn resolve_path(&self, path: &str) -> Option<TypeRef> {
        let cleaned = path.trim();
        if cleaned.is_empty() {
            return None;
        }
        if let Some(lit) = literal_type(cleaned) {
            return Some(lit);
        }
        let segments: Vec<&str> = cleaned.split('.').collect();
        let head = segments.first()?.trim();

        // odczyt wlasciwosci obiektu: currentUser.displayName
        if segments.len() > 1 {
            if let Some(root) = self.lookup(head) {
                let base = unwrap_optional(&root.ty);
                if let TypeRef::Named(model) = base {
                    return self.field_type(&model, segments[1].trim());
                }
                if let TypeRef::Json = base {
                    return Some(TypeRef::Json);
                }
            }
            // sciezka zagniezdzona: model pole pole
            if let Some(first) = self.lookup(head) {
                if let TypeRef::Named(model) = unwrap_optional(&first.ty) {
                    if let Some(mid) = self.field_type(&model, segments[1].trim()) {
                        if let TypeRef::Named(inner) = unwrap_optional(&mid) {
                            if let Some(t) = segments.get(2) {
                                return self.field_type(&inner, t.trim());
                            }
                        }
                    }
                }
            }
            return Some(TypeRef::Json);
        }

        if let Some(sym) = self.lookup(head) {
            return Some(sym.ty.clone());
        }
        if let Some(ty) = self.stores.get(head) {
            return Some(ty.clone());
        }
        if self.data_types.contains_key(head) {
            return Some(TypeRef::Named(head.to_string()));
        }
        None
    }

    /// Zwraca element listy, gdy symbol jest lista.
    fn element_type(&self, name: &str) -> Option<TypeRef> {
        if let Some(sym) = self.lookup(name) {
            if let TypeRef::List(inner) = &sym.ty {
                return Some((**inner).clone());
            }
        }
        if let Some(ty) = self.stores.get(name) {
            if self.store_is_list.get(name).copied().unwrap_or(false) {
                return Some(ty.clone());
            }
        }
        None
    }

    /// Sprawdza, czy wywolanie funkcji ma zgodna liczbe i typy argumentow.
    fn check_call(&self, name: &str, args: &[String], diagnostics: &mut Diagnostics, line: usize) {
        let target = name.trim();
        if let Some((api_name, method)) = target.split_once('.') {
            if let Some(methods) = self.apis.get(api_name.trim()) {
                match methods.get(method.trim()) {
                    Some(sig) => {
                        if sig.params.len() != args.iter().filter(|a| !a.trim().is_empty()).count() {
                            diagnostics.push_diagnostic(Diagnostics::single(
                                ErrorCatalog::Api(format!("{}.{} oczekuje {} argumentow", api_name, method, sig.params.len())),
                                line,
                                1,
                            ));
                        }
                    }
                    None => {
                        diagnostics.push_diagnostic(Diagnostics::single(
                            ErrorCatalog::Api(format!("warstwa {} nie ma funkcji {}", api_name, method)),
                            line,
                            1,
                        ));
                    }
                }
                return;
            }
        }

        if let Some(sig) = self.functions.get(target) {
            let provided = args.iter().filter(|a| !a.trim().is_empty()).count();
            if sig.params.len() != provided {
                diagnostics.push_diagnostic(Diagnostics::single(
                    ErrorCatalog::Types(format!("funkcja {} oczekuje {} argumentow, podano {}", target, sig.params.len(), provided)),
                    line,
                    1,
                ));
            }
            for (i, arg) in args.iter().enumerate() {
                if arg.trim().is_empty() {
                    continue;
                }
                let expected = sig.params.get(i);
                let actual = self.resolve_path(arg.trim());
                if let (Some(e), Some(a)) = (expected, actual) {
                    if !types_compatible(e, &a) {
                        diagnostics.push_diagnostic(Diagnostics::single(
                            ErrorCatalog::Types(format!("argument {} funkcji {} ma zly typ", i + 1, target)),
                            line,
                            1,
                        ));
                    }
                }
            }
            return;
        }

        if self.lookup(target).is_some() {
            // wywolanie lambdy zapisane w stanie: sprawdzenie miekkie
            return;
        }

        if is_builtin(target) {
            return;
        }

        diagnostics.push_diagnostic(Diagnostics::single(
            ErrorCatalog::Types(format!("nieznana funkcja {}", target)),
            line,
            1,
        ));
    }
}

/// Punkt wejscia analizy typow.
pub fn check_types(program: &Program, diagnostics: &mut Diagnostics) {
    let mut ctx = TypeContext::build(program);

    for screen in &program.screens {
        ctx.push_scope();
        for p in &screen.params {
            ctx.declare(&p.name, p.ty.clone(), false);
        }
        for st in &screen.states {
            ctx.declare(&st.name, st.ty.clone(), true);
        }
        check_statements(&screen.body, &mut ctx, diagnostics);
        ctx.pop_scope();
    }

    for comp in &program.components {
        ctx.push_scope();
        for p in &comp.params {
            ctx.declare(&p.name, p.ty.clone(), false);
        }
        check_statements(&comp.body, &mut ctx, diagnostics);
        ctx.pop_scope();
    }

    for f in &program.functions {
        ctx.push_scope();
        for p in &f.params {
            ctx.declare(&p.name, p.ty.clone(), false);
        }
        check_statements(&f.body, &mut ctx, diagnostics);
        check_returns(&f.body, &f.returns, diagnostics, f.line, &mut ctx);
        ctx.pop_scope();
    }

    for t in &program.tests {
        let _ = t;
    }
}

/// Sprawdza liste instrukcji w danym zasiegu.
fn check_statements(body: &[Stmt], ctx: &mut TypeContext, diagnostics: &mut Diagnostics) {
    for stmt in body {
        match stmt {
            Stmt::Set { expr, line } => {
                if let Some((lhs, rhs)) = expr.split_once('=') {
                    let name = lhs.trim();
                    let value = rhs.trim();
                    let target_ty = ctx.lookup(name).map(|s| s.ty.clone());
                    if let Some(t) = &target_ty {
                        if let Some(v) = ctx.resolve_path(value) {
                            if !types_compatible(t, &v) {
                                diagnostics.push_diagnostic(Diagnostics::single(
                                    ErrorCatalog::Types(format!("przypisanie do {} ma niezgodny typ", name)),
                                    *line,
                                    1,
                                ));
                            }
                        }
                    } else if !ctx.store_is_list.keys().any(|k| k == name) {
                        diagnostics.push_diagnostic(Diagnostics::single(
                            ErrorCatalog::State(format!("przypisanie do niezadeklarowanego symbolu {}", name)),
                            *line,
                            1,
                        ));
                    }
                } else if !expr.trim().is_empty() && !expr.contains('=') {
                    diagnostics.push_diagnostic(Diagnostics::single(
                        ErrorCatalog::Syntax(format!("niepoprawne przypisanie: {}", expr)),
                        *line,
                        1,
                    ));
                }
            }
            Stmt::Call { name, args, line } => {
                let arg_list: Vec<String> = split_args(args);
                ctx.check_call(name, &arg_list, diagnostics, *line);
            }
            Stmt::Add { expr, line } => {
                let (value, target) = match expr.split_once(" to ") {
                    Some((v, t)) => (v.trim(), t.trim()),
                    None => (expr.trim(), ""),
                };
                if !target.is_empty() {
                    if let Some(elem) = ctx.element_type(target) {
                        let model = unwrap_optional(&elem);
                        // konstruktor: Message(id = ..., body = ...)
                        if let Some(open) = value.find('(') {
                            let ctor = value[..open].trim();
                            match &model {
                                TypeRef::Named(expected) if ctor != expected => {
                                    diagnostics.push_diagnostic(Diagnostics::single(
                                        ErrorCatalog::Types(format!("do listy {} dodawany jest {}, a oczekiwany {}", target, ctor, expected)),
                                        *line,
                                        1,
                                    ));
                                }
                                _ => {}
                            }
                            let inner = value[open + 1..].trim_end_matches(')');
                            check_constructor_fields(inner, &model, ctx, diagnostics, *line);
                        }
                    } else {
                        diagnostics.push_diagnostic(Diagnostics::single(
                            ErrorCatalog::State(format!("{} nie jest magazynem listy", target)),
                            *line,
                            1,
                        ));
                    }
                }
            }
            Stmt::Remove { expr, line } => {
                if !expr.contains(" from ") {
                    diagnostics.push_diagnostic(Diagnostics::single(
                        ErrorCatalog::Syntax(format!("usuwanie bez magazynu: {}", expr)),
                        *line,
                        1,
                    ));
                }
            }
            Stmt::Save { target, line } | Stmt::Load { target, line } | Stmt::Clear { target, line } => {
                let name = target.split_whitespace().next().unwrap_or("").trim();
                if !name.is_empty()
                    && !ctx.stores.contains_key(name)
                    && !matches!(name, "outbox" | "queue" | "cache")
                {
                    diagnostics.push_diagnostic(Diagnostics::single(
                        ErrorCatalog::State(format!("nieznany magazyn {}", name)),
                        *line,
                        1,
                    ));
                }
            }
            Stmt::Container { target, body, kind, line } => {
                // list messages → deklaracja symbolu elementu w item
                if kind == "list" && !target.trim().is_empty() {
                    let list_name = target.trim();
                    if let Some(elem) = ctx.element_type(list_name) {
                        ctx.push_scope();
                        ctx.declare("item", elem, false);
                        check_statements(body, ctx, diagnostics);
                        ctx.pop_scope();
                        continue;
                    }
                }
                let _ = line;
                check_statements(body, ctx, diagnostics);
            }
            Stmt::Block { body, .. } => {
                ctx.push_scope();
                check_statements(body, ctx, diagnostics);
                ctx.pop_scope();
            }
            Stmt::When { body, .. } => {
                ctx.push_scope();
                check_statements(body, ctx, diagnostics);
                ctx.pop_scope();
            }
            Stmt::Derive { body, .. } => {
                ctx.push_scope();
                check_statements(body, ctx, diagnostics);
                ctx.pop_scope();
            }
            Stmt::Try { body, handler, finally, .. } => {
                ctx.push_scope();
                check_statements(body, ctx, diagnostics);
                ctx.pop_scope();
                ctx.push_scope();
                ctx.declare("error", TypeRef::Named("Error".to_string()), false);
                check_statements(handler, ctx, diagnostics);
                ctx.pop_scope();
                ctx.push_scope();
                check_statements(finally, ctx, diagnostics);
                ctx.pop_scope();
            }
            Stmt::Go { target, line } | Stmt::Open { target, line } => {
                if let Some(open) = target.find('(') {
                    let inner = target[open + 1..].trim_end_matches(')').to_string();
                    for arg in split_args(&inner) {
                        let a = arg.trim();
                        if a.is_empty() {
                            continue;
                        }
                        if ctx.resolve_path(a).is_none() && !ctx.store_is_list.contains_key(a) {
                            diagnostics.push_diagnostic(Diagnostics::single(
                                ErrorCatalog::Platform(format!("parametr nawigacji {} nie istnieje", a)),
                                *line,
                                1,
                            ));
                        }
                    }
                }
            }
            Stmt::Announce { line, .. } => {
                let _ = line;
            }
            Stmt::Return { expr, line } => {
                if !expr.trim().is_empty() && expr.trim() != "none" {
                    if ctx.resolve_path(expr).is_none() && !expr.contains('(') && !is_builtin(expr.trim()) {
                        diagnostics.push_diagnostic(Diagnostics::single(
                            ErrorCatalog::Types(format!("zwracany symbol {} nie istnieje", expr.trim())),
                            *line,
                            1,
                        ));
                    }
                }
            }
            _ => {}
        }
    }
}

/// Sprawdza pola przekazane do konstruktora modelu.
fn check_constructor_fields(inner: &str, model: &TypeRef, ctx: &TypeContext, diagnostics: &mut Diagnostics, line: usize) {
    let TypeRef::Named(name) = model else { return };
    let Some(fields) = ctx.data_field_types.get(name) else { return };
    let mut assigned: Vec<String> = Vec::new();
    for part in split_args(inner) {
        let p = part.trim();
        if p.is_empty() {
            continue;
        }
        if let Some((k, v)) = p.split_once('=') {
            let key = k.trim();
            assigned.push(key.to_string());
            if !fields.contains_key(key) {
                diagnostics.push_diagnostic(Diagnostics::single(
                    ErrorCatalog::Types(format!("model {} nie ma pola {}", name, key)),
                    line,
                    1,
                ));
                continue;
            }
            if let Some(expected) = fields.get(key) {
                if let Some(actual) = ctx.resolve_path(v.trim()) {
                    if !types_compatible(expected, &actual) {
                        diagnostics.push_diagnostic(Diagnostics::single(
                            ErrorCatalog::Types(format!("pole {} modelu {} ma niezgodny typ", key, name)),
                            line,
                            1,
                        ));
                    }
                }
            }
        }
    }
    for field in fields.keys() {
        if !assigned.contains(field) {
            let optional = ctx
                .data_types
                .get(name)
                .and_then(|fs| fs.iter().find(|f| &f.name == field))
                .map(|f| matches!(f.ty, TypeRef::Optional(_)))
                .unwrap_or(false);
            if !optional {
                diagnostics.push_diagnostic(Diagnostics::single(
                    ErrorCatalog::Types(format!("brakuje pola {} w konstruktorze {}", field, name)),
                    line,
                    1,
                ));
            }
        }
    }
}

/// Sprawdza zgodnosc instrukcji return z zadeklarowanym typem zwracanym.
fn check_returns(body: &[Stmt], returns: &Option<TypeRef>, diagnostics: &mut Diagnostics, line: usize, ctx: &mut TypeContext) {
    let has_return = body.iter().any(collect_has_return);
    match returns {
        Some(expected) => {
            if !has_return {
                diagnostics.push_diagnostic(Diagnostics::single(
                    ErrorCatalog::Types(format!("funkcja deklaruje zwrot {:?}, ale nigdy nie zwraca wartosci", expected)),
                    line,
                    1,
                ));
            } else {
                for stmt in body {
                    if let Stmt::Return { expr, line: rline } = stmt {
                        if let Some(actual) = ctx.resolve_path(expr.trim()) {
                            if !types_compatible(expected, &actual) {
                                diagnostics.push_diagnostic(Diagnostics::single(
                                    ErrorCatalog::Types("zwracana wartosc ma niezgodny typ".to_string()),
                                    *rline,
                                    1,
                                ));
                            }
                        }
                    }
                }
            }
        }
        None => {
            if has_return {
                diagnostics.push_diagnostic(Diagnostics::single(
                    ErrorCatalog::Types("funkcja zwraca wartosc, ale nie deklaruje typu zwracanego".to_string()),
                    line,
                    1,
                ));
            }
        }
    }
}

fn collect_has_return(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Return { .. } => true,
        Stmt::Container { body, .. }
        | Stmt::Block { body, .. }
        | Stmt::When { body, .. }
        | Stmt::Derive { body, .. } => body.iter().any(collect_has_return),
        Stmt::Try { body, handler, finally, .. } => {
            body.iter().any(collect_has_return)
                || handler.iter().any(collect_has_return)
                || finally.iter().any(collect_has_return)
        }
        _ => false,
    }
}

/// Rozpoznaje typ literalu.
fn literal_type(s: &str) -> Option<TypeRef> {
    let t = s.trim();
    if t.is_empty() {
        return None;
    }
    if t.starts_with('"') && t.ends_with('"') && t.len() >= 2 {
        return Some(TypeRef::Text);
    }
    if t == "true" || t == "false" {
        return Some(TypeRef::Boolean);
    }
    if t == "none" || t == "null" {
        return Some(TypeRef::Optional(Box::new(TypeRef::Json)));
    }
    if t.parse::<f64>().is_ok() {
        return Some(TypeRef::Number);
    }
    if t.starts_with('[') && t.ends_with(']') {
        return Some(TypeRef::List(Box::new(TypeRef::Json)));
    }
    None
}

/// Zdejmuje opcjonalnosc z typu.
fn unwrap_optional(ty: &TypeRef) -> TypeRef {
    match ty {
        TypeRef::Optional(inner) => (**inner).clone(),
        other => other.clone(),
    }
}

/// Sprawdza, czy typ rzeczywisty pasuje do oczekiwanego.
fn types_compatible(expected: &TypeRef, actual: &TypeRef) -> bool {
    if matches!(expected, TypeRef::Json) || matches!(actual, TypeRef::Json) {
        return true;
    }
    if matches!(expected, TypeRef::Optional(_)) {
        if matches!(actual, TypeRef::Optional(_)) {
            return true;
        }
        return types_compatible(&unwrap_optional(expected), actual);
    }
    if matches!(actual, TypeRef::Optional(_)) {
        return types_compatible(expected, &unwrap_optional(actual));
    }
    match (expected, actual) {
        (TypeRef::Text, TypeRef::Text) => true,
        (TypeRef::Number, TypeRef::Number) => true,
        (TypeRef::Duration, TypeRef::Number) => true,
        (TypeRef::Number, TypeRef::Duration) => true,
        (TypeRef::Boolean, TypeRef::Boolean) => true,
        (TypeRef::Date, TypeRef::Date) => true,
        (TypeRef::File, TypeRef::File) => true,
        (TypeRef::Image, TypeRef::Image) => true,
        (TypeRef::Video, TypeRef::Video) => true,
        (TypeRef::Audio, TypeRef::Audio) => true,
        (TypeRef::Location, TypeRef::Location) => true,
        (TypeRef::List(a), TypeRef::List(b)) => types_compatible(a, b),
        (TypeRef::Map(_, av), TypeRef::Map(_, bv)) => types_compatible(av, bv),
        (TypeRef::Named(a), TypeRef::Named(b)) => a == b,
        // enum przypisany do Text i odwrotnie jest dopuszczalny
        (TypeRef::Text, TypeRef::Named(_)) => true,
        (TypeRef::Named(_), TypeRef::Text) => true,
        _ => false,
    }
}

/// Dzieli liste argumentow, uwzgledniajac nawiasy i cudzyslowy.
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
                out.push(current.trim().to_string());
                current.clear();
            }
            _ => current.push(ch),
        }
    }
    if !current.trim().is_empty() {
        out.push(current.trim().to_string());
    }
    out
}

/// Funkcje wbudowane jezyka wraz z przestrzenia nazw AI.
fn is_builtin(name: &str) -> bool {
    matches!(
        name,
        "newId"
            | "now"
            | "round"
            | "floor"
            | "ceil"
            | "abs"
            | "min"
            | "max"
            | "length"
            | "count"
            | "sum"
            | "average"
            | "join"
            | "split"
            | "replace"
            | "trim"
            | "upper"
            | "lower"
            | "contains"
            | "startsWith"
            | "endsWith"
            | "substring"
            | "format"
            | "parseNumber"
            | "parseDate"
            | "encodeMorse"
            | "decodeMorse"
            | "vibrate"
            | "speak"
            | "listen"
            | "encryptForPeer"
            | "decryptFromPeer"
            | "hash"
            | "verifySafetyNumber"
            | "AI.chat"
            | "AI.generateText"
            | "AI.analyzeImage"
            | "AI.generateImage"
            | "AI.generateMusic"
            | "AI.transcribe"
            | "AI.translate"
            | "AI.summarize"
            | "AI.search"
            | "AI.runAgent"
            | "AI.recognizeSign"
            | "AI.speak"
            | "AI.embed"
            | "AI.captionImage"
            | "AI.describeVideo"
    )
}
