use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::ast::{Expression, Statement};
use crate::codegen::Codegen;
use crate::config::{self, CompilerOptions, ElectroliticConfig};
use crate::decl_emit;
use crate::diagnostic::{Severity, SourceFile};
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::source_map::SourceMap;
use crate::type_checker::TypeChecker;
use crate::type_checker::env::TypeEnv;
use crate::type_checker::ty::Type;

struct ModuleData {
  program: crate::ast::Program,
  source_file: SourceFile,
}

#[derive(Default)]
pub struct Compiler {
  modules: Vec<(PathBuf, ModuleData)>,
  registry: HashMap<String, HashMap<String, Type>>,
  options: CompilerOptions,
  /// Project root directory (for resolving path aliases against `base_url`).
  root_dir: PathBuf,
  /// Output directory (from electrolitic.config — if set, outputs go here instead of next to source).
  out_dir: Option<PathBuf>,
  /// Parsed electrolitic.config (entry, outDir, dts, minify, strict, etc.).
  electrolitic_cfg: Option<ElectroliticConfig>,
}

impl Compiler {
  #[must_use]
  pub fn new() -> Self {
    Self::default()
  }

  /// Compile entry point, resolving relative imports recursively.
  /// Returns (`out_path`, `js_output`) for each file in dependency order.
  pub fn compile(entry: &str) -> Result<Vec<(String, String)>, Vec<String>> {
    let mut compiler = Self::new();
    let entry_path =
      std::fs::canonicalize(entry).map_err(|e| vec![format!("Cannot resolve '{entry}': {e}")])?;
    // Load electrolitic.config.ts/js/json (walk up from entry's directory)
    let mut search_dir = entry_path.parent().map(|p| p.to_path_buf());
    while let Some(ref dir) = search_dir {
      if let Some((cfg, cfg_dir)) = config::load_electrolitic_config(dir) {
        if let Some(out) = &cfg.out_dir {
          compiler.out_dir = Some(cfg_dir.join(out));
        }
        compiler.electrolitic_cfg = Some(cfg);
        break;
      }
      if !search_dir.as_mut().unwrap().pop() {
        break;
      }
    }
    // Load tsconfig.json (walk up from entry point's directory)
    if let Some(dir) = entry_path.parent() {
      let (opts, found_dir) = config::load_tsconfig(dir)?;
      compiler.options = opts;
      compiler.root_dir = found_dir;
    }
    // Electrolitic config overrides tsconfig settings
    if let Some(ref cfg) = compiler.electrolitic_cfg {
      if let Some(ref t) = cfg.target {
        compiler.options.target = Some(t.clone());
      }
      if let Some(s) = cfg.strict {
        compiler.options.strict = Some(s);
      }
      if let Some(ref m) = cfg.module {
        compiler.options.module = Some(m.clone());
      }
      if let Some(ref mr) = cfg.module_resolution {
        compiler.options.module_resolution = Some(mr.clone());
      }
      if let Some(ref l) = cfg.lib {
        compiler.options.lib = Some(l.clone());
      }
      if let Some(ref p) = cfg.paths {
        compiler.options.paths = Some(p.clone());
      }
      if let Some(ref bu) = cfg.base_url {
        compiler.options.base_url = Some(bu.clone());
      }
      if let Some(ref j) = cfg.jsx {
        compiler.options.jsx = Some(j.clone());
      }
      if let Some(ref jf) = cfg.jsx_factory {
        compiler.options.jsx_factory = Some(jf.clone());
      }
      if let Some(ref jff) = cfg.jsx_fragment_factory {
        compiler.options.jsx_fragment_factory = Some(jff.clone());
      }
      if let Some(ed) = cfg.experimental_decorators {
        compiler.options.experimental_decorators = Some(ed);
      }
      if let Some(ei) = cfg.es_module_interop {
        compiler.options.es_module_interop = Some(ei);
      }
      if let Some(asdi) = cfg.allow_synthetic_default_imports {
        compiler.options.allow_synthetic_default_imports = Some(asdi);
      }
    }
    compiler.parse_recursive(&entry_path)?;
    compiler.type_check_all()
  }

  /// Compile with a specific tsconfig.json path.
  pub fn compile_with_tsconfig(
    entry: &str,
    tsconfig_path: &str,
  ) -> Result<Vec<(String, String)>, Vec<String>> {
    let mut compiler = Self::new();
    let entry_path =
      std::fs::canonicalize(entry).map_err(|e| vec![format!("Cannot resolve '{entry}': {e}")])?;
    let tc_path = std::fs::canonicalize(tsconfig_path)
      .map_err(|e| vec![format!("Cannot resolve tsconfig: {e}")])?;
    let content =
      std::fs::read_to_string(&tc_path).map_err(|e| vec![format!("Cannot read tsconfig: {e}")])?;
    compiler.options = config::parse_tsconfig(&content)?;
    if let Some(dir) = entry_path.parent() {
      compiler.root_dir = dir.to_path_buf();
    }
    compiler.parse_recursive(&entry_path)?;
    compiler.type_check_all()
  }

  /// Resolve an import source string to a filesystem path using tsconfig paths/baseUrl.
  /// Returns None for bare specifiers (`node_modules`).
  fn resolve_import(&self, source: &str, from: &Path) -> Option<PathBuf> {
    if source.starts_with('.') {
      // Relative import — resolve against parent directory
      return Some(from.parent().unwrap_or(from).join(source).with_extension("ts"));
    }
    // Check path aliases (e.g. "@/*" → "src/*")
    if let Some(resolved) = config::resolve_path_alias(source, source, &self.options) {
      // resolved is relative to base_url; join with root_dir
      return Some(self.root_dir.join(resolved).with_extension("ts"));
    }
    None // bare specifier (node_modules) — not resolved here
  }

  fn parse_recursive(&mut self, path: &Path) -> Result<(), Vec<String>> {
    // Already parsed?
    if self.modules.iter().any(|(p, _)| p == path) {
      return Ok(());
    }

    let key = path.to_string_lossy().to_string();
    let source =
      std::fs::read_to_string(path).map_err(|e| vec![format!("Cannot read '{key}': {e}")])?;
    let source_file = SourceFile::new(key, source.clone());
    let mut lexer = Lexer::new(&source);
    let tokens = lexer.tokenize().to_vec();
    let lex_diags = lexer.into_diagnostics();
    let mut parser = Parser::new(tokens, source_file.clone());
    let program = parser.parse();

    // Collect lex + parse errors with source context
    let mut errors: Vec<String> = lex_diags
      .iter()
      .filter(|d| d.severity == Severity::Error)
      .map(|d| source_file.format_error(&d.code, &d.message, d.span))
      .collect();
    errors.extend(
      parser
        .diagnostics()
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .map(|d| source_file.format_error(&d.code, &d.message, d.span)),
    );
    if !errors.is_empty() {
      return Err(errors);
    }

    // Recurse into imports (relative and path-aliased)
    let imports: Vec<String> = program
      .body
      .iter()
      .filter_map(|stmt| {
        if let Statement::ImportDeclaration { source, .. } = stmt {
          return Some(source.clone());
        }
        None
      })
      .collect();

    for import_source in &imports {
      if let Some(import_path) = self.resolve_import(import_source, path) {
        self.parse_recursive(&import_path)?;
      }
    }

    let source_file = SourceFile::new(path.to_string_lossy().to_string(), source);
    self.modules.push((path.to_path_buf(), ModuleData { program, source_file }));
    Ok(())
  }

  fn type_check_all(&mut self) -> Result<Vec<(String, String)>, Vec<String>> {
    let mut checker = TypeChecker::new();
    let mut outputs = Vec::new();
    let mut all_errors = Vec::new();

    // ponytail: clone registry once, not per module — update in-place, write back after
    checker.module_registry = std::mem::take(&mut self.registry);
    checker.options = self.options.clone();
    checker.root_dir.clone_from(&self.root_dir);

    // Modules are in dependency order (deps first) from parse_recursive's DFS.
    let module_count = self.modules.len();
    for i in 0..module_count {
      let (path, program, source_file) = {
        let (path, data) = &self.modules[i];
        (path.clone(), data.program.clone(), data.source_file.clone())
      };

      checker.current_module.clone_from(&path);

      let mut env = TypeEnv::new();
      checker.errors.clear();
      checker.narrowed.clear();
      checker.check(&program, &mut env);

      // Report errors with file info + source context
      for err in &checker.errors {
        let span = err.span();
        all_errors.push(source_file.format_error("E0001", &format!("{err}"), span));
      }

      if all_errors.is_empty() {
        // Collect exports for cross-file resolution
        let exports = Self::collect_exports_static(&program, &env);
        let key = path.to_string_lossy().to_string();
        checker.module_registry.insert(key, exports);
      }

      // Codegen with source map
      // Compute map path first to derive relative source name
      let map_path = if let Some(ref out_dir) = self.out_dir {
        let rel = path.strip_prefix(&self.root_dir).unwrap_or(path.as_path());
        out_dir.join(rel).with_extension("js.map")
      } else {
        path.with_extension("js.map")
      };
      let map_dir = map_path.parent().unwrap_or(Path::new("."));
      let source_rel = diff_paths(&path, map_dir);
      let source_name = source_rel.to_string_lossy().replace('\\', "/");
      let mut source_map =
        SourceMap::new(&source_file.source, &source_name, Some(&source_file.source));
      let mut codegen = Codegen::new();
      if self.electrolitic_cfg.as_ref().and_then(|c| c.format.as_deref()) == Some("cjs") {
        codegen.cjs = true;
      }
      // Safety: source_map outlives codegen (both on this stack frame)
      unsafe { codegen.set_source_map(&raw mut source_map) };
      codegen.generate(&program);
      let out_path = if let Some(ref out_dir) = self.out_dir {
        // Mirror source tree under out_dir
        let rel = path.strip_prefix(&self.root_dir).unwrap_or(path.as_path());
        out_dir.join(rel).with_extension("js")
      } else {
        path.with_extension("js")
      };
      let js_name = out_path.file_name().unwrap().to_string_lossy().to_string();
      let js_output = if self.electrolitic_cfg.as_ref().and_then(|c| c.minify) == Some(true) {
        minify_js(&codegen.output)
      } else {
        codegen.output
      };
      outputs.push((out_path.to_string_lossy().to_string(), js_output));
      // Source map (skip if sourcemap: false in electrolitic.config)
      let emit_map = self.electrolitic_cfg.as_ref().and_then(|c| c.sourcemap) != Some(false);
      if emit_map {
        let map_json = source_map.to_json(&js_name);
        outputs.push((map_path.to_string_lossy().to_string(), map_json));
      }

      // Declaration emit (skip if dts: false in electrolitic.config)
      let emit_dts = self.electrolitic_cfg.as_ref().and_then(|c| c.dts).unwrap_or(true);
      if emit_dts {
        let dts = decl_emit::emit_declarations(&program);
        let dts_path = if let Some(ref out_dir) = self.out_dir {
          let rel = path.strip_prefix(&self.root_dir).unwrap_or(path.as_path());
          out_dir.join(rel).with_extension("d.ts")
        } else {
          path.with_extension("d.ts")
        };
        outputs.push((dts_path.to_string_lossy().to_string(), dts));
      }
    }

    // Write registry back
    self.registry = std::mem::take(&mut checker.module_registry);

    if all_errors.is_empty() { Ok(outputs) } else { Err(all_errors) }
  }

  /// Collect exported symbols from a program's top-level declarations.
  fn collect_exports_static(program: &crate::ast::Program, env: &TypeEnv) -> HashMap<String, Type> {
    let mut exports = HashMap::new();
    for stmt in &program.body {
      if let Statement::ExportDeclaration { declaration, .. } = stmt {
        Self::extract_export_name(declaration.as_ref())
          .and_then(|name| env.lookup(&name).map(|t| (name, t)))
          .map(|(name, t)| exports.insert(name, t));
      }
    }
    exports
  }

  fn extract_export_name(decl: &Statement) -> Option<String> {
    match decl {
      Statement::FunctionDeclaration { name, .. } => {
        if name.is_empty() {
          None
        } else {
          Some(name.clone())
        }
      }
      Statement::VariableDeclaration { declarations, .. } => declarations.iter().find_map(|d| {
        if let Expression::Identifier { name, .. } = d.id.as_ref() {
          Some(name.clone())
        } else {
          None
        }
      }),
      Statement::ClassDeclaration { name, .. } => Some(name.clone()),
      Statement::TypeAliasDeclaration { name, .. } => Some(name.clone()),
      Statement::EnumDeclaration { name, .. } => Some(name.clone()),
      Statement::ExpressionStatement { .. } => Some("default".to_string()),
      Statement::DecoratedStatement { statement, .. } => Self::extract_export_name(statement),
      _ => None,
    }
  }
}

/// Compute relative path from `base` to `path`.
fn diff_paths(path: &Path, base: &Path) -> PathBuf {
  path.strip_prefix(base).map(|p| p.to_path_buf()).unwrap_or_else(|_| path.to_path_buf())
}

/// Minify JS via `npx terser`. Returns original on failure.
fn minify_js(input: &str) -> String {
  use std::io::Write;
  let Ok(mut child) = std::process::Command::new("npx")
    .args(["--yes", "terser", "--compress", "--mangle"])
    .stdin(std::process::Stdio::piped())
    .stdout(std::process::Stdio::piped())
    .stderr(std::process::Stdio::null())
    .spawn()
  else {
    return input.to_string();
  };
  if let Some(ref mut stdin) = child.stdin {
    let _ = stdin.write_all(input.as_bytes());
  }
  drop(child.stdin.take());
  match child.wait_with_output() {
    Ok(output) if output.status.success() => {
      String::from_utf8(output.stdout).unwrap_or_else(|_| input.to_string())
    }
    _ => input.to_string(),
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::fs;

  fn tmp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("compiler_test_{name}"));
    let _ = fs::create_dir_all(&dir);
    dir
  }

  fn write_file(dir: &Path, name: &str, content: &str) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, content).unwrap();
    path
  }

  fn clean_dir(dir: &Path) {
    let _ = fs::remove_dir_all(dir);
  }

  #[test]
  fn compile_single_file() {
    let dir = tmp_dir("single_file");
    let entry = write_file(&dir, "main.ts", "let x: number = 1;\n");
    let result = Compiler::compile(entry.to_str().unwrap());
    assert!(result.is_ok(), "errors: {:?}", result.err());
    let outputs = result.unwrap();
    assert_eq!(outputs.len(), 3); // main.js + main.js.map + main.d.ts
    assert!(outputs.iter().any(|(p, _)| p.ends_with(".js")));
    assert!(outputs.iter().any(|(p, _)| p.ends_with(".d.ts")));
    clean_dir(&dir);
  }

  #[test]
  fn compile_with_relative_import() {
    let dir = tmp_dir("relative_import");
    write_file(
      &dir,
      "utils.ts",
      "export function add(a: number, b: number): number {\n  return a + b;\n}\n",
    );
    write_file(
      &dir,
      "main.ts",
      "import { add } from \"./utils\";\nlet result: number = add(1, 2);\n",
    );
    let result = Compiler::compile(dir.join("main.ts").to_str().unwrap());
    assert!(result.is_ok(), "errors: {:?}", result.err());
    let outputs = result.unwrap();
    assert_eq!(outputs.len(), 6); // 2 files × (js + js.map + d.ts)
    // utils should come before main (dependency order)
    let js_files: Vec<_> = outputs.iter().filter(|(p, _)| p.ends_with(".js")).collect();
    assert_eq!(js_files.len(), 2);
    assert!(js_files[0].0.contains("utils"));
    assert!(js_files[1].0.contains("main"));
    clean_dir(&dir);
  }

  #[test]
  fn compile_type_error_across_files() {
    let dir = tmp_dir("cross_file_error");
    write_file(
      &dir,
      "lib.ts",
      "export function greet(name: string): string {\n  return name;\n}\n",
    );
    write_file(&dir, "main.ts", "import { greet } from \"./lib\";\nlet n: number = greet(42);\n");
    let result = Compiler::compile(dir.join("main.ts").to_str().unwrap());
    assert!(result.is_err(), "should have type errors");
    let errors = result.err().unwrap();
    assert!(!errors.is_empty());
    assert!(errors.iter().any(|e| e.contains("main.ts")));
    clean_dir(&dir);
  }

  #[test]
  fn compile_import_default() {
    let dir = tmp_dir("import_default");
    write_file(&dir, "mod.ts", "export default function hello(): string {\n  return \"hi\";\n}\n");
    write_file(&dir, "main.ts", "import hello from \"./mod\";\nlet h: string = hello();\n");
    let result = Compiler::compile(dir.join("main.ts").to_str().unwrap());
    assert!(result.is_ok(), "errors: {:?}", result.err());
    clean_dir(&dir);
  }

  #[test]
  fn compile_transitive_imports() {
    let dir = tmp_dir("transitive");
    write_file(&dir, "a.ts", "export function foo(): number {\n  return 1;\n}\n");
    write_file(
      &dir,
      "b.ts",
      "import { foo } from \"./a\";\nexport function bar(): number {\n  return foo();\n}\n",
    );
    write_file(&dir, "main.ts", "import { bar } from \"./b\";\nlet x: number = bar();\n");
    let result = Compiler::compile(dir.join("main.ts").to_str().unwrap());
    assert!(result.is_ok(), "errors: {:?}", result.err());
    let outputs = result.unwrap();
    assert_eq!(outputs.len(), 9); // 3 files × (js + js.map + d.ts)
    clean_dir(&dir);
  }

  #[test]
  fn compile_decl_emit() {
    let dir = tmp_dir("decl_emit");
    write_file(
      &dir,
      "lib.ts",
      "export function add(a: number, b: number): number { return a + b; }\nexport const PI: number = 3.14;\nexport type ID = string | number;\n",
    );
    let result = Compiler::compile(dir.join("lib.ts").to_str().unwrap());
    assert!(result.is_ok(), "errors: {:?}", result.err());
    let outputs = result.unwrap();
    assert_eq!(outputs.len(), 3); // lib.js + lib.js.map + lib.d.ts
    let dts = &outputs[2].1;
    assert!(dts.contains("export function add(a: number, b: number): number;"), "d.ts: {dts}");
    assert!(dts.contains("export const PI: number;"), "d.ts: {dts}");
    assert!(dts.contains("export type ID = string | number;"), "d.ts: {dts}");
    clean_dir(&dir);
  }

  #[test]
  fn compile_path_alias() {
    let dir = tmp_dir("path_alias");
    write_file(
      &dir,
      "tsconfig.json",
      r#"{"compilerOptions": {"baseUrl": ".", "paths": {"@/*": ["src/*"]}}}"#,
    );
    let src = dir.join("src");
    let _ = fs::create_dir_all(&src);
    write_file(
      &src,
      "utils.ts",
      "export function add(a: number, b: number): number { return a + b; }\n",
    );
    write_file(&src, "main.ts", "import { add } from \"@/utils\";\nlet r: number = add(1, 2);\n");
    let result = Compiler::compile(src.join("main.ts").to_str().unwrap());
    assert!(result.is_ok(), "errors: {:?}", result.err());
    clean_dir(&dir);
  }

  #[test]
  fn compile_source_map() {
    let dir = tmp_dir("source_map");
    write_file(
      &dir,
      "app.ts",
      "function greet(name: string): string {\n  return \"hello \" + name;\n}\nlet x: number = 42;\n",
    );
    let result = Compiler::compile(dir.join("app.ts").to_str().unwrap());
    assert!(result.is_ok(), "errors: {:?}", result.err());
    let outputs = result.unwrap();
    // Should have .js, .js.map, .d.ts
    assert_eq!(outputs.len(), 3);
    let map = &outputs[1].1;
    assert!(map.contains("\"version\":3"), "missing version: {map}");
    assert!(map.contains("\"sources\":[\""), "missing sources: {map}");
    assert!(map.contains("\"mappings\":\""), "missing mappings: {map}");
    // Mappings should have semicolons (multi-line output)
    let mappings_start = map.find("\"mappings\":\"").unwrap() + 12;
    let mappings_end = map[mappings_start..].find('"').unwrap() + mappings_start;
    let mappings = &map[mappings_start..mappings_end];
    assert!(mappings.contains(';'), "expected semicolons: {mappings}");
    clean_dir(&dir);
  }
}
