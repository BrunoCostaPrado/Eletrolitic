import { defineConfig } from "./src/ferrite.js"

export default defineConfig({
	entry: ["src/ferrite.ts", "src/cli.ts"],
	format: "esm",
	dts: true,
	clean: true,
	splitting: false,
	minify: false,
	outDir: "dist",
})
