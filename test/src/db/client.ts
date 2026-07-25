import { drizzle } from "drizzle-orm/postgres-js"
import postgres from "postgres"
import * as schema from "./schema.js"

// ── Connection ───────────────────────────────────────────────────────

const DATABASE_URL =
	process.env.DATABASE_URL ?? "postgres://localhost:5432/todo_api"

const client = postgres(DATABASE_URL, {
	max: 10,
	idle_timeout: 20,
	connect_timeout: 10,
})

export const db = drizzle(client, { schema })

// ── Helper: now timestamp ────────────────────────────────────────────

export function now(): Date {
	return new Date()
}

// ── Helper: paginate ─────────────────────────────────────────────────

export function paginate<T>(items: T[], page: number, limit: number) {
	const start = (page - 1) * limit
	return {
		data: items.slice(start, start + limit),
		total: items.length,
		page,
		limit,
		hasNext: start + limit < items.length,
	}
}
