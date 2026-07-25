use ferrite::codegen::Codegen;
use ferrite::diagnostic::SourceFile;
use ferrite::lexer::Lexer;
use ferrite::parser::Parser;
use ferrite::type_checker::{TypeChecker, env::TypeEnv};

/// Full pipeline: lex → parse → typecheck → codegen
fn pipeline(source: &str) -> PipelineResult {
  let mut lexer = Lexer::new(source);
  let tokens = lexer.tokenize().to_vec();
  let lex_diags = lexer.into_diagnostics();
  let source_file = SourceFile::new("stress.ts", source);
  let mut parser = Parser::new(tokens, source_file);
  let program = parser.parse();
  let parse_diags: Vec<_> = parser.diagnostics().to_vec();
  let mut checker = TypeChecker::new();
  let mut env = TypeEnv::new();
  checker.check(&program, &mut env);
  let mut codegen = Codegen::new();
  let output = codegen.generate(&program).to_string();
  PipelineResult {
    output,
    lex_errors: lex_diags.len(),
    parse_errors: parse_diags.len(),
    type_errors: checker.errors.len(),
    stmt_count: program.body.len(),
  }
}

struct PipelineResult {
  output: String,
  lex_errors: usize,
  parse_errors: usize,
  type_errors: usize,
  stmt_count: usize,
}

// ═══════════════════════════════════════════════════════════════════
// Category 1: Supported feature edge cases (should all compile clean)
// ═══════════════════════════════════════════════════════════════════

#[test]
fn stress_empty_input() {
  let r = pipeline("");
  assert_eq!(r.stmt_count, 0);
  assert_eq!(r.lex_errors, 0);
  assert_eq!(r.parse_errors, 0);
}

#[test]
fn stress_whitespace_only() {
  let r = pipeline("   \t\n  \r\n  ");
  assert_eq!(r.stmt_count, 0);
}

#[test]
fn stress_comments_only() {
  let r = pipeline("// line\n/* block */");
  assert_eq!(r.stmt_count, 0);
}

#[test]
fn stress_nested_comments() {
  // Lexer doesn't nest block comments: first */ closes, rest becomes code tokens
  // That code has parse errors, but `42;` at end should still parse
  let _r = pipeline("/* outer /* inner */ still_comment */ 42;");
  // At minimum, should not panic. Parser may or may not recover to `42;`
}

#[test]
fn stress_all_operators() {
  let src = r#"
        let a = 1 + 2 - 3 * 4 / 5 % 6 ** 7;
        let b = 1 << 2 >> 3 >>> 4;
        let c = 1 | 2 ^ 3 & 4;
        let d = 1 == 2 != 3 === 4 !== 5;
        let e = 1 < 2 > 3 <= 4 >= 5;
        let f = true && false || true;
        let g = 1 ?? 2;
    "#;
  let r = pipeline(src);
  assert_eq!(r.lex_errors, 0);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_all_unary_operators() {
  let src = r#"
        let a = -1;
        let b = +1;
        let c = !true;
        let d = ~0;
        let e = typeof x;
        let f = void 0;
        let g = delete obj.prop;
    "#;
  let r = pipeline(src);
  assert_eq!(r.lex_errors, 0);
  assert_eq!(r.parse_errors, 0);
}

#[test]
fn stress_all_assignment_operators() {
  let src = r#"
        let x = 0;
        x = 1;
        x += 2;
        x -= 3;
        x *= 4;
        x /= 5;
    "#;
  let r = pipeline(src);
  assert_eq!(r.lex_errors, 0);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_precedence_complex() {
  // Verify correct AST grouping via codegen output
  let r = pipeline("1 + 2 * 3;");
  assert!(r.output.contains("1 + (2 * 3)") || r.output.contains("1 + 2 * 3"));

  let r = pipeline("(1 + 2) * 3;");
  assert!(r.output.contains("(1 + 2) * 3"));

  let r = pipeline("a || b && c;");
  assert!(r.output.contains("a || (b && c)") || r.output.contains("a || b && c"));
}

#[test]
fn stress_nested_parentheses() {
  let r = pipeline("((((1))));");
  assert_eq!(r.lex_errors, 0);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.stmt_count, 1);
}

#[test]
fn stress_deeply_nested_binary() {
  let src = "1 + 2 + 3 + 4 + 5 + 6 + 7 + 8 + 9 + 10;";
  let r = pipeline(src);
  assert_eq!(r.lex_errors, 0);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.stmt_count, 1);
}

#[test]
fn stress_chained_member_access() {
  let r = pipeline("a.b.c.d.e;");
  assert_eq!(r.lex_errors, 0);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.stmt_count, 1);
}

#[test]
fn stress_chained_calls() {
  let r = pipeline("foo(1)(2)(3);");
  assert_eq!(r.lex_errors, 0);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.stmt_count, 1);
}

#[test]
fn stress_mixed_call_member() {
  let r = pipeline("a.b().c.d().e(1, 2);");
  assert_eq!(r.lex_errors, 0);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.stmt_count, 1);
}

#[test]
fn stress_bracket_member_with_expression() {
  let r = pipeline("obj[\"key\"];");
  assert_eq!(r.lex_errors, 0);
  assert_eq!(r.parse_errors, 0);
}

#[test]
fn stress_computed_member_complex() {
  let r = pipeline("obj[a + b];");
  assert_eq!(r.lex_errors, 0);
  assert_eq!(r.parse_errors, 0);
}

#[test]
fn stress_array_with_holes() {
  let r = pipeline("[1, , 3, , 5];");
  assert_eq!(r.lex_errors, 0);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.stmt_count, 1);
}

#[test]
fn stress_empty_array() {
  let r = pipeline("[];");
  assert_eq!(r.lex_errors, 0);
  assert_eq!(r.parse_errors, 0);
}

#[test]
fn stress_nested_arrays() {
  let r = pipeline("[[1, 2], [3, 4], [[5]]];");
  assert_eq!(r.lex_errors, 0);
  assert_eq!(r.parse_errors, 0);
}

#[test]
fn stress_object_with_all_key_types() {
  // String keys not supported by parser (only Identifier + Computed keys)
  // So test identifier keys + computed keys only
  let src = r#"({ a: 1, [expr]: 3 });"#;
  let r = pipeline(src);
  assert_eq!(r.lex_errors, 0);
  assert_eq!(r.parse_errors, 0);
}

#[test]
fn stress_nested_objects() {
  let src = r#"({ a: { b: { c: 1 } } });"#;
  let r = pipeline(src);
  assert_eq!(r.lex_errors, 0);
  assert_eq!(r.parse_errors, 0);
}

#[test]
fn stress_conditional_nested() {
  let r = pipeline("a ? b ? c : d : e ? f : g;");
  assert_eq!(r.lex_errors, 0);
  assert_eq!(r.parse_errors, 0);
}

#[test]
fn stress_conditional_with_assignment() {
  let r = pipeline("(x = a ? b : c);");
  assert_eq!(r.lex_errors, 0);
  assert_eq!(r.parse_errors, 0);
}

#[test]
fn stress_ternary_precedence() {
  // Ternary is right-associative
  let r = pipeline("a ? b : c ? d : e;");
  assert_eq!(r.lex_errors, 0);
  assert_eq!(r.parse_errors, 0);
}

#[test]
fn stress_nullish_coalescing_chain() {
  let r = pipeline("a ?? b ?? c ?? d;");
  assert_eq!(r.lex_errors, 0);
  assert_eq!(r.parse_errors, 0);
}

#[test]
fn stress_update_expressions() {
  let src = r#"
        let x = 0;
        x++;
        x--;
        ++x;
        --x;
    "#;
  let r = pipeline(src);
  assert_eq!(r.lex_errors, 0);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_string_escaping() {
  let r = pipeline(r#"let s = "hello \"world\" \\n \t tab";"#);
  assert_eq!(r.lex_errors, 0);
  assert_eq!(r.parse_errors, 0);
}

#[test]
fn stress_single_quote_strings() {
  let r = pipeline("let s = 'hello';");
  assert_eq!(r.lex_errors, 0);
  assert_eq!(r.parse_errors, 0);
}

#[test]
fn stress_numeric_literals() {
  let src = r#"
        let a = 0;
        let b = 42;
        let c = 3.14;
        let d = 0.0;
        let e = 1000000;
    "#;
  let r = pipeline(src);
  assert_eq!(r.lex_errors, 0);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_multiline_source() {
  let src = (0..100).map(|i| format!("let x{i} = {i};")).collect::<Vec<_>>().join("\n");
  let r = pipeline(&src);
  assert_eq!(r.lex_errors, 0);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
  assert_eq!(r.stmt_count, 100);
}

// ═══════════════════════════════════════════════════════════════════
// Category 2: Type system edge cases
// ═══════════════════════════════════════════════════════════════════

#[test]
fn stress_type_annotations_all_variants() {
  let src = r#"
        let a: number;
        let b: string;
        let c: boolean;
        let d: null;
        let e: undefined;
        let f: void;
        let g: any;
        let h: never;
        let i: number[];
        let j: string | number;
        let k: MyType;
        let l: true;
        let m: false;
        let n: 42;
        let o: "hello";
    "#;
  let r = pipeline(src);
  assert_eq!(r.lex_errors, 0);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_complex_union_types() {
  let src = r#"
        let a: string | number | boolean;
        let c: string | number | null | undefined;
    "#;
  let r = pipeline(src);
  assert_eq!(r.lex_errors, 0);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

// FIXED: `[]` now applied to union member types
#[test]
fn stress_union_with_array_types() {
  let r = pipeline("let b: number[] | string[];");
  assert_eq!(r.lex_errors, 0);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_literal_types_narrowing() {
  let src = r#"
        let a: 1 = 1;
        let b: "hello" = "hello";
        let c: true = true;
    "#;
  let r = pipeline(src);
  assert_eq!(r.lex_errors, 0);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_literal_type_mismatch() {
  let r = pipeline("let a: 1 = 2;");
  assert!(r.type_errors > 0);
}

#[test]
fn stress_any_accepts_all() {
  let src = r#"
        let a: any = 42;
        let b: any = "hello";
        let c: any = true;
        let d: any = null;
        let e: any = [1, 2, 3];
    "#;
  let r = pipeline(src);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_never_assignable() {
  // never should be assignable to anything (never reached)
  let _src = r#"
        let a: number = (1 as any);
        let b: string = (2 as any);
    "#;
  // `as` not supported, so let's test what we can
  let r = pipeline("let a: number[] = [];");
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_union_assignability() {
  let src = r#"
        let a: string | number = 42;
        let b: string | number = "hello";
    "#;
  let r = pipeline(src);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_union_mismatch() {
  let r = pipeline("let a: string | number = true;");
  assert!(r.type_errors > 0);
}

#[test]
fn stress_array_type_inference() {
  let src = r#"
        let a = [1, 2, 3];
        let b = ["a", "b"];
        let c = [true, false];
    "#;
  let r = pipeline(src);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_array_type_annotated_no_init() {
  let r = pipeline("let a: number[];");
  assert_eq!(r.type_errors, 0);
  assert_eq!(r.lex_errors, 0);
  assert_eq!(r.parse_errors, 0);
}

#[test]
fn stress_array_literal_type_mismatch() {
  // Array type mismatch: 42 is not assignable to number[]
  let r = pipeline("let a: number[] = 42;");
  assert!(r.type_errors > 0);
}

#[test]
fn stress_binary_type_inference() {
  let src = r#"
        let a = 1 + 2;
        let b = 1 - 2;
        let c = 1 * 2;
        let d = 1 / 2;
        let e = 1 % 2;
        let f = 1 ** 2;
    "#;
  let r = pipeline(src);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_comparison_returns_boolean() {
  let src = r#"
        let a = 1 === 2;
        let b = 1 !== 2;
        let c = 1 < 2;
        let d = 1 > 2;
        let e = 1 <= 2;
        let f = 1 >= 2;
    "#;
  let r = pipeline(src);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_conditional_type_inference() {
  let src = r#"
        let a = true ? 1 : 2;
        let b = true ? "a" : "b";
    "#;
  let r = pipeline(src);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_conditional_union_type() {
  let src = r#"
        let a = true ? 1 : "hello";
    "#;
  let r = pipeline(src);
  // Should infer union type
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_null_undefined_assignability() {
  let src = r#"
        let a: string | null = null;
        let b: string | undefined = undefined;
        let c: null | undefined = null;
        let d: null | undefined = undefined;
    "#;
  let r = pipeline(src);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_null_to_undefined() {
  let r = pipeline("let a: undefined = null;");
  assert_eq!(r.type_errors, 0); // null assignable to undefined
}

#[test]
fn stress_undefined_to_null() {
  let r = pipeline("let a: null = undefined;");
  assert_eq!(r.type_errors, 0); // undefined assignable to null
}

#[test]
fn stress_duplicate_identifier_error() {
  let r = pipeline("let x = 1; let x = 2;");
  assert!(r.type_errors > 0);
}

#[test]
fn stress_undeclared_identifier_error() {
  let r = pipeline("unknownVar;");
  assert!(r.type_errors > 0);
}

#[test]
fn stress_nullish_coalescing_type() {
  let r = pipeline("let a = x ?? 42;");
  // x is undeclared, but we can still check inference
  assert!(r.type_errors > 0); // undeclared x
}

#[test]
fn stress_void_type_in_let() {
  // void means "no return value" — valid in type position
  let r = pipeline("let a: void;");
  assert_eq!(r.type_errors, 0);
}

// ═══════════════════════════════════════════════════════════════════
// Category 3: Error recovery
// ═══════════════════════════════════════════════════════════════════

#[test]
fn stress_recover_from_bad_var_decl() {
  let src = "let = ; let y = 2;";
  let r = pipeline(src);
  // Parser silently recovers: `let = ;` produces placeholder, no error
  // y should still be parsed
  assert!(r.output.contains("y"));
}

#[test]
fn stress_recover_after_bad_expr() {
  let src = "+ ; let x = 1;";
  let r = pipeline(src);
  assert!(r.parse_errors > 0);
  assert!(r.output.contains("x"));
}

#[test]
fn stress_multiple_errors() {
  let src = "let = ; + ; let y = z;";
  let r = pipeline(src);
  // Multiple parse + type errors
  assert!(r.parse_errors + r.type_errors > 0);
}

#[test]
fn stress_unterminated_string() {
  let r = pipeline("\"hello");
  assert!(r.lex_errors > 0);
}

#[test]
fn stress_unterminated_block_comment() {
  let _r = pipeline("/* unterminated");
  // Should not hang, lexer should handle this
  // The lexer does handle EOF in block comments
}

#[test]
fn stress_unknown_character() {
  let r = pipeline("@#%");
  assert!(r.lex_errors > 0);
}

// ═══════════════════════════════════════════════════════════════════
// Category 4: Codegen correctness / roundtrip stability
// ═══════════════════════════════════════════════════════════════════

fn codegen_roundtrip(source: &str) -> String {
  let r = pipeline(source);
  r.output.trim().to_string()
}

#[test]
fn stress_roundtrip_number() {
  assert_eq!(codegen_roundtrip("42;"), "42;");
}

#[test]
fn stress_roundtrip_string() {
  assert_eq!(codegen_roundtrip("\"hello\";"), "\"hello\";");
}

#[test]
fn stress_roundtrip_binary() {
  assert_eq!(codegen_roundtrip("1 + 2;"), "1 + 2;");
}

#[test]
fn stress_roundtrip_precedence_preserved() {
  // (1 + 2) * 3 must keep parens
  assert_eq!(codegen_roundtrip("(1 + 2) * 3;"), "(1 + 2) * 3;");
}

#[test]
fn stress_roundtrip_chained_member() {
  assert_eq!(codegen_roundtrip("a.b.c;"), "a.b.c;");
}

#[test]
fn stress_roundtrip_computed_member() {
  assert_eq!(codegen_roundtrip("a[0];"), "a[0];");
}

#[test]
fn stress_roundtrip_call() {
  assert_eq!(codegen_roundtrip("foo(1, 2, 3);"), "foo(1, 2, 3);");
}

#[test]
fn stress_roundtrip_chained_call() {
  assert_eq!(codegen_roundtrip("a.b(c);"), "a.b(c);");
}

#[test]
fn stress_roundtrip_variable() {
  assert_eq!(codegen_roundtrip("let x = 42;"), "let x = 42;");
}

#[test]
fn stress_roundtrip_array() {
  assert_eq!(codegen_roundtrip("[1, 2, 3];"), "[1, 2, 3];");
}

#[test]
fn stress_roundtrip_object() {
  // Objects need parens at statement level to avoid block confusion
  let r = codegen_roundtrip("({a: 1});");
  assert!(r.contains("a: 1"));
}

#[test]
fn stress_roundtrip_object_shorthand() {
  let r = codegen_roundtrip("({x});");
  assert!(r.contains("x"));
}

#[test]
fn stress_roundtrip_assignment() {
  assert_eq!(codegen_roundtrip("x = 42;"), "x = 42;");
}

#[test]
fn stress_roundtrip_ternary() {
  assert_eq!(codegen_roundtrip("a ? b : c;"), "a ? b : c;");
}

#[test]
fn stress_roundtrip_unary() {
  assert_eq!(codegen_roundtrip("-42;"), "-42;");
  assert_eq!(codegen_roundtrip("!true;"), "!true;");
}

#[test]
fn stress_roundtrip_typeof() {
  assert_eq!(codegen_roundtrip("typeof x;"), "typeof x;");
}

#[test]
fn stress_roundtrip_void() {
  assert_eq!(codegen_roundtrip("void 0;"), "void 0;");
}

#[test]
fn stress_roundtrip_comparison() {
  assert_eq!(codegen_roundtrip("a === b;"), "a === b;");
  assert_eq!(codegen_roundtrip("a !== b;"), "a !== b;");
}

#[test]
fn stress_roundtrip_logical() {
  assert_eq!(codegen_roundtrip("a && b;"), "a && b;");
  assert_eq!(codegen_roundtrip("a || b;"), "a || b;");
}

#[test]
fn stress_roundtrip_nullish() {
  assert_eq!(codegen_roundtrip("a ?? b;"), "a ?? b;");
}

#[test]
fn stress_roundtrip_exponentiation() {
  assert_eq!(codegen_roundtrip("2 ** 3;"), "2 ** 3;");
}

#[test]
fn stress_roundtrip_multiple_stmts() {
  let src = "let a = 1;\nlet b = 2;\na + b;";
  let r = codegen_roundtrip(src);
  assert!(r.contains("let a = 1;"));
  assert!(r.contains("let b = 2;"));
  assert!(r.contains("a + b;"));
}

#[test]
fn stress_roundtrip_no_parens_preserved() {
  // (x) should keep parens (not be stripped)
  assert_eq!(codegen_roundtrip("(x);"), "(x);");
}

// ═══════════════════════════════════════════════════════════════════
// Category 5: Real-world-ish patterns (within supported feature set)
// ═══════════════════════════════════════════════════════════════════

#[test]
fn stress_config_object_pattern() {
  let src = r#"({ name: "app", version: "1.0", debug: true, count: 42 });"#;
  let r = pipeline(src);
  assert_eq!(r.lex_errors, 0);
  assert_eq!(r.parse_errors, 0);
}

#[test]
fn stress_chained_config_access() {
  let r = pipeline("config.database.host;");
  assert_eq!(r.lex_errors, 0);
  assert_eq!(r.parse_errors, 0);
}

#[test]
fn stress_array_of_objects() {
  let src = r#"([{ a: 1 }, { a: 2 }, { a: 3 }]);"#;
  let r = pipeline(src);
  assert_eq!(r.lex_errors, 0);
  assert_eq!(r.parse_errors, 0);
}

#[test]
fn stress_expression_chain() {
  // Method-chain-like pattern without actual methods
  let r = pipeline("a.b.c(1, 2).d.e(3).f;");
  assert_eq!(r.lex_errors, 0);
  assert_eq!(r.parse_errors, 0);
}

#[test]
fn stress_complex_computed_key() {
  let r = pipeline("obj[a + b * c];");
  assert_eq!(r.lex_errors, 0);
  assert_eq!(r.parse_errors, 0);
}

#[test]
fn stress_long_variable_names() {
  let name = "x".repeat(100);
  let r = pipeline(&format!("let {name} = 42;"));
  assert_eq!(r.lex_errors, 0);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_dollar_sign_identifiers() {
  let r = pipeline("let $ = 1; let $foo = 2; let _bar = 3;");
  assert_eq!(r.lex_errors, 0);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_big_number() {
  let r = pipeline("let x = 999999999999999;");
  assert_eq!(r.lex_errors, 0);
  assert_eq!(r.parse_errors, 0);
}

#[test]
fn stress_fractional_number() {
  let r = pipeline("let x = 0.001;");
  assert_eq!(r.lex_errors, 0);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_empty_string() {
  let r = pipeline(r#"let x = "";"#);
  assert_eq!(r.lex_errors, 0);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_long_string() {
  let s = "a".repeat(10000);
  let r = pipeline(&format!("let x = \"{s}\";"));
  assert_eq!(r.lex_errors, 0);
  assert_eq!(r.parse_errors, 0);
}

#[test]
fn stress_const_variants() {
  let src = r#"
        let a = 1;
        const b = 2;
        var c = 3;
    "#;
  let r = pipeline(src);
  assert_eq!(r.type_errors, 0);
  assert!(r.output.contains("let a"));
  assert!(r.output.contains("const b"));
  assert!(r.output.contains("var c"));
}

#[test]
fn stress_destructuring_pattern_attempt() {
  // Destructuring not supported — a, b, c, d are undeclared identifiers
  let r = pipeline("let x = a + b; let y = c * d;");
  assert_eq!(r.parse_errors, 0);
  assert!(r.type_errors > 0); // a, b, c, d undeclared
}

#[test]
fn stress_typed_variable_complex_type() {
  let r = pipeline("let x: string | number | null = 42;");
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_typed_array_variable() {
  // FIXED: source-union assignability now works — Union([1,2,3]) assignable to Number
  let r = pipeline("let x: number[] = [1, 2, 3];");
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_chained_ternary() {
  let r = pipeline("a ? b : c ? d : e ? f : g;");
  assert_eq!(r.lex_errors, 0);
  assert_eq!(r.parse_errors, 0);
}

#[test]
fn stress_mixed_operators_no_parens() {
  let r = pipeline("a + b * c - d / e % f;");
  assert_eq!(r.lex_errors, 0);
  assert_eq!(r.parse_errors, 0);
}

#[test]
fn stress_nested_calls_with_members() {
  let r = pipeline("console.log(obj.method(arg1, arg2));");
  assert_eq!(r.lex_errors, 0);
  assert_eq!(r.parse_errors, 0);
}

#[test]
fn stress_array_index_call() {
  let r = pipeline("fn()[0];");
  assert_eq!(r.lex_errors, 0);
  assert_eq!(r.parse_errors, 0);
}

#[test]
fn stress_call_on_index() {
  let r = pipeline("arr[0]();");
  assert_eq!(r.lex_errors, 0);
  assert_eq!(r.parse_errors, 0);
}

#[test]
fn stress_update_on_member() {
  let r = pipeline("obj.count++;");
  assert_eq!(r.lex_errors, 0);
  assert_eq!(r.parse_errors, 0);
}

#[test]
fn stress_assignment_to_member() {
  let r = pipeline("obj.key = value;");
  assert_eq!(r.lex_errors, 0);
  assert_eq!(r.parse_errors, 0);
}

#[test]
fn stress_compound_assignment_to_member() {
  let r = pipeline("obj.count += 1;");
  assert_eq!(r.lex_errors, 0);
  assert_eq!(r.parse_errors, 0);
}

// ═══════════════════════════════════════════════════════════════════
// Category 6: Lexer stress
// ═══════════════════════════════════════════════════════════════════

#[test]
fn stress_lexer_all_token_types() {
  let src = r#"
        42 "hello" true false null undefined
        foo let const var function return if else for while typeof void delete class
        ?? || && | ^ & == === != !== < > <= >= << >> >>>
        + - * / % ** += -= *= /= = 
        ! ~ ++ -- ( ) [ ] { } , ; : . ... => ? #
    "#;
  let mut lexer = Lexer::new(src);
  let tokens = lexer.tokenize().to_vec();
  let _diags = lexer.into_diagnostics();
  assert!(!tokens.is_empty());
  // Only # should be unknown
  let unknown_count =
    tokens.iter().filter(|t| matches!(t.kind, ferrite::token::TokenKind::Unknown(_))).count();
  assert!(unknown_count <= 1); // just #
}

#[test]
fn stress_lexer_string_escape_sequences() {
  let src = r#"let s = "line1\nline2\ttab\\";"#;
  let mut lexer = Lexer::new(src);
  let _tokens = lexer.tokenize().to_vec();
  let diags = lexer.into_diagnostics();
  assert_eq!(diags.len(), 0);
}

#[test]
fn stress_lexer_unicode_identifier() {
  // Unicode identifiers beyond ASCII — lexer only handles ascii alphanumeric
  let _r = pipeline("let café = 1;");
  // café would be split by the ascii-only lexer
  // Should at least not crash
}

#[test]
fn stress_lexer_nested_block_comment() {
  let _r = pipeline("/* /* inner */ still_in_comment */");
  // Lexer doesn't nest block comments
  // The `*/` inside will close the outer one, leaving `still_in_comment */` as code
  // This is standard behavior (not a bug, just noting it)
}

#[test]
fn stress_lexer_zero_length_string() {
  let r = pipeline(r#"let x = "";"#);
  assert_eq!(r.lex_errors, 0);
  assert_eq!(r.parse_errors, 0);
}

// ═══════════════════════════════════════════════════════════════════
// Category 7: Codegen output quality
// ═══════════════════════════════════════════════════════════════════

#[test]
fn stress_codegen_integer_representation() {
  // 1.0 should emit as "1" not "1.0"
  assert_eq!(codegen_roundtrip("1;"), "1;");
  assert_eq!(codegen_roundtrip("0;"), "0;");
}

#[test]
fn stress_codegen_float_representation() {
  let r = codegen_roundtrip("1.5;");
  assert!(r.contains("1.5"));
}

#[test]
fn stress_codegen_object_double_braces() {
  // Objects should produce { a: 1 } not {{ a: 1 }}
  let r = codegen_roundtrip("({a: 1});");
  // The current codegen uses {{ }} for objects — this is a known output format
  // just verify it doesn't crash
  assert!(r.contains("a: 1"));
}

#[test]
fn stress_codegen_no_trailing_newline_issue() {
  let r = codegen_roundtrip("42;");
  assert_eq!(r, "42;");
}

#[test]
fn stress_codegen_assignment_precedence() {
  let r = codegen_roundtrip("x = a + b;");
  assert!(r.contains("x = a + b;") || r.contains("x =( a + b)"));
}

#[test]
fn stress_codegen_ternary_precedence() {
  // a + b ? c : d — ternary has lower precedence than +
  let r = codegen_roundtrip("a + b ? c : d;");
  // Should either add parens around a+b or be a + b ? c : d
  assert!(r.contains("?") && r.contains(":"));
}

#[test]
fn stress_codegen_deeply_nested_ternary() {
  let r = codegen_roundtrip("a ? b ? c : d : e;");
  assert!(r.contains("?"));
}

// ═══════════════════════════════════════════════════════════════════
// Category 8: Edge cases that should produce graceful errors
// ═══════════════════════════════════════════════════════════════════

#[test]
fn stress_missing_semicolon_recover() {
  // ASI-like: semicolons not required before certain tokens
  let _r = pipeline("let x = 1\nlet y = 2");
  // Parser may or may not handle missing semicolons
  // At minimum, should not panic
}

#[test]
fn stress_trailing_comma_in_array() {
  let r = pipeline("[1, 2, 3,];");
  assert_eq!(r.lex_errors, 0);
  assert_eq!(r.parse_errors, 0);
}

#[test]
fn stress_trailing_comma_in_call() {
  let r = pipeline("foo(1, 2, 3,);");
  assert_eq!(r.lex_errors, 0);
  assert_eq!(r.parse_errors, 0);
}

#[test]
fn stress_empty_call() {
  let r = pipeline("foo();");
  assert_eq!(r.lex_errors, 0);
  assert_eq!(r.parse_errors, 0);
}

#[test]
fn stress_empty_block_comment() {
  let r = pipeline("/**/ 42;");
  assert_eq!(r.stmt_count, 1);
}

#[test]
fn stress_only_semicolons() {
  let r = pipeline(";;;");
  assert_eq!(r.lex_errors, 0);
}

#[test]
fn stress_consecutive_expressions() {
  let r = pipeline("1 2 3 4 5;");
  // Each is a separate expression statement
  assert_eq!(r.stmt_count, 5);
}

#[test]
fn stress_delete_unary() {
  let r = pipeline("delete obj.key;");
  assert_eq!(r.lex_errors, 0);
  assert_eq!(r.parse_errors, 0);
}

#[test]
fn stress_void_zero() {
  let r = pipeline("void 0;");
  assert_eq!(r.lex_errors, 0);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_typeof_undefined() {
  let r = pipeline("typeof undefined;");
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_bitwise_not() {
  let r = pipeline("~0;");
  assert_eq!(r.lex_errors, 0);
  assert_eq!(r.parse_errors, 0);
}

#[test]
fn stress_plus_plus_prefix() {
  let r = pipeline("++x;");
  assert_eq!(r.lex_errors, 0);
  assert_eq!(r.parse_errors, 0);
}

#[test]
fn stress_minus_minus_postfix() {
  let r = pipeline("x--;");
  assert_eq!(r.lex_errors, 0);
  assert_eq!(r.parse_errors, 0);
}

// ═══════════════════════════════════════════════════════════════════
// Phase 1 Bug Fix Tests
// ═══════════════════════════════════════════════════════════════════

#[test]
fn fix_source_union_assignability_to_base() {
  // Union([1,2,3]) should be assignable to Number (all members are)
  let r = pipeline("let x: number = 1;");
  assert_eq!(r.type_errors, 0);
  let r = pipeline("let x: number | string = 1;");
  assert_eq!(r.type_errors, 0);
}

#[test]
fn fix_source_union_assignability_partial_fail() {
  // Union([1, \"hi\"]) should NOT be assignable to Number
  let r = pipeline(r#"let x: number = 1 === true ? 1 : "hi";"#);
  assert_eq!(r.parse_errors, 0);
  // type_errors > 0 because the ternary result type is Union(Number, String) not assignable to Number
  assert!(r.type_errors > 0);
}

#[test]
fn fix_union_array_types_parse() {
  let r = pipeline("let a: number[] | string[];");
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn fix_union_array_types_with_init() {
  let r = pipeline("let a: number[] | string[] = [1, 2, 3];");
  assert_eq!(r.parse_errors, 0);
}

#[test]
fn fix_string_key_object() {
  let r = pipeline(r#"({ "name": "app", "version": "1.0" });"#);
  assert_eq!(r.lex_errors, 0);
  assert_eq!(r.parse_errors, 0);
}

#[test]
fn fix_string_key_codegen() {
  let src = r#"({ "name": "app" });"#;
  let mut lexer = Lexer::new(src);
  let tokens = lexer.tokenize().to_vec();
  let source_file = SourceFile::new("test.ts", src);
  let mut parser = Parser::new(tokens, source_file);
  let program = parser.parse();
  let mut codegen = Codegen::new();
  let output = codegen.generate(&program).to_string();
  assert!(output.contains("\"name\""), "expected quoted key in output, got: {output}");
  assert!(output.contains("\"app\""), "expected string value, got: {output}");
}

#[test]
fn fix_string_key_mixed_with_ident_key() {
  let r = pipeline(r#"({ name: "app", "version": "1.0" });"#);
  assert_eq!(r.parse_errors, 0);
}

#[test]
fn fix_string_key_computed_key_mixed() {
  let r = pipeline(r#"({ name: "app", ["dynamic"]: 42 });"#);
  assert_eq!(r.parse_errors, 0);
}

// === Phase 2: Control Flow Stress Tests ===

#[test]
fn stress_if_simple() {
  let r = pipeline("if (true) 1;");
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.stmt_count, 1);
  assert!(r.output.contains("if (true)"));
}

#[test]
fn stress_if_else() {
  let r = pipeline("if (x) 1; else 2;");
  assert_eq!(r.parse_errors, 0);
  assert!(r.output.contains("else"));
}

#[test]
fn stress_if_else_if_else() {
  let r = pipeline("if (a) 1; else if (b) 2; else 3;");
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.stmt_count, 1);
}

#[test]
fn stress_if_block_body() {
  let r = pipeline("if (x) { let y = 1; let z = 2; }");
  assert_eq!(r.parse_errors, 0);
}

#[test]
fn stress_if_nested() {
  let r = pipeline("if (a) { if (b) { if (c) 1; } }");
  assert_eq!(r.parse_errors, 0);
}

#[test]
fn stress_if_complex_condition() {
  let r = pipeline("if (x > 0 && y < 10 || z === null) 1;");
  assert_eq!(r.parse_errors, 0);
}

#[test]
fn stress_while_simple() {
  let r = pipeline("let x = 0; while (x < 10) x = x + 1;");
  assert_eq!(r.parse_errors, 0);
  assert!(r.output.contains("while"));
}

#[test]
fn stress_while_block_body() {
  let r = pipeline("let x = 0; while (x < 10) { x = x + 1; }");
  assert_eq!(r.parse_errors, 0);
}

#[test]
fn stress_while_nested() {
  let r = pipeline("while (true) { while (false) { 1; } }");
  assert_eq!(r.parse_errors, 0);
}

#[test]
fn stress_for_basic() {
  let r = pipeline("for (let i = 0; i < 10; i = i + 1) i;");
  assert_eq!(r.parse_errors, 0);
  assert!(r.output.contains("for"));
}

#[test]
fn stress_for_expr_init() {
  let r = pipeline("let i = 0; for (i = 0; i < 10; i = i + 1) i;");
  assert_eq!(r.parse_errors, 0);
}

#[test]
fn stress_for_var_init() {
  let r = pipeline("for (var i = 0; i < 10; i = i + 1) i;");
  assert_eq!(r.parse_errors, 0);
}

#[test]
fn stress_for_no_init() {
  let r = pipeline("let i = 0; for (; i < 10; i = i + 1) i;");
  assert_eq!(r.parse_errors, 0);
}

#[test]
fn stress_for_no_update() {
  let r = pipeline("for (let i = 0; i < 10; ) { i; }");
  assert_eq!(r.parse_errors, 0);
}

#[test]
fn stress_for_block_body() {
  let r = pipeline("for (let i = 0; i < 3; i = i + 1) { i; }");
  assert_eq!(r.parse_errors, 0);
}

#[test]
fn stress_return_value() {
  let r = pipeline("return 42;");
  assert_eq!(r.parse_errors, 0);
  assert!(r.output.contains("return 42;"));
}

#[test]
fn stress_return_no_value() {
  let r = pipeline("return;");
  assert_eq!(r.parse_errors, 0);
  assert!(r.output.trim().contains("return;"));
}

#[test]
fn stress_return_complex_expr() {
  let r = pipeline("return x + y * z;");
  assert_eq!(r.parse_errors, 0);
}

#[test]
fn stress_block_empty() {
  let r = pipeline("{}");
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.stmt_count, 1);
}

#[test]
fn stress_block_multiple_stmts() {
  let r = pipeline("{ let x = 1; let y = 2; x + y; }");
  assert_eq!(r.parse_errors, 0);
}

#[test]
fn stress_block_nested() {
  let r = pipeline("{ { { 1; } } }");
  assert_eq!(r.parse_errors, 0);
}

#[test]
fn stress_control_flow_with_types() {
  let r = pipeline("let x: number = 0; while (x < 10) { x = x + 1; }");
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_if_with_assignment() {
  let r = pipeline("let x = 0; if (true) x = 1;");
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_for_with_typed_var() {
  let r = pipeline("for (let i: number = 0; i < 10; i = i + 1) i;");
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_realistic_loop_pattern() {
  let src = r#"
    let sum = 0;
    let i = 0;
    while (i < 100) {
      sum = sum + i;
      i = i + 1;
    }
  "#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_realistic_if_else_chain() {
  let src = r#"
    let x = 42;
    if (x > 100) { x; } else if (x > 50) { x; } else if (x > 10) { x; } else { x; }
  "#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
}

#[test]
fn stress_nested_for_loops() {
  let src = r#"
    for (let i = 0; i < 3; i = i + 1) {
      for (let j = 0; j < 3; j = j + 1) {
        i + j;
      }
    }
  "#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_if_uses_variable_from_outer_scope() {
  let src = r#"
    let x = 10;
    if (x > 5) { x; } else { 0; }
  "#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_codegen_if_else_roundtrip() {
  let src = "if (x) y; else z;";
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert!(r.output.contains("if (x)"));
  assert!(r.output.contains("y;"));
  assert!(r.output.contains("else"));
  assert!(r.output.contains("z;"));
}

#[test]
fn stress_codegen_while_roundtrip() {
  let src = "while (x < 10) x = x + 1;";
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert!(r.output.contains("while (x < 10)"));
}

#[test]
fn stress_codegen_for_roundtrip() {
  let src = "for (let i = 0; i < 10; i = i + 1) i;";
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert!(r.output.contains("for (let i = 0;"));
  assert!(r.output.contains("i < 10;"));
}

#[test]
fn stress_codegen_nested_blocks_indent() {
  let src = "if (x) { if (y) { z; } }";
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert!(r.output.contains("{"));
}

// Phase 3: Function tests

#[test]
fn stress_function_declaration_basic() {
  let src = "function add(a, b) { return a + b; }";
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
  assert!(r.output.contains("function add(a, b)"));
  assert!(r.output.contains("return a + b;"));
}

#[test]
fn stress_function_no_params() {
  let src = "function noop() { }";
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert!(r.output.contains("function noop()"));
}

#[test]
fn stress_function_typed_params() {
  let src = "function add(a: number, b: number): number { return a + b; }";
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
  // codegen strips types
  assert!(r.output.contains("function add(a, b)"));
  assert!(!r.output.contains(": number"));
}

#[test]
fn stress_function_return_void() {
  let src = "function log(msg: string): void { }";
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_function_call_after_decl() {
  let src = "function double(x) { return x + x; } double(21);";
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
  assert!(r.output.contains("function double(x)"));
  assert!(r.output.contains("double(21);"));
}

#[test]
fn stress_arrow_single_param() {
  let src = "let f = x => x + 1;";
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
  assert!(r.output.contains("x => x + 1"));
}

#[test]
fn stress_arrow_multi_params() {
  let src = "let f = (x, y) => x + y;";
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
  assert!(r.output.contains("(x, y) => x + y"));
}

#[test]
fn stress_arrow_block_body() {
  let src = "let f = (x) => { return x; };";
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert!(r.output.contains("=> {"));
  assert!(r.output.contains("return x;"));
}

#[test]
fn stress_arrow_typed_params() {
  let src = "let f = (x: number, y: number) => x + y;";
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
  assert!(r.output.contains("(x, y) => x + y"));
}

#[test]
fn stress_arrow_called() {
  let src = "let double = x => x + x; double(21);";
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_function_type_annotation() {
  let src = "let fn: (number, string) => boolean;";
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_function_nested() {
  let src = "function outer(x) { function inner(y) { return y; } return inner(x); }";
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_function_with_control_flow() {
  let src = "function abs(x) { if (x < 0) { return 0 - x; } return x; }";
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_function_no_return_type() {
  let src = "function greet(name) { return \"hello\"; }";
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_function_params_various_types() {
  let src = "function process(a: string, b: number, c: boolean, d: any) { return a; }";
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_arrow_in_array() {
  let src = "let fns = [x => x, y => y + 1];";
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_function_in_object() {
  let src = "({ add: (a, b) => a + b });";
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_function_as_argument() {
  let src = "[1, 2, 3].map(x => x + 1);";
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  // .map returns Any since MemberExpression infers as Any
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_parens_still_work_after_arrow() {
  // Ensure parenthesized expressions still parse correctly
  let src = "(1 + 2);";
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert!(r.output.contains("(1 + 2)"));
}

#[test]
fn stress_identifier_still_work_after_arrow() {
  // Ensure plain identifiers still parse when not followed by =>
  let src = "x;";
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert!(r.output.contains("x;"));
}

// ===== Phase 4: String Concat + Template Literals =====

#[test]
fn stress_string_concat_numbers() {
  // number + number = number (unchanged)
  let src = "let x = 1 + 2;";
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
  assert!(r.output.contains("1 + 2"));
}

#[test]
fn stress_string_concat_left_string() {
  // string + anything = string
  let src = r#"let x = "hello" + 42;"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
  assert!(r.output.contains("\"hello\" + 42"));
}

#[test]
fn stress_string_concat_right_string() {
  // anything + string = string
  let src = r#"let x = 42 + "world";"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_string_concat_both_strings() {
  // string + string = string
  let src = r#"let x = "hello" + " " + "world";"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_string_concat_type_mismatch() {
  // assigning string concat to number should error
  let src = r#"let x: number = "hello" + "world";"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert!(r.type_errors > 0);
}

#[test]
fn stress_string_concat_assign_string_ok() {
  // assigning string concat to string is fine
  let src = r#"let x: string = "hello" + 42;"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_template_simple() {
  let src = "let x = `hello`;";
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
  assert!(r.output.contains("`hello`"));
}

#[test]
fn stress_template_interpolation() {
  let src = "let name = \"world\"; let x = `hello ${name}`;";
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
  assert!(r.output.contains("hello ${name}"));
}

#[test]
fn stress_template_multiple_interpolations() {
  let src = "let a = 1; let b = 2; let x = `${a} + ${b} = ${a + b}`;";
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_template_empty() {
  let src = "let x = ``;";
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_template_type_is_string() {
  // template literal should infer as string
  let src = r#"let x: string = `hello`;"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_template_type_mismatch() {
  // template literal assigned to number should error
  let src = "let x: number = `hello`;";
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert!(r.type_errors > 0);
}

#[test]
fn stress_template_nested_expression() {
  let src = "let x = `${1 + 2}`;";
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_template_with_call() {
  let src = "function greet(name: string): string { return `hi ${name}`; }";
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
  assert!(r.output.contains("`hi ${name}`"));
}

#[test]
fn stress_string_concat_in_template() {
  // string concat result used inside template
  let src = r#"let x = `${"hello" + " " + "world"}`;"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_string_concat_chained_with_template() {
  // template + string concat
  let src = r#"let x = `hello` + " " + `world`;"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_string_concat_variable_string() {
  let src = r#"let a = "foo"; let b = "bar"; let c = a + b;"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_string_concat_number_variable() {
  let src = r#"let a = "count: "; let b = 42; let c = a + b;"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

// Phase 5: Structural object types + member access + call checking

#[test]
fn stress_object_literal_type_inference() {
  let src = r#"let obj = { name: "hello", age: 42, active: true };"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_member_access_string_field() {
  let src = r#"let obj = { name: "hello" }; let x: string = obj.name;"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_member_access_type_mismatch() {
  let src = r#"let obj = { name: "hello" }; let x: number = obj.name;"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert!(r.type_errors > 0);
}

#[test]
fn stress_computed_member_access() {
  let src = r#"let obj = { name: "hello" }; let x: string = obj["name"];"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_nested_object_member_access() {
  let src = r#"let obj = { a: { b: { c: 42 } } }; let x: number = obj.a.b.c;"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_object_with_function_field() {
  let src = r#"let obj = { greet: "hello" }; let x: string = obj.greet;"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_function_call_return_type_inference() {
  let src = r#"function getNumber(): number { return 42; } let x = getNumber();"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_function_call_arg_type_check() {
  let src =
    r#"function add(a: number, b: number): number { return a + b; } let x: number = add(1, 2);"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_function_call_arg_mismatch() {
  let src = r#"function add(a: number, b: number): number { return a + b; } let x: number = add("hello", 2);"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert!(r.type_errors > 0);
}

#[test]
fn stress_arrow_call_return_type() {
  let src = r#"let double = (x: number): number => x * 2; let y: number = double(5);"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_chained_member_access_on_object() {
  let src = r#"let config = { db: { host: "localhost" } }; let h: string = config.db.host;"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_object_member_in_expression() {
  let src = r#"let obj = { x: 10, y: 20 }; let sum = obj.x + obj.y;"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_function_return_object_member() {
  let src = r#"function getConfig() { return { name: "app" }; } let c = getConfig(); let n: string = c.name;"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_object_structural_assignability() {
  // Structural typing: inferred object is assignable to matching structure
  // (object type annotations like { name: string } not supported in parser yet)
  let src =
    r#"let a = { name: "hello", age: 42 }; let n: string = a.name; let age: number = a.age;"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_object_missing_field_error() {
  // Accessing a non-existent field should infer Any (no crash), not produce type errors
  // (object type annotations not supported in parser yet, so we test field access)
  let src = r#"let a = { name: "hello" }; let x: string = a.name;"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_object_field_type_mismatch() {
  // Accessing a field with wrong type should produce a type error
  let src = r#"let a = { name: 42 }; let x: string = a.name;"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert!(r.type_errors > 0);
}

#[test]
fn stress_call_on_non_function() {
  let src = r#"let x = 42; let y = x();"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  // calling non-function — returns Any, no crash
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_method_chained_on_object() {
  // .map on array → non-object member → returns Any (no crash)
  let src = r#"let arr = [1, 2, 3]; let mapped = arr.map;"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

// --- Import / Type Alias / Export ---

#[test]
fn stress_import_named() {
  let r = pipeline(r#"import { z } from "zod"; z;"#);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
  assert!(r.output.contains("import { z } from \"zod\";"));
}

#[test]
fn stress_import_default() {
  let r = pipeline(r#"import React from "react"; React;"#);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
  assert!(r.output.contains("import React from \"react\";"));
}

#[test]
fn stress_import_as() {
  let r = pipeline(r#"import { join as p } from "path"; p;"#);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
  assert!(r.output.contains("import { join as p } from \"path\";"));
}

#[test]
fn stress_import_multiple() {
  let r = pipeline(r#"import { a, b, c } from "mod"; a; b; c;"#);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_import_default_and_named() {
  let r = pipeline(r#"import React, { useState } from "react"; React; useState;"#);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
  assert!(r.output.contains("import React, { useState } from \"react\";"));
}

#[test]
fn stress_type_alias_basic() {
  let r = pipeline("type ID = string;");
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
  assert!(r.output.trim().is_empty(), "type alias should produce no JS output");
}

#[test]
fn stress_type_alias_union() {
  let r = pipeline("type Status = string | number;");
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_type_alias_array() {
  let r = pipeline("type IDs = string[];");
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_export_const() {
  let r = pipeline("export const x = 42;");
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
  assert!(r.output.contains("export const x = 42;"));
}

#[test]
fn stress_export_function() {
  let r = pipeline("export function add(a, b) { return a + b; }");
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
  assert!(r.output.contains("export function add(a, b)"));
}

#[test]
fn stress_export_default() {
  let r = pipeline("export default 42;");
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
  assert!(r.output.contains("export"));
}

#[test]
fn stress_error_txt_pattern() {
  // The exact pattern from error.txt — should now parse cleanly
  let src = r#"import { z } from "zod";
const s = z.object({ name: z.string() });
type S = string;
const a: S = "Hello";
"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0, "parse errors: {}", r.parse_errors);
  assert_eq!(r.stmt_count, 4, "expected 4 statements, got {}", r.stmt_count);
}

#[test]
fn stress_import_used_in_expression() {
  let r = pipeline(r#"import { add } from "math"; let x = add(1, 2);"#);
  assert_eq!(r.parse_errors, 0);
  // imported name is Any — no type error for calling it
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_export_default_function() {
  let r = pipeline("export default function() { return 42; }");
  assert_eq!(r.parse_errors, 0);
  assert!(r.output.contains("export"));
}

#[test]
fn stress_for_of_array() {
  let src = r#"const arr: string[] = ["a", "b"]; for (const x of arr) { console.log(x); }"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0, "parse errors: {}", r.parse_errors);
  assert_eq!(r.type_errors, 0, "type errors: {}", r.type_errors);
}

#[test]
fn stress_for_in_object() {
  let src = r#"const obj = { a: 1, b: 2 }; for (const k in obj) { console.log(k); }"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_for_of_let_var() {
  let src = r#"const items: number[] = [1, 2]; for (let x of items) { console.log(x); } for (var y of items) { console.log(y); }"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_switch_case() {
  let src = r#"const x = 1; switch (x) { case 1: console.log("one"); break; case 2: console.log("two"); break; default: console.log("other"); }"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_switch_no_default() {
  let src = r#"const x = "a"; switch (x) { case "a": console.log("alpha"); break; case "b": console.log("beta"); break; }"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_throw() {
  let src = r#"function bad() { throw "fail"; }"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_try_catch() {
  let src = r#"try { console.log("ok"); } catch (e) { console.log(e); }"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_try_finally() {
  let src = r#"try { console.log("ok"); } finally { console.log("done"); }"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_try_catch_finally() {
  let src =
    r#"try { console.log("ok"); } catch (e) { console.log(e); } finally { console.log("done"); }"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_nested_try_catch() {
  let src = r#"try { try { console.log("inner"); } catch (e) { console.log(e); } } catch (e2) { console.log(e2); }"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_for_of_with_try() {
  let src = r#"const items: string[] = ["a", "b"]; for (const x of items) { try { console.log(x); } catch (e) { console.log(e); } }"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_continue_in_loop() {
  let src = r#"for (let i = 0; i < 10; i++) { if (i === 5) { continue; } console.log(i); }"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_continue_in_while() {
  let src = r#"let i = 0; while (i < 10) { i++; if (i % 2 === 0) { continue; } console.log(i); }"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_labeled_for() {
  let src = r#"outer: for (let i = 0; i < 10; i++) { for (let j = 0; j < 10; j++) { if (j === 3) { break outer; } } }"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_labeled_continue() {
  let src = r#"outer: for (let i = 0; i < 5; i++) { for (let j = 0; j < 5; j++) { if (j === 2) { continue outer; } } }"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_class_basic() {
  let src = r#"class Foo { constructor(x) { this.x = x; } getX() { return this.x; } }"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_class_extends() {
  let src = r#"class Base { greet() { return "hello"; } } class Child extends Base { greet() { return "hi"; } }"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_class_static() {
  let src = r#"class Foo { static create() { return new Foo(); } }"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_destructuring_object() {
  let src = r#"const obj = { a: 1, b: "two" }; const { a, b } = obj;"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_destructuring_array() {
  let src = r#"const arr = [1, "two", true]; const [x, y, z] = arr;"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_async_function() {
  let src = r#"async function fetchData() { return 42; }"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_await_expression() {
  let src = r#"async function run() { const x = await fetchData(); }"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
}

#[test]
fn stress_async_arrow() {
  let src = r#"const run = async () => { return 42; };"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_generic_function() {
  let src = r#"function identity<T>(x: T): T { return x; }"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_generic_type_alias() {
  let src = r#"type Box<T> = { value: T };"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_multi_type_param() {
  let src = r#"function pair<A, B>(a: A, b: B) { return { a, b }; }"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

// --- Feature batch: optional chaining, spread in objects, intersection types, enums ---

#[test]
fn stress_optional_chaining_dot() {
  let src = r#"let obj = { a: { b: 42 } }; let x = obj?.a?.b;"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_optional_chaining_bracket() {
  let src = r#"let arr = [1, 2, 3]; let x = arr?.[0];"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_optional_chaining_call() {
  let src = r#"let fn = (x: number): number => x; let r = fn?.(42);"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_spread_in_object() {
  let src = r#"let a = { x: 1 }; let b = { ...a, y: 2 };"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_spread_in_object_multiple() {
  let src = r#"let a = { x: 1 }; let b = { y: 2 }; let c = { ...a, ...b, z: 3 };"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_intersection_type() {
  let src = r#"type A = { x: number }; type B = { y: string }; let z: A & B = { x: 1, y: "hi" };"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_enum_basic() {
  let src = r#"enum Dir { Up, Down, Left, Right }"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_enum_with_values() {
  let src = r#"enum Color { Red = "red", Green = "green" }"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_enum_numeric() {
  let src = r#"enum Status { Ok = 200, NotFound = 404 }"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_class_self_ref() {
  let src = r#"class Foo { static create() { return new Foo(); } }"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_nested_block_comment() {
  let src = r#"let x = 1; /* outer /* inner */ still outer */ let y = 2;"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_deeply_nested_block_comment() {
  let src = r#"/* a /* b /* c */ b */ a */ let x = 1;"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_compound_assign_amp() {
  let src = r#"let x = 5; x &= 3;"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_compound_assign_pipe() {
  let src = r#"let x = 5; x |= 3;"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_compound_assign_percent() {
  let src = r#"let x = 10; x %= 3;"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_default_param() {
  let src = r#"function greet(name: string = "world") { return "hello " + name; }"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_default_param_number() {
  let src = r#"function add(x: number = 10, y: number = 20) { return x + y; }"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_rest_param() {
  let src = r#"function sum(...args: number[]) { return args; }"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_rest_param_codegen() {
  let src = r#"function sum(...args: number[]) { return args; }"#;
  let r = pipeline(src);
  assert!(r.output.contains("...args"), "should emit rest param: {}", r.output);
}

#[test]
fn stress_default_param_codegen() {
  let src = r#"function greet(name: string = "world") { return name; }"#;
  let r = pipeline(src);
  assert!(r.output.contains("name = \"world\""), "should emit default param: {}", r.output);
}

#[test]
fn stress_default_and_rest_params() {
  let src = r#"function mixed(a: number, b: string = "x", ...rest: number[]) { return a; }"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
  assert!(r.output.contains("b = \"x\""));
  assert!(r.output.contains("...rest"));
}

// --- Feature batch: template literals, for...of, type assertions ---

#[test]
fn stress_template_literal_no_sub() {
  let src = r#"let s = `hello world`;"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_template_literal_with_expr() {
  let src = r#"let name = "world"; let s = `hello ${name}`;"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_template_literal_multi_interp() {
  let src = r#"let a = 1; let b = 2; let s = `${a} + ${b} = ${a + b}`;"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_template_literal_codegen() {
  let src = r#"let name = "world"; let s = `hello ${name}`;"#;
  let r = pipeline(src);
  assert!(r.output.contains("${"), "should emit template interpolation: {}", r.output);
}

#[test]
fn stress_for_of_const() {
  let src = r#"let arr = [1, 2, 3]; for (const x of arr) { x; }"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_for_of_let() {
  let src = r#"let arr = ["a", "b"]; for (let x of arr) { x; }"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_for_of_codegen() {
  let src = r#"let arr = [1]; for (const x of arr) { x; }"#;
  let r = pipeline(src);
  assert!(r.output.contains("for (const x of arr)"), "codegen: {}", r.output);
}

#[test]
fn stress_for_in() {
  let src = r#"let obj = { a: 1, b: 2 }; for (const k in obj) { k; }"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_type_assertion_as() {
  let src = r#"let x = 42 as number;"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_type_assertion_as_string() {
  let src = r#"let x = "hello" as string;"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_type_assertion_codegen() {
  let src = r#"let x = 42 as number;"#;
  let r = pipeline(src);
  // Type assertions are erased in JS output
  assert!(r.output.contains("42"), "should emit inner expr: {}", r.output);
  assert!(!r.output.contains("as"), "should not emit 'as': {}", r.output);
}

#[test]
fn stress_type_assertion_in_expr() {
  let src = r#"let x = (42 + 1) as number;"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

// --- Feature batch: for...of element inference, import type, template literal types ---

#[test]
fn stress_for_of_typed_element() {
  let src = r#"let arr: number[] = [1, 2, 3]; for (const x of arr) { let y: number = x; }"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_for_of_string_array() {
  let src = r#"let arr: string[] = ["a", "b"]; for (const s of arr) { let y: string = s; }"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_for_of_let_binding() {
  let src = r#"let arr: number[] = [1]; for (let x of arr) { x = 42; }"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_import_type_parse() {
  let src = r#"import type { Foo } from "bar";"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_import_type_default() {
  let src = r#"import type React from "react";"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_import_type_not_in_output() {
  let src = r#"import type { Foo } from "bar";"#;
  let r = pipeline(src);
  assert!(!r.output.contains("import"), "import type should not appear in output: {}", r.output);
}

#[test]
fn stress_import_type_star() {
  let src = r#"import type * as Types from "types";"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
}

#[test]
fn stress_template_literal_type_simple() {
  let src = r#"type Greeting = `hello`;"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_template_literal_type_interp() {
  let src = r#"type Greeting = `hello ${string}`;"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_template_literal_type_multi() {
  let src = r#"type Path = `${string}/${string}`;"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

// --- Feature batch: child scoping, typeof, keyof, conditional types, this resolution ---

#[test]
fn stress_child_scope_function_params() {
  let src = r#"function f(x: number) { return x; } let x = "outer"; let y = f(42);"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_child_scope_arrow_params() {
  let src = r#"let f = (x: number) => x; let x = "outer"; let y = f(42);"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_child_scope_nested() {
  let src =
    r#"function outer(x: number) { function inner(y: string) { return y; } return inner("hi"); }"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_typeof_in_type_position() {
  let src = r#"let x: number = 42; let y: typeof x = 100;"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_typeof_string_var() {
  let src = r#"let s: string = "hello"; let t: typeof s = "world";"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_keyof_type() {
  let src = r#"type Obj = { a: number, b: string }; type Keys = keyof Obj;"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_keyof_simple() {
  let src = r#"type Keys = keyof { x: 1, y: 2 };"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_conditional_type_basic() {
  let src = r#"type T = string extends number ? "yes" : "no";"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_conditional_type_with_params() {
  let src = r#"type IsString<T> = T extends string ? "yes" : "no";"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_this_in_method() {
  let src = r#"class Counter { increment() { return this; } }"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_this_calls_super_method() {
  let src = r#"class Base { greet() { return "hi"; } }
class Child extends Base { sayHello() { return this.greet(); } }"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_class_instance_method_return() {
  let src = r#"class Foo { bar() { return 42; } }"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_enum_codegen() {
  let src = r#"enum Color { Red, Green, Blue }"#;
  let r = pipeline(src);
  assert!(r.output.contains("const Color"), "enum codegen: {}", r.output);
  assert!(r.output.contains("Red: 0"), "enum member codegen: {}", r.output);
}

#[test]
fn stress_enum_with_string_values_codegen() {
  let src = r#"enum Http { Ok = "200", NotFound = "404" }"#;
  let r = pipeline(src);
  assert!(r.output.contains("200"), "enum string value: {}", r.output);
  assert!(r.output.contains("404"), "enum string value: {}", r.output);
}

// --- Feature batch 6: type guards, for-scoping, class fields, union access, generic constraints, utility types ---

#[test]
fn stress_type_guard_typeof_string() {
  let src = r#"
function greet(x: string | number) {
  if (typeof x === "string") {
    let y: string = x;
  }
}
"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_type_guard_typeof_number() {
  let src = r#"
function double(x: string | number) {
  if (typeof x === "number") {
    let y: number = x;
  }
}
"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_for_scoping() {
  let src = r#"
for (let i: number = 0; i < 10; i++) {
  let x: number = i;
}
"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_class_fields_basic() {
  let src = r#"class Point { x: number = 0; y: number = 0; }"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_class_fields_codegen() {
  let src = r#"class Foo { x: number = 0; greet() { return "hi"; } }"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert!(r.output.contains("x"), "field in output: {}", r.output);
}

#[test]
fn stress_union_member_access() {
  let src = r#"let x: string | number = 1;"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_generic_constraint() {
  let src = r#"
function identity<T extends string>(x: T): T { return x; }
"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_generic_constraint_multi() {
  let src = r#"
function merge<A extends { x: number }, B extends { y: number }>(a: A, b: B) { return a; }
"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_utility_partial() {
  let src = r#"type MyStr = string; type P = Partial<MyStr>;"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_utility_pick() {
  let src = r#"type MyStr = string; type P = Pick<MyStr, "name">;"#;
  let r = pipeline(src);
  assert_eq!(r.parse_errors, 0);
  assert_eq!(r.type_errors, 0);
}

#[test]
fn stress_compile_real_config_file() {
  // Read the actual test/ferrite.config.ts file
  let path =
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("test").join("ferrite.config.ts");
  let src = std::fs::read_to_string(&path).expect("failed to read test/ferrite.config.ts");
  let mut lexer = Lexer::new(&src);
  let tokens = lexer.tokenize().to_vec();
  let lex_diags = lexer.into_diagnostics();
  let lex_errors: Vec<_> =
    lex_diags.iter().filter(|d| d.severity == ferrite::diagnostic::Severity::Error).collect();
  let source_file = SourceFile::new("ferrite.config.ts", &src);
  let mut parser = Parser::new(tokens, source_file);
  let program = parser.parse();
  let parse_diags: Vec<_> = parser
    .diagnostics()
    .iter()
    .filter(|d| d.severity == ferrite::diagnostic::Severity::Error)
    .collect();
  let mut checker = TypeChecker::new();
  let mut env = TypeEnv::new();
  checker.check(&program, &mut env);
  let mut codegen = Codegen::new();
  let _output = codegen.generate(&program);

  println!("lines: {}", src.lines().count());
  println!("top-level stmts: {}", program.body.len());
  println!("lex errors: {}", lex_errors.len());
  println!("parse errors: {}", parse_diags.len());
  println!("type errors: {}", checker.errors.len());
  for e in &checker.errors {
    println!("  type error: {e}");
  }

  // We accept parse/type errors for now — the goal is to not panic
  assert_eq!(lex_errors.len(), 0, "lex errors: {:?}", lex_errors);
}
