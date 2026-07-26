import { execFileSync } from "node:child_process"
import {
	existsSync,
	mkdirSync,
	readFileSync,
	unlinkSync,
	writeFileSync,
} from "node:fs"
import { arch, platform } from "node:os"
import { basename, dirname, resolve } from "node:path"
import { fileURLToPath } from "node:url"
import type { CompilerConfig } from "./types.js"

export type { CompilerConfig }

const __dirname = dirname(fileURLToPath(import.meta.url))

/** Load eletrolitic.config.ts from project root. */
export async function loadConfig(): Promise<CompilerConfig | null> {
	const cwd = process.cwd()
	for (const name of [
		"eletrolitic.config.ts",
		"eletrolitic.config.js",
		"eletrolitic.config.mjs",
	]) {
		const p = resolve(cwd, name)
		if (!existsSync(p)) continue
		try {
			const mod = await import(p)
			const cfg = mod.default ?? mod
			if (cfg && typeof cfg === "object") return cfg as CompilerConfig
		} catch {
			// needs tsx or similar TS loader
		}
	}
	return null
}

function platformDir(): string {
	const p = platform()
	const a = arch()
	const os = p === "win32" ? "win32" : p === "darwin" ? "darwin" : "linux"
	const cpu = a === "arm64" ? "arm64" : "x64"
	return `${os}-${cpu}`
}

function findBinary(): string {
	const ext = platform() === "win32" ? ".exe" : ""
	const binName = `eletrolitic${ext}`
	const pkgBin = resolve(__dirname, `../bin/${platformDir()}/${binName}`)
	if (existsSync(pkgBin)) return pkgBin
	for (const dir of ["../../target/release", "../../target/debug"]) {
		const p = resolve(__dirname, dir, binName)
		if (existsSync(p)) return p
	}
	return "eletrolitic"
}

export interface CompileOutput {
	path: string
	content: string
}

export interface CompileResult {
	ok: boolean
	outputs: CompileOutput[]
	errors: string[]
}

function toArray(entry: string | string[] | Record<string, string>): string[] {
	if (Array.isArray(entry)) return entry
	if (typeof entry === "string") return [entry]
	return Object.values(entry)
}

/**
 * Compile a TypeScript file using the Rust compiler.
 * Loads eletrolitic.config.ts if no config passed.
 */
export async function compile(
	entry?: string,
	config?: CompilerConfig,
): Promise<CompileResult> {
	if (!config) config = (await loadConfig()) ?? ({} as CompilerConfig)

	const entries: string[] = entry
		? [entry]
		: config?.entry
			? toArray(config.entry)
			: []

	if (entries.length === 0) {
		return {
			ok: false,
			outputs: [],
			errors: [
				"No entry file specified. Pass a file or set entry in eletrolitic.config.ts",
			],
		}
	}

	const bin = findBinary()
	const allOutputs: CompileOutput[] = []
	const allErrors: string[] = []

	for (const e of entries) {
		const absEntry = resolve(e)
		const args = [absEntry]

		// Write temp tsconfig — pass all config, Rust CLI ignores unknowns
		let tmpConfig: string | null = null
		const tsOpts: Record<string, unknown> = {}
		for (const [k, v] of Object.entries(config)) {
			if (v !== undefined) tsOpts[k] = v
		}
		if (Object.keys(tsOpts).length > 0) {
			tmpConfig = absEntry.replace(/\.ts$/, ".ferrite-tsconfig.json")
			writeFileSync(
				tmpConfig,
				JSON.stringify({ compilerOptions: tsOpts }),
				"utf-8",
			)
			args.push("--tsconfig", tmpConfig)
		}

		try {
			execFileSync(bin, args, {
				encoding: "utf-8",
				timeout: 30_000,
				windowsHide: true,
			})

			const base = absEntry.replace(/\.ts$/, "")
			for (const ext of [".js", ".js.map", ".d.ts"]) {
				const outPath = base + ext
				if (!existsSync(outPath)) continue
				const content = readFileSync(outPath, "utf-8")
				if (config.outDir) {
					const outDir = resolve(config.outDir)
					const dest = resolve(outDir, basename(outPath))
					if (!existsSync(outDir)) mkdirSync(outDir, { recursive: true })
					writeFileSync(dest, content)
					allOutputs.push({ path: dest, content })
				} else {
					allOutputs.push({ path: outPath, content })
				}
			}
		} catch (err: unknown) {
			const e = err as { stderr?: string; stdout?: string; message?: string }
			const msg = e.stderr || e.stdout || e.message || "Unknown error"
			allErrors.push(...msg.split("\n").filter((l: string) => l.length > 0))
		} finally {
			if (tmpConfig && existsSync(tmpConfig)) unlinkSync(tmpConfig)
		}
	}

	return { ok: allErrors.length === 0, outputs: allOutputs, errors: allErrors }
}
