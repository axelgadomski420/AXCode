
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// Pozycja w pliku zrodlowym. Numerowane od jedynki, jak w komunikatach kompilatora.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Span {
    pub line: usize,
    pub column: usize,
    pub end_line: usize,
    pub end_column: usize,
    pub offset: usize,
    pub length: usize,
}

impl Span {
    pub fn new(line: usize, column: usize, offset: usize, length: usize) -> Self {
        Span {
            line,
            column,
            end_line: line,
            end_column: column + length,
            offset,
            length,
        }
    }

    pub fn dummy() -> Self {
        Span {
            line: 0,
            column: 0,
            end_line: 0,
            end_column: 0,
            offset: 0,
            length: 0,
        }
    }

    pub fn merge(self, other: Span) -> Span {
        Span {
            line: self.line,
            column: self.column,
            end_line: other.end_line,
            end_column: other.end_column,
            offset: self.offset,
            length: (other.offset + other.length) - self.offset,
        }
    }

    pub fn is_dummy(&self) -> bool {
        self.line == 0 && self.offset == 0 && self.length == 0
    }
}

/// Typy wbudowane jezyka AXCode.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BuiltinType {
    Text,
    Number,
    Boolean,
    Date,
    Duration,
    Json,
    File,
    Image,
    Video,
    Audio,
    Location,
    Void,
}

impl BuiltinType {
    pub fn parse(name: &str) -> Option<BuiltinType> {
        match name {
            "Text" => Some(BuiltinType::Text),
            "Number" => Some(BuiltinType::Number),
            "Boolean" => Some(BuiltinType::Boolean),
            "Date" => Some(BuiltinType::Date),
            "Duration" => Some(BuiltinType::Duration),
            "Json" => Some(BuiltinType::Json),
            "File" => Some(BuiltinType::File),
            "Image" => Some(BuiltinType::Image),
            "Video" => Some(BuiltinType::Video),
            "Audio" => Some(BuiltinType::Audio),
            "Location" => Some(BuiltinType::Location),
            "Void" => Some(BuiltinType::Void),
            _ => None,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            BuiltinType::Text => "Text",
            BuiltinType::Number => "Number",
            BuiltinType::Boolean => "Boolean",
            BuiltinType::Date => "Date",
            BuiltinType::Duration => "Duration",
            BuiltinType::Json => "Json",
            BuiltinType::File => "File",
            BuiltinType::Image => "Image",
            BuiltinType::Video => "Video",
            BuiltinType::Audio => "Audio",
            BuiltinType::Location => "Location",
            BuiltinType::Void => "Void",
        }
    }
}

/// Typ w jezyku: wbudowany, nazwany, lista, mapa albo opcjonalny.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Type {
    Builtin(BuiltinType),
    Named(String),
    List(Box<Type>),
    Map(Box<Type>, Box<Type>),
    Optional(Box<Type>),
    Unknown,
}

impl Type {
    pub fn display(&self) -> String {
        match self {
            Type::Builtin(b) => b.name().to_string(),
            Type::Named(n) => n.clone(),
            Type::List(inner) => format!("List<{}>", inner.display()),
            Type::Map(k, v) => format!("Map<{}, {}>", k.display(), v.display()),
            Type::Optional(inner) => format!("{}?", inner.display()),
            Type::Unknown => "nieznany".to_string(),
        }
    }

    pub fn is_optional(&self) -> bool {
        matches!(self, Type::Optional(_))
    }

    pub fn inner(&self) -> &Type {
        match self {
            Type::Optional(inner) => inner,
            other => other,
        }
    }
}

/// Model danych: zestaw pol z typami.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataModel {
    pub name: String,
    pub fields: Vec<Field>,
    pub span: Span,
    pub doc: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Field {
    pub name: String,
    pub ty: Type,
    pub span: Span,
    pub default: Option<Expression>,
}

/// Typ wyliczeniowy: zamkniety zbior wartosci.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumDecl {
    pub name: String,
    pub variants: Vec<String>,
    pub span: Span,
    pub doc: Option<String>,
}

/// Deklaracja aplikacji.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppDecl {
    pub name: String,
    pub version: Option<String>,
    pub language: Option<String>,
    pub supports: Vec<String>,
    pub orientation: Option<String>,
    pub offline: Option<String>,
    pub entry: Option<String>,
    pub min_os: Option<String>,
}

/// Klasa przechowywania magazynu.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StoreClass {
    Local,
    Synced,
    Encrypted,
    Secure,
    Cache,
    Queue,
}

impl StoreClass {
    pub fn parse(name: &str) -> Option<StoreClass> {
        match name {
            "local" => Some(StoreClass::Local),
            "synced" => Some(StoreClass::Synced),
            "encrypted" => Some(StoreClass::Encrypted),
            "secure" => Some(StoreClass::Secure),
            "cache" => Some(StoreClass::Cache),
            "queue" => Some(StoreClass::Queue),
            _ => None,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            StoreClass::Local => "local",
            StoreClass::Synced => "synced",
            StoreClass::Encrypted => "encrypted",
            StoreClass::Secure => "secure",
            StoreClass::Cache => "cache",
            StoreClass::Queue => "queue",
        }
    }

    pub fn is_protected(&self) -> bool {
        matches!(self, StoreClass::Secure | StoreClass::Encrypted)
    }
}

/// Deklaracja magazynu z klasa przechowywania.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreDecl {
    pub name: String,
    pub classes: Vec<StoreClass>,
    pub content: StoreContent,
    pub key: Option<String>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StoreContent {
    Single(Type),
    List(Type),
}

/// Deklaracja stanu globalnego.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalDecl {
    pub name: String,
    pub ty: Type,
    pub initial: Option<Expression>,
    pub span: Span,
}

/// Deklaracja uprawnienia.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionDecl {
    pub name: String,
    pub purpose: Option<String>,
    pub required: bool,
    pub precision: Option<String>,
    pub duration: Option<String>,
    pub span: Span,
}

/// Deklaracja warstwy API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiDecl {
    pub name: String,
    pub base: Option<String>,
    pub auth: Option<String>,
    pub timeout: Option<u32>,
    pub retry: Option<u32>,
    pub endpoints: Vec<EndpointDecl>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointDecl {
    pub name: String,
    pub method: HttpMethod,
    pub path: String,
    pub params: Vec<Param>,
    pub returns: Type,
    pub body: Option<Expression>,
    pub multipart: Option<String>,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
}

impl HttpMethod {
    pub fn as_str(&self) -> &'static str {
        match self {
            HttpMethod::Get => "GET",
            HttpMethod::Post => "POST",
            HttpMethod::Put => "PUT",
            HttpMethod::Patch => "PATCH",
            HttpMethod::Delete => "DELETE",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Param {
    pub name: String,
    pub ty: Type,
    pub span: Span,
}

/// Deklaracja funkcji.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDecl {
    pub name: String,
    pub params: Vec<Param>,
    pub returns: Type,
    pub body: Vec<Statement>,
    pub span: Span,
    pub doc: Option<String>,
}

/// Deklaracja komponentu interfejsu.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentDecl {
    pub name: String,
    pub params: Vec<Param>,
    pub body: Vec<UiNode>,
    pub span: Span,
    pub doc: Option<String>,
}

/// Deklaracja ekranu.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenDecl {
    pub name: String,
    pub params: Vec<Param>,
    pub title: Option<String>,
    pub state: Vec<StateDecl>,
    pub events: Vec<EventBlock>,
    pub body: Vec<UiNode>,
    pub span: Span,
    pub doc: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateDecl {
    pub name: String,
    pub ty: Type,
    pub initial: Option<Expression>,
    pub derived: Option<DerivedExpr>,
    pub span: Span,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DerivedExpr {
    pub source: String,
    pub filter: Option<String>,
    pub operation: String,
    pub span: Span,
}

/// Deklaracja stylu.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleDecl {
    pub name: String,
    pub properties: IndexMap<String, String>,
    pub span: Span,
}

/// Deklaracja motywu.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeDecl {
    pub name: String,
    pub follows_system: bool,
    pub colors: IndexMap<String, String>,
    pub radii: IndexMap<String, String>,
    pub fonts: IndexMap<String, String>,
    pub shadows: IndexMap<String, String>,
    pub span: Span,
}

/// Test zadeklarowany w pliku.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestDecl {
    pub kind: TestKind,
    pub name: String,
    pub steps: Vec<TestStep>,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TestKind {
    Unit,
    Ui,
    Accessibility,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestStep {
    pub keyword: String,
    pub value: String,
    pub span: Span,
}

/// Wezel interfejsu.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UiNode {
    Container {
        kind: ContainerKind,
        source: Option<String>,
        accessories: Vec<Accessory>,
        children: Vec<UiNode>,
        span: Span,
    },
    Text {
        value: Expression,
        accessories: Vec<Accessory>,
        span: Span,
    },
    Button {
        label: Option<String>,
        icon: Option<String>,
        accessories: Vec<Accessory>,
        events: Vec<EventBlock>,
        span: Span,
    },
    Field {
        name: String,
        label: Option<String>,
        input_type: Option<String>,
        placeholder: Option<String>,
        required: bool,
        min_length: Option<usize>,
        max_length: Option<usize>,
        error_text: Option<String>,
        span: Span,
    },
    Image {
        reference: Expression,
        alt: Option<String>,
        accessories: Vec<Accessory>,
        span: Span,
    },
    Checkbox {
        reference: String,
        accessibility_label: Option<String>,
        events: Vec<EventBlock>,
        span: Span,
    },
    Media {
        kind: MediaKind,
        reference: Option<Expression>,
        controls: bool,
        subtitles: bool,
        accessibility_label: Option<String>,
        span: Span,
    },
    ComponentCall {
        name: String,
        args: Vec<Expression>,
        span: Span,
    },
    Conditional {
        condition: Expression,
        then_branch: Vec<UiNode>,
        else_branch: Vec<UiNode>,
        span: Span,
    },
    Loop {
        variable: String,
        source: Expression,
        body: Vec<UiNode>,
        span: Span,
    },
    Spacer {
        size: Option<f64>,
        span: Span,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContainerKind {
    Column,
    Row,
    Grid,
    Card,
    Scroll,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MediaKind {
    AudioPlayer,
    VideoPlayer,
    CameraPreview,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Accessory {
    Style(String),
    Color(String),
    Gap(f64),
    Padding(Vec<f64>),
    Margin(Vec<f64>),
    Align(String),
    Justify(String),
    Width(String),
    Height(String),
    Columns(usize),
    Direction(String),
    MaxLines(usize),
    Overflow(String),
    AccessibilityLabel(String),
    Role(String),
    Disabled(Expression),
    Controls(bool),
    Subtitles(bool),
    Facing(String),
    AccessibilityState(String),
    Animation(AnimationSpec),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimationSpec {
    pub kind: String,
    pub duration_ms: u64,
    pub curve: Option<String>,
    pub properties: IndexMap<String, String>,
}

/// Blok zdarzenia.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventBlock {
    pub kind: EventKind,
    pub target: Option<String>,
    pub body: Vec<Statement>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventKind {
    Tapped,
    Changed,
    ScreenAppears,
    ScreenDisappears,
    AppLaunches,
    Valid,
    Invalid,
    Error,
    Online,
    Offline,
    PermissionGranted,
    PermissionDenied,
    IsOnlineChanged,
}

impl EventKind {
    pub fn parse(text: &str) -> Option<EventKind> {
        let t = text.trim();
        match t {
            "tapped" => Some(EventKind::Tapped),
            "changed value" => Some(EventKind::Changed),
            "screen appears" => Some(EventKind::ScreenAppears),
            "screen disappears" => Some(EventKind::ScreenDisappears),
            "app launches" => Some(EventKind::AppLaunches),
            "valid" => Some(EventKind::Valid),
            "invalid" => Some(EventKind::Invalid),
            "error" => Some(EventKind::Error),
            "online" => Some(EventKind::Online),
            "offline" => Some(EventKind::Offline),
            "permission granted" => Some(EventKind::PermissionGranted),
            "permission denied" => Some(EventKind::PermissionDenied),
            _ => {
                if t.starts_with("isOnline changes to") {
                    Some(EventKind::IsOnlineChanged)
                } else {
                    None
                }
            }
        }
    }
}

/// Instrukcja w logice.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Statement {
    Set {
        target: LValue,
        value: Expression,
        span: Span,
    },
    Add {
        value: Expression,
        target: String,
        span: Span,
    },
    Remove {
        value: Expression,
        target: String,
        span: Span,
    },
    Update {
        target: LValue,
        value: Expression,
        span: Span,
    },
    Save {
        store: String,
        span: Span,
    },
    Load {
        store: String,
        span: Span,
    },
    Clear {
        store: String,
        span: Span,
    },
    Enqueue {
        value: Expression,
        target: String,
        span: Span,
    },
    Flush {
        target: String,
        span: Span,
    },
    Return {
        value: Option<Expression>,
        span: Span,
    },
    If {
        condition: Expression,
        then_branch: Vec<Statement>,
        else_branch: Vec<Statement>,
        span: Span,
    },
    For {
        variable: String,
        source: Expression,
        body: Vec<Statement>,
        span: Span,
    },
    TryCatch {
        try_body: Vec<Statement>,
        catch_var: Option<String>,
        catch_body: Vec<Statement>,
        finally_body: Vec<Statement>,
        span: Span,
    },
    Expression {
        value: Expression,
        span: Span,
    },
    Navigate {
        screen: String,
        args: Vec<Expression>,
        span: Span,
    },
    GoBack {
        span: Span,
    },
    Announce {
        value: Expression,
        urgent: bool,
        span: Span,
    },
    Vibrate {
        pattern: String,
        span: Span,
    },
    Raise {
        value: Expression,
        span: Span,
    },
}

/// Miejsce docelowe przypisania.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LValue {
    Variable(String),
    Field {
        object: String,
        field: String,
        index: Option<usize>,
    },
    Indexed {
        name: String,
        index: Expression,
    },
}

impl LValue {
    pub fn root_name(&self) -> &str {
        match self {
            LValue::Variable(n) => n,
            LValue::Field { object, .. } => object,
            LValue::Indexed { name, .. } => name,
        }
    }
}

/// Wyrazenie.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Expression {
    Text(String),
    Number(f64),
    Boolean(bool),
    None,
    Identifier(String),
    Path {
        root: String,
        fields: Vec<String>,
    },
    EnumValue {
        enum_name: String,
        variant: String,
    },
    Concatenate(Vec<Expression>),
    Compare {
        left: Box<Expression>,
        op: CompareOp,
        right: Box<Expression>,
    },
    Call {
        name: String,
        args: Vec<Expression>,
    },
    Constructor {
        type_name: String,
        fields: IndexMap<String, Expression>,
    },
    List(Vec<Expression>),
    Object(IndexMap<String, Expression>),
    Length(Box<Expression>),
    Negate(Box<Expression>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompareOp {
    Equal,
    NotEqual,
    Greater,
    Less,
    GreaterEqual,
    LessEqual,
}

/// Calosc pliku zrodlowego.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Program {
    pub app: AppDecl,
    pub themes: Vec<ThemeDecl>,
    pub styles: Vec<StyleDecl>,
    pub enums: Vec<EnumDecl>,
    pub data: Vec<DataModel>,
    pub stores: Vec<StoreDecl>,
    pub globals: Vec<GlobalDecl>,
    pub permissions: Vec<PermissionDecl>,
    pub apis: Vec<ApiDecl>,
    pub functions: Vec<FunctionDecl>,
    pub components: Vec<ComponentDecl>,
    pub screens: Vec<ScreenDecl>,
    pub tests: Vec<TestDecl>,
}

impl Program {
    pub fn screen(&self, name: &str) -> Option<&ScreenDecl> {
        self.screens.iter().find(|s| s.name == name)
    }

    pub fn component(&self, name: &str) -> Option<&ComponentDecl> {
        self.components.iter().find(|c| c.name == name)
    }

    pub fn function(&self, name: &str) -> Option<&FunctionDecl> {
        self.functions.iter().find(|f| f.name == name)
    }

    pub fn data_model(&self, name: &str) -> Option<&DataModel> {
        self.data.iter().find(|d| d.name == name)
    }

    pub fn enum_decl(&self, name: &str) -> Option<&EnumDecl> {
        self.enums.iter().find(|e| e.name == name)
    }

    pub fn store(&self, name: &str) -> Option<&StoreDecl> {
        self.stores.iter().find(|s| s.name == name)
    }

    pub fn all_type_names(&self) -> Vec<String> {
        let mut names = Vec::new();
        names.extend(self.data.iter().map(|d| d.name.clone()));
        names.extend(self.enums.iter().map(|e| e.name.clone()));
        names
    }

    pub fn all_screen_names(&self) -> Vec<&str> {
        self.screens.iter().map(|s| s.name.as_str()).collect()
    }

    pub fn all_component_names(&self) -> Vec<&str> {
        self.components.iter().map(|c| c.name.as_str()).collect()
    }

    pub fn all_style_names(&self) -> Vec<&str> {
        self.styles.iter().map(|s| s.name.as_str()).collect()
    }

    pub fn all_store_names(&self) -> Vec<&str> {
        self.stores.iter().map(|s| s.name.as_str()).collect()
    }
}
