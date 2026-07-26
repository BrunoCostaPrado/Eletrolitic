/**
 * Eletrolitic compiler config types — used by CompilerConfig below.
 */

/** Runtime platform target. */
export type Platform = "browser" | "node" | "neutral"

/** Output module format. */
export type Format = "iife" | "cjs" | "esm"

/** File loader — how to handle non-JS/TS imports. */
export type Loader =
	| "base64"
	| "binary"
	| "copy"
	| "css"
	| "dataurl"
	| "default"
	| "empty"
	| "file"
	| "js"
	| "json"
	| "jsx"
	| "local-css"
	| "text"
	| "ts"
	| "tsx"

/** Browser engine target. */
export type BrowserTarget =
	| "chrome"
	| "deno"
	| "edge"
	| "firefox"
	| "hermes"
	| "ie"
	| "ios"
	| "node"
	| "opera"
	| "rhino"
	| "safari"

/** ECMAScript version target. */
export type EsTarget =
	| "es3"
	| "es5"
	| "es6"
	| "es2015"
	| "es2016"
	| "es2017"
	| "es2018"
	| "es2019"
	| "es2020"
	| "es2021"
	| "es2022"
	| "es2023"
	| "es2024"
	| "esnext"

/** Combined target: ES version, browser engine, or arbitrary string. */
export type Target = EsTarget | BrowserTarget | (string & {})

/** Entry point — array of paths or name→path map. */
export type Entry = string[] | Record<string, string>

/** ECMAScript year/edition number. */
export type Ecma = number

/** Name of a 'console' method to drop. */
export type ConsoleProperty = string

/** Drop all 'console.*' calls, or only specific methods. */
export type DropConsoleOption = boolean | ConsoleProperty[]

/** Tree-shaking strategy — enable, configure, or pass a preset name. */
export type TreeshakingStrategy = boolean | TreeshakingOptions | string

/** Fine-grained tree-shaking controls. */
export interface TreeshakingOptions {
	moduleSideEffects?: boolean
	propertyReadSideEffects?: boolean
	annotations?: boolean
	tryCatchDeoptimization?: boolean
	unknownGlobalsSideEffects?: boolean
}

/** Signal used to kill child processes. */
export type KillSignal = "SIGKILL" | "SIGTERM"

/** Signal used to kill child processes. */
export type MangleCacheValue = string | false

/** Inline functions level — 0=off, 1=simple, 2=with arguments, 3=any. */
export type InlineFunctions = 0 | 1 | 2 | 3
export type PureGettersValue = boolean | "strict"
export type SequencesValue = boolean | number
export type TopRetainValue = null | string | string[]
export type BoolOrRegex = boolean | string
export type KeepQuotedValue = boolean | "strict"

/** Terser minification configuration. */
export interface MinifyOptions {
	compress?: CompressValue
	ecma?: Ecma
	enclose?: EncloseValue
	ie8?: boolean
	keepClassnames?: BoolOrRegex
	keepFnames?: BoolOrRegex
	mangle?: MangleValue
	module?: boolean
	nameCache?: unknown
	format?: FormatOptions
	output?: FormatOptions
	parse?: ParseOptions
	safari10?: boolean
	sourceMap?: SourceMapOptions
	toplevel?: boolean
}

export type CompressValue = boolean | CompressOptions
export type EncloseValue = boolean | string
export type MangleValue = boolean | MangleOptions

export interface CompressOptions {
	arguments?: boolean
	arrows?: boolean
	booleansAsIntegers?: boolean
	booleans?: boolean
	collapseVars?: boolean
	comparisons?: boolean
	computedProps?: boolean
	conditionals?: boolean
	deadCode?: boolean
	defaults?: boolean
	directives?: boolean
	dropConsole?: DropConsoleOption
	dropDebugger?: boolean
	ecma?: Ecma
	evaluate?: boolean
	expression?: boolean
	globalDefs?: unknown
	hoistFuns?: boolean
	hoistProps?: boolean
	hoistVars?: boolean
	ie8?: boolean
	ifReturn?: boolean
	inline?: InlineFunctions
	joinVars?: boolean
	keepClassnames?: BoolOrRegex
	keepFargs?: boolean
	keepFnames?: BoolOrRegex
	keepInfinity?: boolean
	lhsConstants?: boolean
	loops?: boolean
	module?: boolean
	negateIife?: boolean
	passes?: number
	properties?: boolean
	pureFuncs?: string[]
	pureNew?: boolean
	pureGetters?: PureGettersValue
	reduceFuncs?: boolean
	reduceVars?: boolean
	sequences?: SequencesValue
	sideEffects?: boolean
	switches?: boolean
	toplevel?: boolean
	topRetain?: TopRetainValue
	typeofs?: boolean
	unsafeArrows?: boolean
	unsafe?: boolean
	unsafeComps?: boolean
	unsafeFunction?: boolean
	unsafeMath?: boolean
	unsafeSymbols?: boolean
	unsafeMethods?: boolean
	unsafeProto?: boolean
	unsafeRegexp?: boolean
	unsafeUndefined?: boolean
	unused?: boolean
}

export interface MangleOptions {
	eval?: boolean
	keepClassnames?: BoolOrRegex
	keepFnames?: BoolOrRegex
	module?: boolean
	properties?: ManglePropertiesValue
	reserved?: string[]
	safari10?: boolean
	toplevel?: boolean
}

export type ManglePropertiesValue = boolean | ManglePropertiesOptions

export interface ManglePropertiesOptions {
	builtins?: boolean
	debug?: boolean
	keepQuoted?: KeepQuotedValue
	regex?: string
	reserved?: string[]
}

export interface ParseOptions {
	bareReturns?: boolean
	ecma?: Ecma
	html5Comments?: boolean
	shebang?: boolean
}

export type CommentsValue = boolean | "all" | "some" | string
export type OutputQuoteStyle =
	| "preferDouble"
	| "alwaysSingle"
	| "alwaysDouble"
	| "alwaysOriginal"

export interface FormatOptions {
	asciiOnly?: boolean
	beautify?: boolean
	braces?: boolean
	comments?: CommentsValue
	ecma?: Ecma
	ie8?: boolean
	keepNumbers?: boolean
	indentLevel?: number
	indentStart?: number
	inlineScript?: boolean
	keepQuotedProps?: boolean
	maxLineLen?: number | false
	preamble?: string
	preserveAnnotations?: boolean
	quoteKeys?: boolean
	quoteStyle?: OutputQuoteStyle
	safari10?: boolean
	semicolons?: boolean
	shebang?: boolean
	shorthand?: boolean
	sourceMap?: SourceMapOptions
	webkit?: boolean
	width?: number
	wrapIife?: boolean
	wrapFuncArgs?: boolean
}

export interface SourceMapOptions {
	content?: string
	includeSources?: boolean
	filename?: string
	root?: string
	asObject?: boolean
	url?: string
}

/** Entry point — already defined above. */
export type WatchOption = boolean | string | WatchEntry[]
export type WatchEntry = string | boolean

export type OutExtensionFactory = OutExtensionObject | string
export interface OutExtensionObject {
	js?: string
	dts?: string
}

/** TypeScript declaration file generation options. */
export interface DtsConfig {
	entry?: Entry
	resolve?: DtsResolve
	only?: boolean
	banner?: string
	footer?: string
	compilerOptions?: unknown
}
export type DtsResolve = boolean | string[]
export type DtsOption = boolean | string | DtsConfig

export interface ExperimentalDtsConfig {
	entry?: Entry
	compilerOptions?: unknown
}
export type ExperimentalDtsOption = boolean | string | ExperimentalDtsConfig

export type CleanOption = boolean | string[]

/** Banner or footer — string for JS, or object with 'js'/'css' keys. */
export type BannerOrFooter = BannerFooterObject | string
export interface BannerFooterObject {
	js?: string
	css?: string
}

/** Build plugin configuration — lifecycle hooks as module paths. */
export interface PluginConfig {
	name: string
	esbuildOptions?: string
	buildStart?: string
	renderChunk?: string
	buildEnd?: string
}

// ── ferrite compiler config ──────────────────────────────────────

/** Full ferrite compiler configuration — tsconfig options + build options. */
export interface CompilerConfig {
	// ── tsconfig.json compilerOptions ──────────────────────────
	/** Target ECMAScript version. */
	target?: string
	/** Enable strict type checking. */
	strict?: boolean
	/** Module system. */
	module?: string
	/** Module resolution strategy. */
	moduleResolution?: string
	/** Include lib type definitions. */
	lib?: string[]
	/** Path aliases (e.g. { "@/*": ["src/*"] }). */
	paths?: Record<string, string[]>
	/** Base directory for path resolution. */
	baseUrl?: string
	/** JSX transform mode. */
	jsx?: string
	/** JSX factory function. */
	jsxFactory?: string
	/** JSX fragment factory. */
	jsxFragmentFactory?: string
	/** Enable experimental decorators. */
	experimentalDecorators?: boolean
	/** Enable ES module interop. */
	esModuleInterop?: boolean
	/** Allow synthetic default imports. */
	allowSyntheticDefaultImports?: boolean

	// ── build options ──────────────────────────────────────────
	/** Configuration name (for logging). */
	name?: string
	/** Entry point(s). */
	entry?: Entry
	entryPoints?: Entry
	/** Legacy output format. */
	legacyOutput?: boolean
	/** Build targets (ES version, browser engines). */
	buildTarget?: Target[]
	/** Minify output. */
	minify?: boolean | "terser"
	/** Advanced terser minification options. */
	terserOptions?: MinifyOptions
	/** Minify whitespace only. */
	minifyWhitespace?: boolean
	/** Minify identifiers only. */
	minifyIdentifiers?: boolean
	/** Minify syntax only. */
	minifySyntax?: boolean
	/** Preserve original identifier names. */
	keepNames?: boolean
	/** Watch mode. */
	watch?: WatchOption
	/** Paths to ignore in watch mode. */
	ignoreWatch?: string[]
	/** Command to run on successful build. */
	onSuccess?: string
	/** Output directory. */
	outDir?: string
	/** Output file extension mapping. */
	outExtension?: OutExtensionFactory
	/** Output format(s). */
	format?: Format | Format[]
	/** Global variable name for IIFE/UMD bundles. */
	globalName?: string
	/** Environment variable replacements. */
	env?: Record<string, string>
	/** Global identifier replacements. */
	define?: Record<string, string>
	/** TypeScript declaration generation. */
	dts?: DtsOption
	/** Experimental declaration generation. */
	experimentalDts?: ExperimentalDtsOption
	/** Source map style. */
	sourcemap?: boolean | "inline"
	/** Packages to include (not external). */
	noExternal?: string[]
	/** Packages to exclude from bundle. */
	external?: string[]
	/** Replace process.env.NODE_ENV. */
	replaceNodeEnv?: boolean
	/** Enable code splitting. */
	splitting?: boolean
	/** Clean output directory before build. */
	clean?: CleanOption
	/** Suppress output. */
	silent?: boolean
	/** Skip bundling node_modules. */
	skipNodeModulesBundle?: boolean
	/** Pure functions (tree-shaking). */
	pure?: string[]
	/** Enable bundling. */
	bundle?: boolean
	/** Modules to inject. */
	inject?: string[]
	/** Generate metafile. */
	metafile?: boolean
	/** Footer banner. */
	footer?: BannerOrFooter
	/** Header banner. */
	banner?: BannerOrFooter
	/** Runtime platform. */
	platform?: Platform
	/** File loader mapping. */
	loader?: Record<string, Loader>
	/** Config file path. */
	config?: string
	/** tsconfig.json path. */
	tsconfig?: string
	/** Inject styles. */
	injectStyle?: boolean | string
	/** Enable shims for CJS/ESM compat. */
	shims?: boolean
	/** Build plugins. */
	plugins?: PluginConfig[]
	/** Tree-shaking strategy. */
	treeshake?: TreeshakingStrategy
	/** Public directory. */
	publicDir?: boolean | string
	/** Signal to kill child processes. */
	killSignal?: KillSignal
	/** Enable CJS interop. */
	cjsInterop?: boolean
	/** Remove 'node:' protocol prefix. */
	removeNodeProtocol?: boolean
}
