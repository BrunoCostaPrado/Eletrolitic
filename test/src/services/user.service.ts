import { eq } from "drizzle-orm"
import { db } from "../db/client.js"
import { users } from "../db/schema.js"
import { createUserSchema } from "../db/validation.js"

export class NotFoundError extends Error {
	constructor(resource: string, id: string) {
		super(`${resource} ${id} not found`)
		this.name = "NotFoundError"
	}
}

export async function getUserById(id: string) {
	const [user] = await db.select().from(users).where(eq(users.id, id)).limit(1)
	return user ?? null
}

export async function getUserByEmail(email: string) {
	const [user] = await db
		.select()
		.from(users)
		.where(eq(users.email, email))
		.limit(1)
	return user ?? null
}

export async function createUser(input: {
	email: string
	name: string
	avatarUrl?: string | null
}) {
	const parsed = createUserSchema.parse(input)
	const [user] = await db
		.insert(users)
		.values({
			email: parsed.email,
			name: parsed.name,
			avatarUrl: parsed.avatarUrl ?? null,
		})
		.returning()
	return user
}

export async function updateUser(
	id: string,
	data: { name?: string; avatarUrl?: string | null },
) {
	const existing = await getUserById(id)
	if (!existing) throw new NotFoundError("User", id)

	const updateData: Record<string, unknown> = { updatedAt: new Date() }
	if (data.name !== undefined) updateData.name = data.name
	if (data.avatarUrl !== undefined) updateData.avatarUrl = data.avatarUrl

	await db.update(users).set(updateData).where(eq(users.id, id))
	return getUserById(id)
}

export async function listUsers() {
	return db.select().from(users)
}
