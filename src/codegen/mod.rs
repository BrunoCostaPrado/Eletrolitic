mod expr;

use crate::ast::{Expression, ForInit, MethodKind, Program, PropertyKey, Statement, VariableKind};
use crate::source_map::SourceMap;
use crate::token::Span;
use std::fmt::Write;

#[derive(Default)]
pub struct Codegen {
  pub output: String,
  pub indent: usize,
  pub cjs: bool,
  gen_line: usize,
  gen_col: usize,
  source_map: *mut SourceMap,
  last_pos: usize,
}

impl Codegen {
  #[must_use]
  pub fn new() -> Self {
    Self {
      output: String::new(),
      indent: 0,
      cjs: false,
      gen_line: 0,
      gen_col: 0,
      source_map: std::ptr::null_mut(),
      last_pos: 0,
    }
  }

  /// Attach a source map for tracking position mappings.
  /// # Safety
  /// Caller must ensure the `SourceMap` outlives this Codegen.
  pub unsafe fn set_source_map(&mut self, sm: *mut SourceMap) {
    self.source_map = sm;
  }

  /// Incrementally update `gen_line/gen_col` from last known position.
  fn update_position(&mut self) {
    // ponytail: O(delta) scan from last_pos, not O(n) full rescan
    let bytes = self.output.as_bytes();
    let mut i = self.last_pos;
    while i < bytes.len() {
      if bytes[i] == b'\n' {
        self.gen_line += 1;
        self.gen_col = 0;
      } else {
        self.gen_col += 1;
      }
      i += 1;
    }
    self.last_pos = bytes.len();
  }

  /// Record a source mapping at current generated position.
  fn record_mapping(&mut self, span: Span) {
    if !self.source_map.is_null() {
      self.update_position();
      // Safety: caller guarantees source_map outlives this Codegen
      unsafe { (*self.source_map).add_mapping(self.gen_line, self.gen_col, span) };
    }
  }

  pub fn generate(&mut self, program: &Program) -> &str {
    for stmt in &program.body {
      self.gen_statement(stmt);
    }
    &self.output
  }

  fn gen_statement(&mut self, stmt: &Statement) {
    self.record_mapping(stmt.span());
    match stmt {
      Statement::ExpressionStatement { expression, .. } => {
        expr::gen_expression(self, expression);
        let _ = writeln!(self.output, ";");
      }
      Statement::VariableDeclaration { kind, declarations, .. } => {
        let kw = match kind {
          VariableKind::Let => "let",
          VariableKind::Const => "const",
          VariableKind::Var => "var",
        };
        let _ = write!(self.output, "{kw} ");
        for (i, decl) in declarations.iter().enumerate() {
          if i > 0 {
            let _ = write!(self.output, ", ");
          }
          expr::gen_expression(self, &decl.id);
          if let Some(init) = &decl.init {
            let _ = write!(self.output, " = ");
            expr::gen_expression(self, init);
          }
        }
        let _ = writeln!(self.output, ";");
      }
      Statement::BlockStatement { body, .. } => {
        let _ = writeln!(self.output, "{{");
        self.indent += 1;
        for stmt in body {
          self.gen_statement(stmt);
        }
        self.indent -= 1;
        self.gen_indent();
        let _ = writeln!(self.output, "}}");
      }
      Statement::IfStatement { test, consequent, alternate, .. } => {
        self.gen_indent();
        let _ = write!(self.output, "if (");
        expr::gen_expression(self, test);
        let _ = write!(self.output, ") ");
        self.gen_statement(consequent);
        if let Some(alt) = alternate {
          let _ = write!(self.output, "else ");
          self.gen_statement(alt);
        }
      }
      Statement::WhileStatement { test, body, .. } => {
        self.gen_indent();
        let _ = write!(self.output, "while (");
        expr::gen_expression(self, test);
        let _ = write!(self.output, ") ");
        self.gen_statement(body);
      }
      Statement::ForStatement { init, test, update, body, .. } => {
        self.gen_indent();
        let _ = write!(self.output, "for (");
        match init {
          Some(ForInit::Expression(expr)) => {
            expr::gen_expression(self, expr);
          }
          Some(ForInit::VariableDeclaration { kind, declarations }) => {
            let kw = match kind {
              VariableKind::Let => "let",
              VariableKind::Const => "const",
              VariableKind::Var => "var",
            };
            let _ = write!(self.output, "{kw} ");
            for (i, decl) in declarations.iter().enumerate() {
              if i > 0 {
                let _ = write!(self.output, ", ");
              }
              expr::gen_expression(self, &decl.id);
              if let Some(init_expr) = &decl.init {
                let _ = write!(self.output, " = ");
                expr::gen_expression(self, init_expr);
              }
            }
          }
          None => {}
        }
        let _ = write!(self.output, "; ");
        if let Some(t) = test {
          expr::gen_expression(self, t);
        }
        let _ = write!(self.output, "; ");
        if let Some(u) = update {
          expr::gen_expression(self, u);
        }
        let _ = write!(self.output, ") ");
        self.gen_statement(body);
      }
      Statement::ReturnStatement { value, .. } => {
        self.gen_indent();
        let _ = write!(self.output, "return");
        if let Some(v) = value {
          let _ = write!(self.output, " ");
          expr::gen_expression(self, v);
        }
        let _ = writeln!(self.output, ";");
      }
      Statement::FunctionDeclaration { name, params, body, is_async, type_params, .. } => {
        self.gen_indent();
        if *is_async {
          let _ = write!(self.output, "async ");
        }
        let _ = write!(self.output, "function {name}");
        if !type_params.is_empty() {
          let _ = write!(self.output, "<");
          for (i, (name, _)) in type_params.iter().enumerate() {
            if i > 0 {
              let _ = write!(self.output, ", ");
            }
            let _ = write!(self.output, "{name}");
          }
          let _ = write!(self.output, ">");
        }
        let _ = write!(self.output, "(");
        for (i, param) in params.iter().enumerate() {
          if i > 0 {
            let _ = write!(self.output, ", ");
          }
          if param.is_rest {
            let _ = write!(self.output, "...");
          }
          let _ = write!(self.output, "{}", param.name);
          if let Some(default) = &param.default_value {
            let _ = write!(self.output, " = ");
            expr::gen_expression(self, default);
          }
        }
        let _ = write!(self.output, ") ");
        self.gen_statement(body);
      }
      Statement::ImportDeclaration { specifiers, source, is_type, .. } => {
        if *is_type {
          return;
        }
        self.gen_indent();
        let has_namespace = specifiers.iter().any(|s| s.is_namespace);
        let has_default = specifiers.iter().any(|s| s.is_default);
        let named: Vec<_> =
          specifiers.iter().filter(|s| !s.is_default && !s.is_namespace).cloned().collect();
        if self.cjs {
          if has_namespace {
            let local = &specifiers.iter().find(|s| s.is_namespace).unwrap().local;
            let _ = write!(self.output, "const {local} = require(\"{source}\");");
          } else if has_default && named.is_empty() {
            let _ = write!(self.output, "const {} = require(\"{source}\");", specifiers[0].local);
          } else if has_default {
            let default = specifiers.iter().find(|s| s.is_default).unwrap();
            let _ = write!(self.output, "const {} = require(\"{source}\");", default.local);
            if !named.is_empty() {
              let _ = writeln!(self.output);
              self.gen_indent();
              let _ = write!(self.output, "const ");
              self.write_specifiers(&named);
              let _ = write!(self.output, " = require(\"{source}\");");
            }
          } else {
            let _ = write!(self.output, "const ");
            self.write_specifiers(specifiers);
            let _ = write!(self.output, " = require(\"{source}\");");
          }
        } else {
          if has_namespace {
            let local = &specifiers.iter().find(|s| s.is_namespace).unwrap().local;
            let _ = write!(self.output, "import * as {local} from \"{source}\";");
          } else if has_default && named.is_empty() {
            let _ = write!(self.output, "import {} from \"{source}\";", specifiers[0].local);
          } else if has_default {
            let default = specifiers.iter().find(|s| s.is_default).unwrap();
            let _ = write!(self.output, "import {}, ", default.local);
            self.write_specifiers(&named);
            let _ = write!(self.output, " from \"{source}\";");
          } else {
            let _ = write!(self.output, "import ");
            self.write_specifiers(specifiers);
            let _ = write!(self.output, " from \"{source}\";");
          }
        }
        let _ = writeln!(self.output);
      }
      Statement::TypeAliasDeclaration { .. } => {
        // ponytail: type aliases stripped in JS output
      }
      Statement::InterfaceDeclaration { .. } => {
        // ponytail: interfaces stripped in JS output
      }
      Statement::ExportDeclaration { declaration, is_default, .. } => {
        if self.cjs {
          if *is_default {
            if let Statement::ExpressionStatement { expression, .. } = declaration.as_ref() {
              self.gen_indent();
              let _ = write!(self.output, "module.exports = ");
              expr::gen_expression(self, expression);
              let _ = writeln!(self.output, ";");
            } else {
              let name = Self::extract_decl_name(declaration);
              self.gen_statement(declaration);
              if let Some(name) = name {
                self.gen_indent();
                let _ = writeln!(self.output, "module.exports = {name};");
              }
            }
          } else {
            let name = Self::extract_decl_name(declaration);
            self.gen_statement(declaration);
            if let Some(name) = name {
              self.gen_indent();
              let _ = writeln!(self.output, "module.exports.{name} = {name};");
            }
          }
        } else {
          self.gen_indent();
          if *is_default {
            let _ = write!(self.output, "export default ");
          } else {
            let _ = write!(self.output, "export ");
          }
          self.gen_statement(declaration);
        }
      }
      Statement::ForInOfStatement { kind, left, right, body, is_of, is_await, .. } => {
        self.gen_indent();
        let kw = match kind {
          VariableKind::Let => "let",
          VariableKind::Const => "const",
          VariableKind::Var => "var",
        };
        let op = if *is_of { "of" } else { "in" };
        let _ = write!(self.output, "for (");
        if *is_await {
          let _ = write!(self.output, "await ");
        }
        let _ = write!(self.output, "{kw} ");
        expr::gen_expression(self, left);
        let _ = write!(self.output, " {op} ");
        expr::gen_expression(self, right);
        let _ = write!(self.output, ") ");
        self.gen_statement(body);
      }
      Statement::SwitchStatement { discriminant, cases, .. } => {
        self.gen_indent();
        let _ = write!(self.output, "switch (");
        expr::gen_expression(self, discriminant);
        let _ = writeln!(self.output, ") {{");
        self.indent += 1;
        for case in cases {
          self.gen_indent();
          if let Some(test) = &case.test {
            let _ = write!(self.output, "case ");
            expr::gen_expression(self, test);
            let _ = writeln!(self.output, ":");
          } else {
            let _ = writeln!(self.output, "default:");
          }
          self.indent += 1;
          for stmt in &case.body {
            self.gen_statement(stmt);
          }
          self.indent -= 1;
        }
        self.indent -= 1;
        self.gen_indent();
        let _ = writeln!(self.output, "}}");
      }
      Statement::ThrowStatement { argument, .. } => {
        self.gen_indent();
        let _ = write!(self.output, "throw ");
        expr::gen_expression(self, argument);
        let _ = writeln!(self.output, ";");
      }
      Statement::TryStatement { body, handler, finalizer, .. } => {
        self.gen_indent();
        let _ = writeln!(self.output, "try ");
        self.gen_statement(body);
        if let Some(catch) = handler {
          if let Some(ref param) = catch.param {
            if let Some(ta) = &catch.type_ann {
              let _ =
                write!(self.output, " catch ({param}: {})", crate::decl_emit::render_type_ann(ta));
            } else {
              let _ = write!(self.output, " catch ({param})");
            }
          } else {
            // bare catch (ES2019)
          }
          let _ = writeln!(self.output, " {{");
          self.indent += 1;
          for stmt in &catch.body {
            self.gen_statement(stmt);
          }
          self.indent -= 1;
          self.gen_indent();
          let _ = writeln!(self.output, "}}");
        }
        if let Some(finalizer_body) = finalizer {
          let _ = writeln!(self.output, " finally {{");
          self.indent += 1;
          for stmt in finalizer_body {
            self.gen_statement(stmt);
          }
          self.indent -= 1;
          self.gen_indent();
          let _ = writeln!(self.output, "}}");
        }
      }
      Statement::BreakStatement { label, .. } => {
        self.gen_indent();
        if let Some(label) = label {
          let _ = writeln!(self.output, "break {label};");
        } else {
          let _ = writeln!(self.output, "break;");
        }
      }
      Statement::ContinueStatement { label, .. } => {
        self.gen_indent();
        if let Some(label) = label {
          let _ = writeln!(self.output, "continue {label};");
        } else {
          let _ = writeln!(self.output, "continue;");
        }
      }
      Statement::LabeledStatement { label, body, .. } => {
        self.gen_indent();
        let _ = write!(self.output, "{label}: ");
        self.gen_statement(body);
      }
      Statement::DoWhileStatement { test, body, .. } => {
        self.gen_indent();
        let _ = write!(self.output, "do ");
        self.gen_statement(body);
        let _ = write!(self.output, "while (");
        expr::gen_expression(self, test);
        let _ = writeln!(self.output, ");");
      }
      Statement::ClassDeclaration { name, superclass, body, .. } => {
        self.gen_indent();
        let _ = write!(self.output, "class {name}");
        if let Some(superclass) = superclass {
          let _ = write!(self.output, " extends ");
          expr::gen_expression(self, superclass);
        }
        let _ = writeln!(self.output, " {{");
        self.indent += 1;
        // Fields first
        for field in &body.fields {
          self.gen_indent();
          if field.is_static {
            let _ = write!(self.output, "static ");
          }
          match &field.key {
            PropertyKey::Identifier(n) => {
              let _ = write!(self.output, "{n}");
            }
            PropertyKey::String(n) => {
              let _ = write!(self.output, "\"{n}\"");
            }
            PropertyKey::Expression(e) => {
              let _ = write!(self.output, "[");
              expr::gen_expression(self, e);
              let _ = write!(self.output, "]");
            }
          }
          if let Some(init) = &field.init {
            let _ = write!(self.output, " = ");
            expr::gen_expression(self, init);
          }
          let _ = writeln!(self.output, ";");
        }
        // Methods
        for method in &body.methods {
          self.gen_indent();
          if method.is_static {
            let _ = write!(self.output, "static ");
          }
          match &method.key {
            PropertyKey::Identifier(n) if method.kind == MethodKind::Constructor => {
              let _ = write!(self.output, "constructor(");
            }
            PropertyKey::Identifier(n) => {
              let _ = write!(self.output, "{n}(");
            }
            PropertyKey::String(n) => {
              let _ = write!(self.output, "\"{n}\"(");
            }
            PropertyKey::Expression(e) => {
              let _ = write!(self.output, "[");
              expr::gen_expression(self, e);
              let _ = write!(self.output, "](");
            }
          }
          for (i, param) in method.params.iter().enumerate() {
            if i > 0 {
              let _ = write!(self.output, ", ");
            }
            if param.is_rest {
              let _ = write!(self.output, "...");
            }
            let _ = write!(self.output, "{}", param.name);
            if let Some(default) = &param.default_value {
              let _ = write!(self.output, " = ");
              expr::gen_expression(self, default);
            }
          }
          let _ = write!(self.output, ") ");
          self.gen_statement(&method.body);
        }
        self.indent -= 1;
        self.gen_indent();
        let _ = writeln!(self.output, "}}");
      }
      Statement::EnumDeclaration { name, members, .. } => {
        self.gen_indent();
        let mut auto_val: i64 = 0;
        let _ = write!(self.output, "const {name} = {{");
        for (i, member) in members.iter().enumerate() {
          if i > 0 {
            let _ = write!(self.output, ", ");
          }
          match &member.value {
            Some(v) if v.parse::<i64>().is_ok() => {
              let n: i64 = v.parse().unwrap();
              auto_val = n;
              let _ = write!(self.output, "{}: {n}", member.name);
              auto_val += 1;
            }
            Some(v) => {
              let _ = write!(self.output, "{}: \"{v}\"", member.name);
            }
            None => {
              let _ = write!(self.output, "{}: {auto_val}", member.name);
              auto_val += 1;
            }
          }
        }
        let _ = writeln!(self.output, "}};");
      }
      Statement::DeclareModule { body, .. } | Statement::DeclareNamespace { body, .. } => {
        for stmt in body {
          self.gen_statement(stmt);
        }
      }
      Statement::DecoratedStatement { statement, .. } => {
        self.gen_statement(statement);
      }
    }
  }

  /// Write import specifiers as "{ a as b, c }" — used by both CJS and ESM import codegen.
  fn write_specifiers(&mut self, specs: &[crate::ast::ImportSpecifier]) {
    let _ = write!(self.output, "{{ ");
    for (i, s) in specs.iter().enumerate() {
      if i > 0 {
        let _ = write!(self.output, ", ");
      }
      if let Some(ref imported) = s.imported {
        let _ = write!(self.output, "{imported} as {}", s.local);
      } else {
        let _ = write!(self.output, "{}", s.local);
      }
    }
    let _ = write!(self.output, " }}");
  }

  /// Extract the name from a declaration statement for CJS exports.
  fn extract_decl_name(decl: &Statement) -> Option<String> {
    match decl {
      Statement::FunctionDeclaration { name, .. } if !name.is_empty() => Some(name.clone()),
      Statement::ClassDeclaration { name, .. } => Some(name.clone()),
      Statement::VariableDeclaration { declarations, .. } => declarations.iter().find_map(|d| {
        if let Expression::Identifier { name, .. } = d.id.as_ref() {
          Some(name.clone())
        } else {
          None
        }
      }),
      Statement::DecoratedStatement { statement, .. } => Self::extract_decl_name(statement),
      _ => None,
    }
  }

  fn gen_indent(&mut self) {
    for _ in 0..self.indent {
      let _ = write!(self.output, "  ");
    }
  }
}

#[must_use]
pub fn needs_parens(outer: &Expression, inner: &Expression) -> bool {
  let outer_prec = match outer {
    Expression::BinaryExpression { operator, .. } => expr::binary_precedence(*operator),
    Expression::UnaryExpression { .. } => 15,
    Expression::CallExpression { .. } => 17,
    Expression::MemberExpression { .. } => 18,
    Expression::ConditionalExpression { .. } => 1,
    Expression::AssignmentExpression { .. } => 1,
    _ => return false,
  };
  let inner_prec = match inner {
    Expression::BinaryExpression { operator, .. } => expr::binary_precedence(*operator),
    Expression::UnaryExpression { .. } => 15,
    Expression::ConditionalExpression { .. } => 1,
    Expression::AssignmentExpression { .. } => 1,
    _ => return false,
  };
  inner_prec < outer_prec
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::diagnostic::SourceFile;
  use crate::lexer::Lexer;
  use crate::parser::Parser;

  fn run(source: &str) -> String {
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().to_vec();
    let mut parser = Parser::new(tokens, SourceFile::new("test.ts", source));
    let program = parser.parse();
    let mut cg = Codegen::new();
    cg.generate(&program).to_string()
  }

  #[test]
  fn gen_number() {
    assert_eq!(run("42;"), "42;\n");
  }

  #[test]
  fn gen_string() {
    assert_eq!(run("\"hello\";"), "\"hello\";\n");
  }

  #[test]
  fn gen_boolean() {
    assert_eq!(run("true;"), "true;\n");
    assert_eq!(run("false;"), "false;\n");
  }

  #[test]
  fn gen_identifier() {
    assert_eq!(run("foo;"), "foo;\n");
  }

  #[test]
  fn gen_binary_add() {
    assert_eq!(run("1 + 2;"), "1 + 2;\n");
  }

  #[test]
  fn gen_precedence_parens() {
    assert_eq!(run("(1 + 2) * 3;"), "(1 + 2) * 3;\n");
  }

  #[test]
  fn gen_call() {
    assert_eq!(run("foo(1, 2);"), "foo(1, 2);\n");
  }

  #[test]
  fn gen_unary() {
    assert_eq!(run("-42;"), "-42;\n");
    assert_eq!(run("!true;"), "!true;\n");
  }

  #[test]
  fn gen_conditional() {
    assert_eq!(run("a ? b : c;"), "a ? b : c;\n");
  }

  #[test]
  fn gen_member_dot() {
    assert_eq!(run("a.b;"), "a.b;\n");
  }

  #[test]
  fn gen_member_bracket() {
    assert_eq!(run("a[0];"), "a[0];\n");
  }

  #[test]
  fn gen_variable_let() {
    assert_eq!(run("let x = 42;"), "let x = 42;\n");
  }

  #[test]
  fn gen_variable_no_init() {
    assert_eq!(run("let x;"), "let x;\n");
  }

  #[test]
  fn gen_array() {
    assert_eq!(run("[1, 2, 3];"), "[1, 2, 3];\n");
  }

  #[test]
  fn gen_object() {
    assert_eq!(run("({a: 1});"), "({a: 1});\n");
  }

  #[test]
  fn gen_object_shorthand() {
    assert_eq!(run("({x});"), "({x});\n");
  }

  #[test]
  fn gen_assignment() {
    assert_eq!(run("x = 42;"), "x = 42;\n");
  }

  #[test]
  fn gen_chained_call() {
    assert_eq!(run("a.b(c);"), "a.b(c);\n");
  }

  #[test]
  fn gen_comparison() {
    assert_eq!(run("a === b;"), "a === b;\n");
  }

  #[test]
  fn gen_nullish() {
    assert_eq!(run("a ?? b;"), "a ?? b;\n");
  }

  #[test]
  fn gen_exponentiation() {
    assert_eq!(run("2 ** 3;"), "2 ** 3;\n");
  }

  #[test]
  fn gen_parens_preserved() {
    assert_eq!(run("(x);"), "(x);\n");
  }

  #[test]
  fn gen_block_comment_skipped() {
    assert_eq!(run("/* comment */ 42;"), "42;\n");
  }

  #[test]
  fn gen_line_comment_skipped() {
    assert_eq!(run("// comment\n42;"), "42;\n");
  }

  #[test]
  fn gen_multiple_stmts() {
    let result = run("let a = 1;\nlet b = 2;\na + b;\n");
    assert!(result.contains("let a = 1;"));
    assert!(result.contains("let b = 2;"));
    assert!(result.contains("a + b;"));
  }

  #[test]
  fn gen_void() {
    assert_eq!(run("void 0;"), "void 0;\n");
  }

  #[test]
  fn gen_typeof() {
    assert_eq!(run("typeof x;"), "typeof x;\n");
  }

  #[test]
  fn gen_if_basic() {
    let result = run("if (x) y;");
    assert!(result.contains("if (x)"));
    assert!(result.contains("y;"));
  }

  #[test]
  fn gen_if_else() {
    let result = run("if (x) y; else z;");
    assert!(result.contains("if (x)"));
    assert!(result.contains("else"));
    assert!(result.contains("z;"));
  }

  #[test]
  fn gen_while_basic() {
    let result = run("while (x) y;");
    assert!(result.contains("while (x)"));
    assert!(result.contains("y;"));
  }

  #[test]
  fn gen_for_basic() {
    let result = run("for (let i = 0; i < 10; i = i + 1) x;");
    assert!(result.contains("for (let i = 0;"));
    assert!(result.contains("i < 10;"));
    assert!(result.contains("i = i + 1)"));
    assert!(result.contains("x;"));
  }

  #[test]
  fn gen_for_empty() {
    let result = run("for (;;) x;");
    assert!(result.contains("for (; ; )"));
  }

  #[test]
  fn gen_return_value() {
    let result = run("return 42;");
    assert_eq!(result.trim(), "return 42;");
  }

  #[test]
  fn gen_return_no_value() {
    let result = run("return;");
    assert_eq!(result.trim(), "return;");
  }

  #[test]
  fn gen_block() {
    let result = run("{ x; y; }");
    assert!(result.contains("{"));
    assert!(result.contains("x;"));
    assert!(result.contains("y;"));
    assert!(result.contains("}"));
  }

  #[test]
  fn gen_if_block() {
    let result = run("if (x) { y; z; }");
    assert!(result.contains("if (x)"));
    assert!(result.contains("y;"));
    assert!(result.contains("z;"));
  }

  #[test]
  fn gen_function_basic() {
    let result = run("function add(a, b) { return a + b; }");
    assert!(result.contains("function add(a, b)"));
    assert!(result.contains("return a + b;"));
  }

  #[test]
  fn gen_function_no_params() {
    let result = run("function noop() { }");
    assert!(result.contains("function noop()"));
  }

  #[test]
  fn gen_function_typed_params() {
    let result = run("function add(a: number, b: number): number { return a + b; }");
    assert!(result.contains("function add(a, b)"));
    assert!(!result.contains(": number"));
  }

  #[test]
  fn gen_arrow_single_param() {
    let result = run("let f = x => x + 1;");
    assert!(result.contains("x => x + 1"));
  }

  #[test]
  fn gen_arrow_multi_params() {
    let result = run("let f = (x, y) => x + y;");
    assert!(result.contains("(x, y) => x + y"));
  }

  #[test]
  fn gen_arrow_block_body() {
    let result = run("let f = (x) => { return x; }");
    assert!(result.contains("x => {"));
    assert!(result.contains("return x;"));
  }

  #[test]
  fn gen_import_named() {
    let result = run(r#"import { z } from "zod";"#);
    assert!(result.contains("import { z } from \"zod\";"));
  }

  #[test]
  fn gen_import_default() {
    let result = run(r#"import React from "react";"#);
    assert!(result.contains("import React from \"react\";"));
  }

  #[test]
  fn gen_import_as() {
    let result = run(r#"import { join as pathJoin } from "path";"#);
    assert!(result.contains("import { join as pathJoin } from \"path\";"));
  }

  #[test]
  fn gen_import_multiple() {
    let result = run(r#"import { a, b } from "mod";"#);
    assert!(result.contains("import { a, b } from \"mod\";"));
  }

  #[test]
  fn gen_type_alias_skipped() {
    let result = run("type ID = string;");
    assert!(result.trim().is_empty(), "type alias should produce no output");
  }

  #[test]
  fn gen_export_const() {
    let result = run("export const x = 42;");
    assert!(result.contains("export const x = 42;"));
  }

  #[test]
  fn gen_export_function() {
    let result = run("export function add(a, b) { return a + b; }");
    assert!(result.contains("export function add(a, b)"));
  }

  #[test]
  fn gen_import_then_code() {
    let result = run(r#"import { z } from "zod"; let x = 42;"#);
    assert!(result.contains("import { z } from \"zod\";"));
    assert!(result.contains("let x = 42;"));
  }

  #[test]
  fn gen_for_of() {
    let result = run("for (const x of items) { x; }");
    assert!(result.contains("for (const x of items)"));
  }

  #[test]
  fn gen_for_in() {
    let result = run("for (const x in obj) { x; }");
    assert!(result.contains("for (const x in obj)"));
  }

  #[test]
  fn gen_switch_case() {
    let result = run("switch (x) { case 1: y; break; default: z; }");
    assert!(result.contains("switch (x)"));
    assert!(result.contains("case 1:"));
    assert!(result.contains("default:"));
  }

  #[test]
  fn gen_throw() {
    let result = run("throw \"msg\";");
    assert!(result.contains("throw \"msg\";"));
  }

  #[test]
  fn gen_try_catch() {
    let result = run("try { x; } catch (e) { y; }");
    assert!(result.contains("try"));
    assert!(result.contains("catch (e)"));
  }

  #[test]
  fn gen_try_finally() {
    let result = run("try { x; } finally { z; }");
    assert!(result.contains("try"));
    assert!(result.contains("finally"));
  }

  #[test]
  fn gen_try_catch_finally() {
    let result = run("try { x; } catch (e) { y; } finally { z; }");
    assert!(result.contains("try"));
    assert!(result.contains("catch (e)"));
    assert!(result.contains("finally"));
  }
}
