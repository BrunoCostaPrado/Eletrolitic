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

### tsconfig.json

Ferrite walks up the directory tree looking for `tsconfig.json`. It strips comments (JSONC format), reads `compilerOptions`, and resolves path aliases. You can point to a specific file with `--tsconfig`.

## Output

Each `.ts` file produces up to three files:

```
src/index.ts  →  dist/index.js       (JavaScript)
                dist/index.js.map    (source map)
                dist/index.d.ts      (declarations)
```

Source maps and declarations can be disabled via config.

## Examples

```ts
// Single file — no config needed
let x: number = 1

// With imports
import { add } from "./utils"
let result = add(1, 2)

// Default exports
export default function hello(): string {
  return "world"
}
```

## Test

```bash
cargo test          # unit + stress tests
cargo clippy        # lint
```

## License

ISC
