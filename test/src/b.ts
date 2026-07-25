import { z } from "zod"

interface B {
	a?: string
	b?: string
}

const aa = z.object({
	a: z.string(),
	b: z.string(),
})

type A = z.infer<typeof aa>

export const a: A = { a: "Hello" }
export const b: A = { b: "World" }
