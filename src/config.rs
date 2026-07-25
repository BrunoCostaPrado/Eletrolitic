use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

/// Ferrite build config (from ferrite.config.ts / ferrite.config.json).
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FerriteConfig {
  pub entry: Option<Vec<String>>,
  pub out_dir: Option<String>,
  pub format: Option<String>,
  pub target: Option<String>,
  pub dts: Option<bool>,
  pub minify: Option<bool>,
  pub clean: Option<bool>,
  pub splitting: Option<bool>,
  pub strict: Option<bool>,
  // ── tsconfig overrides ────────────────────────────────────────
  pub module: Option<String>,
  pub module_resolution: Option<String>,
  pub lib: Option<Vec<String>>,
  pub paths: Option<HashMap<String, Vec<String>>>,
  pub base_url: Option<String>,
  pub jsx: Option<String>,
  pub jsx_factory: Option<String>,
  pub jsx_fragment_factory: Option<String>,
  pub experimental_decorators: Option<bool>,
  pub es_module_interop: Option<bool>,
  pub allow_synthetic_default_imports: Option<bool>,
  // ── build options ─────────────────────────────────────────────
  pub name: Option<String>,
  pub sourcemap: Option<bool>,
  pub bundle: Option<bool>,
  pub define: Option<HashMap<String, String>>,
  pub env: Option<HashMap<String, String>>,
  pub external: Option<Vec<String>>,
  pub plugins: Option<Vec<String>>,
  pub silent: Option<bool>,
  pub tsconfig: Option<String>,
}

/// Parsed tsconfig.json compiler options.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompilerOptions {
  pub target: Option<String>,
  pub strict: Option<bool>,
  pub module: Option<String>,
  pub module_resolution: Option<String>,
  pub lib: Option<Vec<String>>,
  pub paths: Option<HashMap<String, Vec<String>>>,
  pub base_url: Option<String>,
  pub jsx: Option<String>,
  pub jsx_factory: Option<String>,
  pub jsx_fragment_factory: Option<String>,
  pub experimental_decorators: Option<bool>,
  pub es_module_interop: Option<bool>,
  pub allow_synthetic_default_imports: Option<bool>,
}

#[derive(Deserialize, Default)]
struct RawTsconfig {
  #[serde(rename = "compilerOptions")]
  compiler_options: Option<CompilerOptions>,
}

/// Load tsconfig.json by walking up from a directory.
/// Returns (options, found_dir) — found_dir is where tsconfig.json lives (for root_dir).
pub fn load_tsconfig(dir: &Path) -> Result<(CompilerOptions, PathBuf), Vec<String>> {
  let mut current = dir.to_path_buf();
  loop {
    let tsconfig_path = current.join("tsconfig.json");
    if tsconfig_path.exists() {
      let content = std::fs::read_to_string(&tsconfig_path)
        .map_err(|e| vec![format!("Cannot read tsconfig.json: {e}")])?;
      let opts = parse_tsconfig(&content)?;
      return Ok((opts, current));
    }
    if !current.pop() {
      break;
    }
  }
  Ok((CompilerOptions::default(), dir.to_path_buf()))
}

/// Parse tsconfig.json content.
pub fn parse_tsconfig(content: &str) -> Result<CompilerOptions, Vec<String>> {
  // Strip JS-style comments (tsconfig.json allows them, serde_json doesn't)
  let stripped = strip_json_comments(content);
  let raw: RawTsconfig =
    serde_json::from_str(&stripped).map_err(|e| vec![format!("Invalid tsconfig.json: {e}")])?;
  Ok(raw.compiler_options.unwrap_or_default())
}

/// Strip `//` and `/* */` comments from JSON content.
/// Handles strings correctly (doesn't strip inside quoted strings).
fn strip_json_comments(s: &str) -> String {
  let bytes = s.as_bytes();
  let mut out = Vec::with_capacity(s.len());
  let mut i = 0;
  while i < bytes.len() {
    match bytes[i] {
      b'"' => {
        out.push(b'"');
        i += 1;
        while i < bytes.len() && bytes[i] != b'"' {
          if bytes[i] == b'\\' {
            out.push(bytes[i]);
            i += 1;
            if i < bytes.len() {
              out.push(bytes[i]);
              i += 1;
            }
          } else {
            out.push(bytes[i]);
            i += 1;
          }
        }
        if i < bytes.len() {
          out.push(b'"');
          i += 1;
        }
      }
      b'/' if i + 1 < bytes.len() && bytes[i + 1] == b'/' => {
        // single-line comment — skip to end of line
        while i < bytes.len() && bytes[i] != b'\n' {
          i += 1;
        }
      }
      b'/' if i + 1 < bytes.len() && bytes[i + 1] == b'*' => {
        // block comment — skip to */
        i += 2;
        while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
          i += 1;
        }
        i += 2; // skip */
      }
      _ => {
        out.push(bytes[i]);
        i += 1;
      }
    }
  }
  String::from_utf8(out).unwrap_or_default()
}

/// Resolve path aliases from tsconfig.json paths.
#[must_use]
pub fn resolve_path_alias(
  alias: &str,
  _import_path: &str,
  options: &CompilerOptions,
) -> Option<PathBuf> {
  let base_url = options.base_url.as_deref().unwrap_or(".");
  let paths = options.paths.as_ref()?;
  // Try patterns with /* suffix first (wildcard matching)
  for (pattern, targets) in paths {
    if let Some(stem) = pattern.strip_suffix("/*")
      && alias.starts_with(stem)
      && alias.len() > stem.len()
    {
      let suffix = &alias[stem.len() + 1..]; // skip the '/'
      if let Some(target) = targets.first() {
        let resolved = target.replace('*', suffix);
        return Some(PathBuf::from(base_url).join(resolved));
      }
    }
  }
  // Try exact match
  for (pattern, targets) in paths {
    if pattern == alias
      && let Some(target) = targets.first()
    {
      return Some(PathBuf::from(base_url).join(target));
    }
  }
  None
}

/// Try to load ferrite.config.ts or ferrite.config.json from a directory.
/// Returns (config, dir) if found, None otherwise.
pub fn load_ferrite_config(dir: &Path) -> Option<(FerriteConfig, PathBuf)> {
  // Try .ts first, then .json
  for name in &["ferrite.config.ts", "ferrite.config.js", "ferrite.config.json"] {
    let path = dir.join(name);
    if let Ok(content) = std::fs::read_to_string(&path) {
      let json = if name.ends_with(".json") {
        content
      } else {
        // Extract defineConfig({...}) body from TS/JS
        match extract_define_config(&content) {
          Some(j) => j,
          None => continue,
        }
      };
      if let Ok(cfg) = serde_json::from_str::<FerriteConfig>(&json) {
        return Some((cfg, dir.to_path_buf()));
      }
    }
  }
  None
}

/// Extract the JSON object from `defineConfig({...})` in a TS/JS config file.
/// Strips imports, export default, and the defineConfig wrapper.
fn extract_define_config(content: &str) -> Option<String> {
  // Find `defineConfig({` and its matching `})`
  let start_marker = "defineConfig({";
  let start = content.find(start_marker)? + start_marker.len();
  // Find matching `}` — count braces
  let mut depth = 1i32;
  let bytes = content[start..].as_bytes();
  for (i, &b) in bytes.iter().enumerate() {
    match b {
      b'{' => depth += 1,
      b'}' => {
        depth -= 1;
        if depth == 0 {
          let mut end = start + i;
          // Skip trailing `)` from `defineConfig({...})`
          if end > start && content.as_bytes().get(end - 1) == Some(&b')') {
            end -= 1;
          }
          let body = &content[start..end];
          // Quote unquoted keys, wrap in {}, strip trailing commas
          let quoted = quote_keys(body);
          // Remove trailing commas before } or newline
          let cleaned = strip_trailing_commas(&quoted);
          return Some(format!("{{{cleaned}}}"));
        }
      }
      _ => {}
    }
  }
  None
}

/// Remove trailing commas before `}` or at end of string.
fn strip_trailing_commas(s: &str) -> String {
  let mut out = s.to_string();
  let bytes = out.as_bytes();
  let mut removes = Vec::new();
  for i in 0..bytes.len() {
    if bytes[i] == b',' {
      let mut j = i + 1;
      while j < bytes.len() && bytes[j].is_ascii_whitespace() {
        j += 1;
      }
      if j < bytes.len() && bytes[j] == b'}' {
        removes.push(i);
      }
    }
  }
  for &pos in removes.iter().rev() {
    out.remove(pos);
  }
  let trimmed = out.trim_end();
  if trimmed.ends_with(',') {
    let trimmed = trimmed.trim_end_matches(',');
    return trimmed.to_string();
  }
  trimmed.to_string()
}

/// Quote unquoted object keys in a JS object literal.
/// Turns `key: value` into `"key": value`.
fn quote_keys(s: &str) -> String {
  let mut out = String::with_capacity(s.len() + 32);
  let mut chars = s.chars().peekable();
  while let Some(c) = chars.next() {
    if c.is_alphanumeric() || c == '_' || c == '$' {
      let mut key = String::new();
      key.push(c);
      while let Some(&next) = chars.peek() {
        if next.is_alphanumeric() || next == '_' || next == '$' {
          key.push(chars.next().unwrap());
        } else {
          break;
        }
      }
      // Skip whitespace
      while let Some(&next) = chars.peek() {
        if next.is_whitespace() {
          chars.next();
        } else {
          break;
        }
      }
      // If followed by `:`, it's a key
      if chars.peek() == Some(&':') {
        out.push('"');
        out.push_str(&key);
        out.push('"');
      } else {
        out.push_str(&key);
      }
    } else {
      out.push(c);
    }
  }
  out
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parse_minimal_tsconfig() {
    let json = r#"{"compilerOptions": {"target": "es2020", "strict": true}}"#;
    let opts = parse_tsconfig(json).unwrap();
    assert_eq!(opts.target.as_deref(), Some("es2020"));
    assert_eq!(opts.strict, Some(true));
    assert!(opts.module.is_none());
  }

  #[test]
  fn parse_empty_tsconfig() {
    let json = r#"{}"#;
    let opts = parse_tsconfig(json).unwrap();
    assert!(opts.target.is_none());
    assert!(opts.strict.is_none());
  }

  #[test]
  fn parse_full_tsconfig() {
    let json = r#"{
      "compilerOptions": {
        "target": "esnext",
        "module": "esnext",
        "moduleResolution": "bundler",
        "strict": true,
        "lib": ["es2020", "dom"],
        "baseUrl": ".",
        "paths": {"@/*": ["src/*"]},
        "jsx": "react-jsx",
        "jsxFactory": "h",
        "jsxFragmentFactory": "Fragment",
        "experimentalDecorators": true,
        "esModuleInterop": true,
        "allowSyntheticDefaultImports": true
      }
    }"#;
    let opts = parse_tsconfig(json).unwrap();
    assert_eq!(opts.target.as_deref(), Some("esnext"));
    assert_eq!(opts.module.as_deref(), Some("esnext"));
    assert_eq!(opts.module_resolution.as_deref(), Some("bundler"));
    assert_eq!(opts.strict, Some(true));
    assert_eq!(opts.lib.as_ref().unwrap(), &vec!["es2020".to_string(), "dom".to_string()]);
    assert_eq!(opts.base_url.as_deref(), Some("."));
    assert_eq!(opts.jsx.as_deref(), Some("react-jsx"));
    assert_eq!(opts.jsx_factory.as_deref(), Some("h"));
    assert_eq!(opts.jsx_fragment_factory.as_deref(), Some("Fragment"));
    assert_eq!(opts.experimental_decorators, Some(true));
    assert_eq!(opts.es_module_interop, Some(true));
    assert_eq!(opts.allow_synthetic_default_imports, Some(true));
    let paths = opts.paths.as_ref().unwrap();
    assert_eq!(paths.get("@/*").unwrap(), &vec!["src/*".to_string()]);
  }

  #[test]
  fn parse_invalid_json() {
    let json = r#"not json"#;
    let err = parse_tsconfig(json).unwrap_err();
    assert!(err[0].contains("Invalid tsconfig.json"));
  }

  #[test]
  fn extract_define_config_basic() {
    let content = r#"import { defineConfig } from "ferrite"

export default defineConfig({
  entry: ["src/index.ts"],
  splitting: false,
})
"#;
    let result = extract_define_config(content).expect("should extract");
    eprintln!("JSON: {result}");
    let cfg: FerriteConfig = serde_json::from_str(&result).expect("should parse");
    assert_eq!(cfg.entry.as_ref().unwrap(), &vec!["src/index.ts".to_string()]);
    assert_eq!(cfg.splitting, Some(false));
  }

  #[test]
  fn extract_define_config_nested() {
    let content = r#"import { defineConfig } from "ferrite"

export default defineConfig({
  entry: ["src/index.ts"],
  splitting: false,
  sourcemap: false,
  clean: true,
  minify: false,
  dts: false,
  target: "esnext",
  outDir: "dist",
  tsconfig: "tsconfig.json",
  external: ["react"],
  format: "esm",
  loader: {
    ".js": "tsx",
  },
})
"#;
    let result = extract_define_config(content).expect("should extract");
    eprintln!("JSON: {result}");
    let cfg: FerriteConfig = serde_json::from_str(&result).expect("should parse");
    assert_eq!(cfg.entry.as_ref().unwrap(), &vec!["src/index.ts".to_string()]);
  }

  #[test]
  fn resolve_path_alias_basic() {
    let mut opts = CompilerOptions::default();
    opts.base_url = Some(".".to_string());
    opts.paths = Some(HashMap::from([("@/*".to_string(), vec!["src/*".to_string()])]));
    let resolved = resolve_path_alias("@/utils", "@/utils", &opts);
    assert!(resolved.is_some());
    let resolved_str = resolved.unwrap().to_string_lossy().replace('\\', "/");
    assert!(resolved_str.ends_with("src/utils"), "got: {resolved_str}");
  }
}
