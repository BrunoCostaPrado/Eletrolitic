import { z } from "zod"

// ── CreateTodo ───────────────────────────────────────────────────────

export const createTodoSchema = z.object({
	title: z.string().min(1).max(200),
	description: z.string().max(5000).optional(),
	priority: z.enum(["low", "medium", "high", "urgent"]).optional(),
	dueDate: z.string().datetime().optional(),
	tagIds: z.array(z.string().uuid()).max(10).optional(),
})

export type CreateTodoInput = z.infer<typeof createTodoSchema>

// ── UpdateTodo ───────────────────────────────────────────────────────

export const updateTodoSchema = z.object({
	title: z.string().min(1).max(200).optional(),
	description: z.string().max(5000).nullable().optional(),
	status: z.enum(["todo", "in_progress", "done", "archived"]).optional(),
	priority: z.enum(["low", "medium", "high", "urgent"]).optional(),
	dueDate: z.string().datetime().nullable().optional(),
	tagIds: z.array(z.string().uuid()).max(10).optional(),
})

export type UpdateTodoInput = z.infer<typeof updateTodoSchema>

// ── ListTodos query ──────────────────────────────────────────────────

export const listTodosSchema = z.object({
	status: z.enum(["todo", "in_progress", "done", "archived"]).optional(),
	priority: z.enum(["low", "medium", "high", "urgent"]).optional(),
	search: z.string().max(100).optional(),
	tagId: z.string().uuid().optional(),
	page: z.coerce.number().int().min(1).default(1),
	limit: z.coerce.number().int().min(1).max(100).default(20),
})

export type ListTodosQuery = z.infer<typeof listTodosSchema>

// ── CreateUser ───────────────────────────────────────────────────────

export const createUserSchema = z.object({
	email: z.string().email().max(255),
	name: z.string().min(1).max(100),
	avatarUrl: z.string().url().nullable().optional(),
})

export type CreateUserInput = z.infer<typeof createUserSchema>
