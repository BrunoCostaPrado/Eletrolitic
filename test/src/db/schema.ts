import {
	index,
	pgEnum,
	pgTable,
	text,
	timestamp,
	uniqueIndex,
	uuid,
	varchar,
} from "drizzle-orm/pg-core"

// ── Enums ────────────────────────────────────────────────────────────

export const priorityEnum = pgEnum("priority", [
	"low",
	"medium",
	"high",
	"urgent",
])
export const todoStatusEnum = pgEnum("todo_status", [
	"todo",
	"in_progress",
	"done",
	"archived",
])

// ── Users ────────────────────────────────────────────────────────────

export const users = pgTable(
	"users",
	{
		id: uuid("id").defaultRandom().primaryKey(),
		email: varchar("email", { length: 255 }).notNull().unique(),
		name: varchar("name", { length: 100 }).notNull(),
		avatarUrl: text("avatar_url"),
		createdAt: timestamp("created_at", { withTimezone: true })
			.defaultNow()
			.notNull(),
		updatedAt: timestamp("updated_at", { withTimezone: true })
			.defaultNow()
			.notNull(),
	},
	(table) => [uniqueIndex("idx_users_email").on(table.email)],
)

// ── Todos ────────────────────────────────────────────────────────────

export const todos = pgTable(
	"todos",
	{
		id: uuid("id").defaultRandom().primaryKey(),
		userId: uuid("user_id")
			.notNull()
			.references(() => users.id, { onDelete: "cascade" }),
		title: varchar("title", { length: 200 }).notNull(),
		description: text("description"),
		status: todoStatusEnum("status").default("todo").notNull(),
		priority: priorityEnum("priority").default("medium").notNull(),
		dueDate: timestamp("due_date", { withTimezone: true }),
		createdAt: timestamp("created_at", { withTimezone: true })
			.defaultNow()
			.notNull(),
		updatedAt: timestamp("updated_at", { withTimezone: true })
			.defaultNow()
			.notNull(),
	},
	(table) => [
		index("idx_todos_user_id").on(table.userId),
		index("idx_todos_status").on(table.status),
		index("idx_todos_priority").on(table.priority),
	],
)

// ── Tags ─────────────────────────────────────────────────────────────

export const tags = pgTable("tags", {
	id: uuid("id").defaultRandom().primaryKey(),
	name: varchar("name", { length: 50 }).notNull(),
	color: varchar("color", { length: 7 }).default("#6366f1").notNull(),
	createdAt: timestamp("created_at", { withTimezone: true })
		.defaultNow()
		.notNull(),
})

// ── Todo ↔ Tags junction ─────────────────────────────────────────────

export const todoTags = pgTable(
	"todo_tags",
	{
		todoId: uuid("todo_id")
			.notNull()
			.references(() => todos.id, { onDelete: "cascade" }),
		tagId: uuid("tag_id")
			.notNull()
			.references(() => tags.id, { onDelete: "cascade" }),
	},
	(table) => [
		index("idx_todo_tags_todo").on(table.todoId),
		index("idx_todo_tags_tag").on(table.tagId),
	],
)
