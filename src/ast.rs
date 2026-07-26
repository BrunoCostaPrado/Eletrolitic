use crate::token::Span;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Program {
  pub body: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Parameter {
  pub name: String,
  pub type_ann: Option<TypeAnn>,
  pub default_value: Option<Box<Expression>>,
  pub is_rest: bool,
  pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImportSpecifier {
  pub local: String,
  pub imported: Option<String>,
  pub span: Span,
  pub is_default: bool,
  pub is_namespace: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ArrowFunctionBody {
  Expression(Box<Expression>),
  Block(Vec<Statement>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Statement {
  ExpressionStatement {
    expression: Box<Expression>,
    span: Span,
  },
  VariableDeclaration {
    kind: VariableKind,
    declarations: Vec<VariableDeclarator>,
    span: Span,
  },
  BlockStatement {
    body: Vec<Statement>,
    span: Span,
  },
  IfStatement {
    test: Box<Expression>,
    consequent: Box<Statement>,
    alternate: Option<Box<Statement>>,
    span: Span,
  },
  WhileStatement {
    test: Box<Expression>,
    body: Box<Statement>,
    span: Span,
  },
  ForStatement {
    init: Option<ForInit>,
    test: Option<Box<Expression>>,
    update: Option<Box<Expression>>,
    body: Box<Statement>,
    span: Span,
  },
  ForInOfStatement {
    kind: VariableKind,
    left: Expression,
    right: Box<Expression>,
    body: Box<Statement>,
    is_of: bool,
    is_await: bool,
    span: Span,
  },
  ReturnStatement {
    value: Option<Box<Expression>>,
    span: Span,
  },
  FunctionDeclaration {
    name: String,
    params: Vec<Parameter>,
    return_type: Option<TypeAnn>,
    body: Box<Statement>,
    is_async: bool,
    type_params: Vec<(String, Option<TypeAnn>)>,
    span: Span,
  },
  ImportDeclaration {
    specifiers: Vec<ImportSpecifier>,
    source: String,
    is_type: bool,
    span: Span,
  },
  TypeAliasDeclaration {
    name: String,
    type_params: Vec<(String, Option<TypeAnn>)>,
    type_annotation: TypeAnn,
    span: Span,
  },
  InterfaceDeclaration {
    name: String,
    span: Span,
  },
  ExportDeclaration {
    declaration: Box<Statement>,
    is_default: bool,
    span: Span,
  },
  SwitchStatement {
    discriminant: Box<Expression>,
    cases: Vec<SwitchCase>,
    span: Span,
  },
  ThrowStatement {
    argument: Box<Expression>,
    span: Span,
  },
  TryStatement {
    body: Box<Statement>,
    handler: Option<CatchClause>,
    finalizer: Option<Vec<Statement>>,
    span: Span,
  },
  BreakStatement {
    label: Option<String>,
    span: Span,
  },
  ContinueStatement {
    label: Option<String>,
    span: Span,
  },
  LabeledStatement {
    label: String,
    body: Box<Statement>,
    span: Span,
  },
  DoWhileStatement {
    test: Box<Expression>,
    body: Box<Statement>,
    span: Span,
  },
  ClassDeclaration {
    name: String,
    superclass: Option<Box<Expression>>,
    body: ClassBody,
    span: Span,
  },
  EnumDeclaration {
    name: String,
    members: Vec<EnumMember>,
    span: Span,
  },
  DeclareModule {
    name: String,
    body: Vec<Statement>,
    span: Span,
  },
  DeclareNamespace {
    name: String,
    body: Vec<Statement>,
    span: Span,
  },
  DecoratedStatement {
    decorators: Vec<Expression>,
    statement: Box<Statement>,
    span: Span,
  },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ForInit {
  Expression(Box<Expression>),
  VariableDeclaration { kind: VariableKind, declarations: Vec<VariableDeclarator> },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SwitchCase {
  pub test: Option<Box<Expression>>,
  pub body: Vec<Statement>,
  pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CatchClause {
  pub param: Option<String>,
  pub type_ann: Option<TypeAnn>,
  pub body: Vec<Statement>,
  pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClassBody {
  pub methods: Vec<MethodDefinition>,
  pub fields: Vec<ClassField>,
  pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Visibility {
  Public,
  Private,
  Protected,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClassField {
  pub key: PropertyKey,
  pub type_ann: Option<TypeAnn>,
  pub init: Option<Box<Expression>>,
  pub is_static: bool,
  pub visibility: Option<Visibility>,
  pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MethodDefinition {
  pub key: PropertyKey,
  pub kind: MethodKind,
  pub params: Vec<Parameter>,
  pub return_type: Option<TypeAnn>,
  pub body: Box<Statement>,
  pub is_static: bool,
  pub visibility: Option<Visibility>,
  pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum MethodKind {
  Constructor,
  Method,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnumMember {
  pub name: String,
  pub value: Option<String>,
  pub span: Span,
}

impl Statement {
  #[must_use]
  pub fn span(&self) -> Span {
    match self {
      Statement::ExpressionStatement { span, .. }
      | Statement::VariableDeclaration { span, .. }
      | Statement::BlockStatement { span, .. }
      | Statement::IfStatement { span, .. }
      | Statement::WhileStatement { span, .. }
      | Statement::ForStatement { span, .. }
      | Statement::ForInOfStatement { span, .. }
      | Statement::ReturnStatement { span, .. }
      | Statement::FunctionDeclaration { span, .. }
      | Statement::ImportDeclaration { span, .. }
      | Statement::TypeAliasDeclaration { span, .. }
      | Statement::InterfaceDeclaration { span, .. }
      | Statement::ExportDeclaration { span, .. }
      | Statement::SwitchStatement { span, .. }
      | Statement::ThrowStatement { span, .. }
      | Statement::TryStatement { span, .. }
      | Statement::BreakStatement { span, .. }
      | Statement::ContinueStatement { span, .. }
      | Statement::LabeledStatement { span, .. }
      | Statement::DoWhileStatement { span, .. }
      | Statement::ClassDeclaration { span, .. }
      | Statement::EnumDeclaration { span, .. }
      | Statement::DeclareModule { span, .. }
      | Statement::DeclareNamespace { span, .. } => *span,
      Statement::DecoratedStatement { span, .. } => *span,
    }
  }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum VariableKind {
  Let,
  Const,
  Var,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LiteralValue {
  String(String),
  Number(f64),
  Boolean(bool),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TypeAnn {
  Number,
  String,
  Boolean,
  Null,
  Undefined,
  Void,
  Any,
  Unknown,
  Never,
  Literal {
    value: LiteralValue,
  },
  Union {
    types: Vec<TypeAnn>,
  },
  Array {
    element: Box<TypeAnn>,
  },
  TypeRef {
    name: String,
    type_args: Vec<TypeAnn>,
  },
  Typeof {
    argument: Box<Expression>,
  },
  Function {
    params: Vec<TypeAnn>,
    return_type: Box<TypeAnn>,
  },
  Object {
    properties: Vec<ObjectTypeProperty>,
  },
  Intersection {
    types: Vec<TypeAnn>,
  },
  TemplateLiteral {
    quasis: Vec<String>,
    types: Vec<TypeAnn>,
  },
  KeyOf {
    type_ann: Box<TypeAnn>,
  },
  Conditional {
    check: Box<TypeAnn>,
    extends: Box<TypeAnn>,
    true_type: Box<TypeAnn>,
    false_type: Box<TypeAnn>,
  },
  Infer {
    name: String,
  },
  Mapped {
    key: String,
    target: Box<TypeAnn>,
    value: Box<TypeAnn>,
  },
  IndexedAccess {
    target: Box<TypeAnn>,
    index: Box<TypeAnn>,
  },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ObjectTypeProperty {
  pub name: String,
  pub type_ann: TypeAnn,
  pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VariableDeclarator {
  pub id: Box<Expression>,
  pub type_ann: Option<TypeAnn>,
  pub init: Option<Box<Expression>>,
  pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Expression {
  NumberLiteral {
    value: f64,
    span: Span,
  },
  StringLiteral {
    value: String,
    span: Span,
  },
  BooleanLiteral {
    value: bool,
    span: Span,
  },
  NullLiteral {
    span: Span,
  },
  UndefinedLiteral {
    span: Span,
  },
  Identifier {
    name: String,
    span: Span,
  },
  ArrayExpression {
    elements: Vec<Option<Box<Expression>>>,
    span: Span,
  },
  ObjectExpression {
    properties: Vec<Property>,
    span: Span,
  },
  BinaryExpression {
    left: Box<Expression>,
    operator: BinaryOp,
    right: Box<Expression>,
    span: Span,
  },
  UnaryExpression {
    operator: UnaryOp,
    operand: Box<Expression>,
    span: Span,
  },
  ConditionalExpression {
    test: Box<Expression>,
    consequent: Box<Expression>,
    alternate: Box<Expression>,
    span: Span,
  },
  CallExpression {
    callee: Box<Expression>,
    arguments: Vec<Box<Expression>>,
    optional: bool,
    span: Span,
  },
  MemberExpression {
    object: Box<Expression>,
    property: Box<Expression>,
    computed: bool,
    optional: bool,
    span: Span,
  },
  AssignmentExpression {
    left: Box<Expression>,
    operator: AssignmentOp,
    right: Box<Expression>,
    span: Span,
  },
  ParenthesizedExpression {
    expression: Box<Expression>,
    span: Span,
  },
  Placeholder {
    span: Span,
  },
  ArrowFunction {
    params: Vec<Parameter>,
    return_type: Option<TypeAnn>,
    body: ArrowFunctionBody,
    is_async: bool,
    span: Span,
  },
  TemplateLiteral {
    quasis: Vec<String>,
    expressions: Vec<Box<Expression>>,
    span: Span,
  },
  NewExpression {
    callee: Box<Expression>,
    arguments: Vec<Box<Expression>>,
    span: Span,
  },
  SpreadElement {
    argument: Box<Expression>,
    span: Span,
  },
  ThisExpression {
    span: Span,
  },
  SuperExpression {
    span: Span,
  },
  ObjectPattern {
    properties: Vec<ObjectPatternProperty>,
    span: Span,
  },
  ArrayPattern {
    elements: Vec<Option<Box<Expression>>>,
    span: Span,
  },
  AwaitExpression {
    argument: Box<Expression>,
    span: Span,
  },
  AsExpression {
    expression: Box<Expression>,
    type_ann: TypeAnn,
    span: Span,
  },
  TaggedTemplateLiteral {
    tag: Box<Expression>,
    quasis: Vec<String>,
    expressions: Vec<Box<Expression>>,
    span: Span,
  },
  ImportMeta {
    span: Span,
  },
  ImportCall {
    source: Box<Expression>,
    span: Span,
  },
  SatisfiesExpression {
    expression: Box<Expression>,
    type_ann: TypeAnn,
    span: Span,
  },
  JsxElement {
    tag: Box<Expression>,
    attributes: Vec<JsxAttribute>,
    children: Vec<JsxChild>,
    self_closing: bool,
    span: Span,
  },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JsxAttribute {
  pub name: String,
  pub value: JsxAttributeValue,
  pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum JsxAttributeValue {
  Expression(Box<Expression>),
  Text(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum JsxChild {
  Text(String),
  Expression(Box<Expression>),
  Element(Box<Expression>),
}

impl Expression {
  #[must_use]
  pub fn span(&self) -> Span {
    match self {
      Expression::NumberLiteral { span, .. }
      | Expression::StringLiteral { span, .. }
      | Expression::BooleanLiteral { span, .. }
      | Expression::NullLiteral { span, .. }
      | Expression::UndefinedLiteral { span, .. }
      | Expression::Identifier { span, .. }
      | Expression::ArrayExpression { span, .. }
      | Expression::ObjectExpression { span, .. }
      | Expression::BinaryExpression { span, .. }
      | Expression::UnaryExpression { span, .. }
      | Expression::ConditionalExpression { span, .. }
      | Expression::CallExpression { span, .. }
      | Expression::MemberExpression { span, .. }
      | Expression::AssignmentExpression { span, .. }
      | Expression::ParenthesizedExpression { span, .. }
      | Expression::Placeholder { span, .. }
      | Expression::ArrowFunction { span, .. }
      | Expression::TemplateLiteral { span, .. }
      | Expression::NewExpression { span, .. }
      | Expression::SpreadElement { span, .. }
      | Expression::ThisExpression { span, .. }
      | Expression::SuperExpression { span, .. }
      | Expression::ObjectPattern { span, .. }
      | Expression::ArrayPattern { span, .. }
      | Expression::AwaitExpression { span, .. }
      | Expression::AsExpression { span, .. }
      | Expression::TaggedTemplateLiteral { span, .. }
      | Expression::ImportMeta { span }
      | Expression::ImportCall { span, .. }
      | Expression::SatisfiesExpression { span, .. }
      | Expression::JsxElement { span, .. } => *span,
    }
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum BinaryOp {
  NullishCoalescing,
  LogicalOr,
  LogicalAnd,
  BitwiseOr,
  BitwiseXor,
  BitwiseAnd,
  Eq,
  NotEq,
  StrictEq,
  StrictNotEq,
  Lt,
  Gt,
  LtEq,
  GtEq,
  Lsh,
  Rsh,
  ZeroFillRsh,
  Add,
  Sub,
  Mul,
  Div,
  Rem,
  Exp,
  Instanceof,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum UnaryOp {
  Minus,
  Plus,
  Not,
  BitwiseNot,
  PlusPlus,
  MinusMinus,
  TypeOf,
  Void,
  Delete,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum AssignmentOp {
  Assign,
  AddAssign,
  SubAssign,
  MulAssign,
  DivAssign,
  ModAssign,
  BitAndAssign,
  BitOrAssign,
  BitXorAssign,
  NullishAssign,
  AndAssign,
  OrAssign,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Property {
  pub key: PropertyKey,
  pub value: Box<Expression>,
  pub shorthand: bool,
  pub is_spread: bool,
  pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ObjectPatternProperty {
  pub key: PropertyKey,
  pub value: Box<Expression>,
  pub shorthand: bool,
  pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PropertyKey {
  Identifier(String),
  String(String),
  Expression(Box<Expression>),
}
