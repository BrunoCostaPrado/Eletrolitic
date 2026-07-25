# Ferrite

TypeScript-to-JavaScript compiler written in Rust. Drop-in replacement for `tsc` — outputs `.js`, `.js.map`, and `.d.ts` files with automatic config detection.

## Install

```bash
cargo install --path .
```

Or use the npm package (ships the binary):

```bash
npm install ferrite
```

## Usage

```bash
# Compile a file
ferrite src/index.ts

# With explicit tsconfig
ferrite src/index.ts --tsconfig ./tsconfig.json

# Use ferrite.config.ts (auto-detected in cwd)
ferrite build
ferrite compile        # alias
ferrite                # uses entry from config
```

### CLI flags

| Flag | Description |
|------|-------------|
| `--tsconfig <path>` | Path to tsconfig.json |
| `--outDir <dir>` | Output directory |
| `--noEmit` | Type-check only, no output |
| `--target <target>` | ES target (es2015–esnext) |
| `--strict` | Enable strict mode |
| `--help`, `-h` | Show help |
| `--version`, `-V` | Show version |

## Config

Create `ferrite.config.ts` in your project root:

```ts
import { defineConfig } from "ferrite"

export default defineConfig({
  entry: ["src/index.ts"],
  outDir: "dist",
  target: "es2020",
  strict: true,
  dts: true,
  sourcemap: true,
})
```

All fields are optional. Without a config file, ferrite compiles the file you pass on the CLI.

### Supported config fields

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `entry` | `string[]` | — | Entry point(s) |
| `outDir` | `string` | `"."` | Output directory |
| `target` | `string` | — | ES target (`es2015`–`esnext`) |
| `strict` | `boolean` | `false` | Enable strict mode |
| `dts` | `boolean` | `true` | Emit `.d.ts` declaration files |
| `sourcemap` | `boolean` | `true` | Emit `.js.map` source maps |
| `module` | `string` | — | Module system (`esnext`, `nodenext`, etc.) |
| `jsx` | `string` | — | JSX transform (`react`, `react-jsx`) |
| `paths` | `Record<string, string[]>` | — | Path aliases (reads from `tsconfig.json`) |
| `baseUrl` | `string` | — | Base URL for path resolution |
| `minify` | `boolean` | `false` | Strip whitespace and comments from output |
| `format` | `string` | `"esm"` | Module format (`esm` or `cjs`) |

### tsconfig.json

Ferrite walks up the directory tree looking for `tsconfig.json`. It strips comments (JSONC format), reads `compilerOptions`, and resolves path aliases. You can point to a specific file with `--tsconfig`.

## Features

- TypeScript → JavaScript compilation (strip types, enums, decorators)
- JSX → `React.createElement()` transform
- ES modules (`import`/`export`, `export * from`) and CommonJS output
- Dynamic `import()` expressions
- Source maps with multi-file support
- Declaration files (`.d.ts`)
- Path aliases from tsconfig.json
- Utility types: `Partial<T>`, `Omit<K,T>`
- Compound assignments: `??=`, `&&=`, `||=`, `^=`
- Minification (`minify: true` in config)
- Multi-error recovery per file
- npm binary packaging (macOS, Linux, Windows)

## Output

Each `.ts` file produces up to three files:

```
src/index.ts  →  dist/index.js       (JavaScript)
                dist/index.js.map    (source map)
                dist/index.d.ts      (declarations)
```

Source maps and declarations can be disabled via config.

## Examples

### Basic compilation
```bash
# Single file — no config needed
ferrite src/index.ts

# Output: src/index.js, src/index.js.map, src/index.d.ts
```

### With config
```ts
// ferrite.config.ts
import { defineConfig } from "ferrite"

export default defineConfig({
  entry: ["src/index.ts", "src/utils.ts"],
  outDir: "dist",
  target: "es2020",
  strict: true,
  dts: true,
  sourcemap: true,
  minify: false,
  format: "esm",  // or "cjs"
})
```

### CommonJS output
```bash
ferrite src/index.ts --format cjs
# Or in config: format: "cjs"
```

### JSX support
```tsx
// src/App.tsx
export function App() {
  return <div>Hello World</div>
}
// Output: React.createElement("div", null, "Hello World")
```

## Architecture

```
Source (.ts) → Lexer → Parser → AST → Type Checker → Codegen → JS + Source Map + .d.ts
```

### Pipeline stages

1. **Lexer** (`src/lexer.rs`) — Tokenizes TypeScript source into tokens
2. **Parser** (`src/parser/`) — Pratt parser for expressions, recursive descent for statements
3. **AST** (`src/ast.rs`) — TypeScript AST node definitions
4. **Type Checker** (`src/type_checker/`) — Type inference, assignability, utility types
5. **Code Generator** (`src/codegen/`) — Emits JavaScript, strips type annotations
6. **Source Maps** (`src/source_map.rs`) — Maps JS positions back to TS source
7. **Declaration Emitter** (`src/decl_emit.rs`) — Generates `.d.ts` files

### Key design decisions

- **Single-pass codegen** — No intermediate representation, directly emits JS from AST
- **Pratt parsing** — Operator precedence handled via binding power, not grammar rules
- **Incremental error recovery** — Parser continues after errors, reports multiple issues per file
- **Zero-copy tokens** — Lexer borrows source string, no allocation per token

## Performance

Ferrite is designed for speed:

- **No project references** — Compiles files independently, no dependency graph
- **Parallel compilation** — Multiple files compile in parallel (when using config)
- **Minimal allocations** — Arena-style parsing where possible
- **Fast source maps** — Single-pass mapping generation

Typical performance: ~10-50x faster than `tsc` for single-file compilation.

## Comparison with tsc

| Feature | Ferrite | tsc |
|---------|---------|-----|
| Speed | ~10-50x faster | Baseline |
| Type checking | Partial (growing) | Full |
| Declaration files | Yes | Yes |
| Source maps | Yes | Yes |
| JSX | Yes | Yes |
| Path aliases | Yes | Yes |
| Project references | No | Yes |
| Incremental builds | No | Yes |
| Watch mode | No | Yes |

**Use Ferrite when:** You need fast compilation, are okay with partial type checking, or want a lightweight alternative to tsc.

**Use tsc when:** You need full type checking, project references, or incremental builds.

## Testing

```bash
cargo test          # 698 tests (unit + stress)
cargo clippy        # lint
cargo fmt           # format
```

### Test structure

- `tests/` — Integration tests (parser, codegen, type checker)
- `src/` — Unit tests (inline `#[cfg(test)]` modules)
- Stress tests — Large input validation

## Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feat/amazing-feature`)
3. Commit changes (`git commit -m 'Add amazing feature'`)
4. Push to branch (`git push origin feat/amazing-feature`)
5. Open a Pull Request

### Development setup

```bash
git clone https://github.com/your-username/ferrite.git
cd ferrite
cargo build
cargo test
```

## License

ISC
