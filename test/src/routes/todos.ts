import { Hono } from "hono"
import {
	archiveAllTodos,
	createTodo,
	deleteAllCompletedTodos,
	deleteTodo,
	getTodoById,
	getTodoStats,
	listTodos,
	NotFoundError,
	updateTodo,
} from "../services/todo.service.js"
import { error, success } from "../utils.js"

// ── Todo routes (Hono) ──────────────────────────────────────────────

const todos = new Hono()

todos.get("/", async (c) => {
	try {
		const userId = c.get("userId" as never) as string
		const result = await listTodos(
			userId,
			c.req.query() as Record<string, string>,
		)
		return c.json(success(result))
	} catch (err) {
		const message = err instanceof Error ? err.message : "Unknown error"
		return c.json(error(message), 500)
	}
})

todos.get("/stats", async (c) => {
	try {
		const userId = c.get("userId" as never) as string
		const stats = await getTodoStats(userId)
		return c.json(success(stats))
	} catch (err) {
		const message = err instanceof Error ? err.message : "Unknown error"
		return c.json(error(message), 500)
	}
})

todos.get("/:id", async (c) => {
	try {
		const userId = c.get("userId" as never) as string
		const todo = await getTodoById(c.req.param("id"), userId)
		return c.json(success(todo))
	} catch (err) {
		if (err instanceof NotFoundError) {
			return c.json(error(err.message), 404)
		}
		const message = err instanceof Error ? err.message : "Unknown error"
		return c.json(error(message), 500)
	}
})

todos.post("/", async (c) => {
	try {
		const userId = c.get("userId" as never) as string
		const body = await c.req.json()
		const todo = await createTodo(userId, body)
		return c.json(success(todo), 201)
	} catch (err) {
		const message = err instanceof Error ? err.message : "Unknown error"
		return c.json(error(message), 400)
	}
})

todos.put("/:id", async (c) => {
	try {
		const userId = c.get("userId" as never) as string
		const body = await c.req.json()
		const todo = await updateTodo(c.req.param("id"), userId, body)
		return c.json(success(todo))
	} catch (err) {
		if (err instanceof NotFoundError) {
			return c.json(error(err.message), 404)
		}
		const message = err instanceof Error ? err.message : "Unknown error"
		return c.json(error(message), 400)
	}
})

todos.delete("/:id", async (c) => {
	try {
		const userId = c.get("userId" as never) as string
		await deleteTodo(c.req.param("id"), userId)
		return c.body(null, 204)
	} catch (err) {
		if (err instanceof NotFoundError) {
			return c.json(error(err.message), 404)
		}
		const message = err instanceof Error ? err.message : "Unknown error"
		return c.json(error(message), 500)
	}
})

// ── Batch operations (for await stress test) ─────────────────────────

todos.post("/archive-all", async (c) => {
	try {
		const userId = c.get("userId" as never) as string
		const result = await archiveAllTodos(userId)
		return c.json(success(result))
	} catch (err) {
		const message = err instanceof Error ? err.message : "Unknown error"
		return c.json(error(message), 500)
	}
})

todos.post("/delete-completed", async (c) => {
	try {
		const userId = c.get("userId" as never) as string
		const result = await deleteAllCompletedTodos(userId)
		return c.json(success(result))
	} catch (err) {
		const message = err instanceof Error ? err.message : "Unknown error"
		return c.json(error(message), 500)
	}
})

export default todos
