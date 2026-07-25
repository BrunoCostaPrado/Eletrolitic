import { and, count, desc, eq, ilike, sql } from "drizzle-orm"
import { db } from "../db/client.js"
import { tags, todos, todoTags } from "../db/schema.js"
import type {
	CreateTodoInput,
	ListTodosQuery,
	UpdateTodoInput,
} from "../db/validation.js"
import {
	createTodoSchema,
	listTodosSchema,
	updateTodoSchema,
} from "../db/validation.js"

// ── Errors ───────────────────────────────────────────────────────────

export class NotFoundError extends Error {
	constructor(resource: string, id: string) {
		super(`${resource} ${id} not found`)
		this.name = "NotFoundError"
	}
}

// ── Async page iterator for batch operations ─────────────────────────

interface TodoPage {
	page: number
	rows: Array<{
		id: string
		userId: string
		title: string
		description: string | null
		status: string
		priority: string
		dueDate: Date | null
		createdAt: Date
		updatedAt: Date
	}>
}

async function fetchTodoPage(
	userId: string,
	page: number,
	pageSize: number,
): Promise<TodoPage> {
	const rows = await db
		.select()
		.from(todos)
		.where(eq(todos.userId, userId))
		.orderBy(desc(todos.createdAt))
		.limit(pageSize)
		.offset((page - 1) * pageSize)
	return { page, rows }
}

// ── Batch archive using for await ────────────────────────────────────

export async function archiveAllTodos(userId: string) {
	let archived = 0
	let page = 1
	const pages: TodoPage[] = []
	// Fetch all pages first
	while (true) {
		const p = await fetchTodoPage(userId, page, 50)
		if (p.rows.length === 0) break
		pages.push(p)
		page++
	}
	// Process with for await
	for await (const p of pages) {
		for await (const todo of p.rows) {
			await db
				.update(todos)
				.set({ status: "archived", updatedAt: new Date() })
				.where(eq(todos.id, todo.id))
			archived++
		}
	}
	return { archived }
}

// ── Batch delete using for await ─────────────────────────────────────

export async function deleteAllCompletedTodos(userId: string) {
	let deleted = 0
	let page = 1
	const pages: TodoPage[] = []
	while (true) {
		const p = await fetchTodoPage(userId, page, 50)
		if (p.rows.length === 0) break
		pages.push(p)
		page++
	}
	for await (const p of pages) {
		for await (const todo of p.rows) {
			if (todo.status === "done") {
				await db.delete(todoTags).where(eq(todoTags.todoId, todo.id))
				await db.delete(todos).where(eq(todos.id, todo.id))
				deleted++
			}
		}
	}
	return { deleted }
}

// ── Todo service ─────────────────────────────────────────────────────

export async function listTodos(userId: string, query: ListTodosQuery) {
	const parsed = listTodosSchema.parse(query)
	const conditions = [eq(todos.userId, userId)]

	if (parsed.status) conditions.push(eq(todos.status, parsed.status))
	if (parsed.priority) conditions.push(eq(todos.priority, parsed.priority))
	if (parsed.search) conditions.push(ilike(todos.title, `%${parsed.search}%`))

	const where = and(...conditions)

	const [totalResult] = await db
		.select({ total: count() })
		.from(todos)
		.where(where)

	const rows = await db
		.select()
		.from(todos)
		.where(where)
		.orderBy(desc(todos.createdAt))
		.limit(parsed.limit)
		.offset((parsed.page - 1) * parsed.limit)

	return {
		data: rows,
		total: totalResult.total,
		page: parsed.page,
		limit: parsed.limit,
		hasNext: parsed.page * parsed.limit < totalResult.total,
	}
}

export async function getTodoById(todoId: string, userId: string) {
	const [todo] = await db
		.select()
		.from(todos)
		.where(and(eq(todos.id, todoId), eq(todos.userId, userId)))
		.limit(1)

	if (!todo) throw new NotFoundError("Todo", todoId)

	const todoTagsList = await db
		.select({ tagId: todoTags.tagId })
		.from(todoTags)
		.where(eq(todoTags.todoId, todoId))

	const tagIds = todoTagsList.map((r) => r.tagId)
	const tagRows =
		tagIds.length > 0
			? await db.select().from(tags).where(sql`${tags.id} IN ${tagIds}`)
			: []

	return { ...todo, tags: tagRows }
}

export async function createTodo(userId: string, input: CreateTodoInput) {
	const parsed = createTodoSchema.parse(input)

	const [todo] = await db
		.insert(todos)
		.values({
			userId,
			title: parsed.title,
			description: parsed.description ?? null,
			priority: parsed.priority ?? "medium",
			dueDate: parsed.dueDate ? new Date(parsed.dueDate) : null,
		})
		.returning()

	if (parsed.tagIds && parsed.tagIds.length > 0) {
		await db
			.insert(todoTags)
			.values(parsed.tagIds.map((tagId) => ({ todoId: todo.id, tagId })))
	}

	return getTodoById(todo.id, userId)
}

export async function updateTodo(
	todoId: string,
	userId: string,
	input: UpdateTodoInput,
) {
	const parsed = updateTodoSchema.parse(input)

	// Verify ownership
	await getTodoById(todoId, userId)

	const updateData: Record<string, unknown> = { updatedAt: new Date() }
	if (parsed.title !== undefined) updateData.title = parsed.title
	if (parsed.description !== undefined)
		updateData.description = parsed.description
	if (parsed.status !== undefined) updateData.status = parsed.status
	if (parsed.priority !== undefined) updateData.priority = parsed.priority
	if (parsed.dueDate !== undefined)
		updateData.dueDate = parsed.dueDate ? new Date(parsed.dueDate) : null

	if (Object.keys(updateData).length > 1) {
		await db.update(todos).set(updateData).where(eq(todos.id, todoId))
	}

	if (parsed.tagIds !== undefined) {
		await db.delete(todoTags).where(eq(todoTags.todoId, todoId))
		if (parsed.tagIds.length > 0) {
			await db
				.insert(todoTags)
				.values(parsed.tagIds.map((tagId) => ({ todoId, tagId })))
		}
	}

	return getTodoById(todoId, userId)
}

export async function deleteTodo(todoId: string, userId: string) {
	await getTodoById(todoId, userId)
	await db.delete(todoTags).where(eq(todoTags.todoId, todoId))
	await db
		.delete(todos)
		.where(and(eq(todos.id, todoId), eq(todos.userId, userId)))
}

export async function getTodoStats(userId: string) {
	const [totalResult] = await db
		.select({ total: count() })
		.from(todos)
		.where(eq(todos.userId, userId))

	const byStatus = await db
		.select({ status: todos.status, count: count() })
		.from(todos)
		.where(eq(todos.userId, userId))
		.groupBy(todos.status)

	const byPriority = await db
		.select({ priority: todos.priority, count: count() })
		.from(todos)
		.where(eq(todos.userId, userId))
		.groupBy(todos.priority)

	return {
		total: totalResult.total,
		byStatus: Object.fromEntries(byStatus.map((r) => [r.status, r.count])),
		byPriority: Object.fromEntries(
			byPriority.map((r) => [r.priority, r.count]),
		),
	}
}
