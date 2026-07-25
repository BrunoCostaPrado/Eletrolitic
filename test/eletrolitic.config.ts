import { defineConfig } from "eletrolitic"

export default defineConfig({
	entry: ["src/index.ts"],
	format: "esm",
	dts: false,
	sourcemap: false,
	clean: true,
	splitting: false,
	minify: false,
	outDir: "dist",
})
