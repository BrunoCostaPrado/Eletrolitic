import { Hono } from "hono"
import { createUser, getUserById, listUsers } from "../services/user.service.js"
import { error, success } from "../utils.js"

// ── User routes (Hono) ──────────────────────────────────────────────

const users = new Hono()

users.get("/", async (c) => {
	try {
		const result = await listUsers()
		return c.json(success(result))
	} catch (err) {
		const message = err instanceof Error ? err.message : "Unknown error"
		return c.json(error(message), 500)
	}
})

users.get("/:id", async (c) => {
	try {
		const user = await getUserById(c.req.param("id"))
		if (!user) return c.json(error("User not found"), 404)
		return c.json(success(user))
	} catch (err) {
		const message = err instanceof Error ? err.message : "Unknown error"
		return c.json(error(message), 500)
	}
})

users.post("/", async (c) => {
	try {
		const body = await c.req.json()
		const user = await createUser(body)
		return c.json(success(user), 201)
	} catch (err) {
		const message = err instanceof Error ? err.message : "Unknown error"
		return c.json(error(message), 400)
	}
})

export default users
