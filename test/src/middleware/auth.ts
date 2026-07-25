import type { UserId } from "../types.js"

// ── Request context ──────────────────────────────────────────────────

export interface RequestContext {
	userId: UserId
	requestId: string
}

// ── Middleware types ──────────────────────────────────────────────────

export type Middleware = (ctx: RequestContext) => RequestContext | null

export function authMiddleware(ctx: RequestContext): RequestContext | null {
	// In real code: verify JWT token from headers
	if (!ctx.userId) return null
	return ctx
}

export function loggingMiddleware(ctx: RequestContext): RequestContext {
	const start = Date.now()
	// Would log: `${ctx.requestId} ${method} ${path} ${status} ${duration}ms`
	return ctx
}

// ── Rate limiter ─────────────────────────────────────────────────────

interface RateLimitEntry {
	count: number
	resetAt: number
}

const store = new Map<string, RateLimitEntry>()

export function rateLimit(
	key: string,
	limit: number,
	windowMs: number,
): boolean {
	const now = Date.now()
	const entry = store.get(key)

	if (!entry || now > entry.resetAt) {
		store.set(key, { count: 1, resetAt: now + windowMs })
		return true
	}

	if (entry.count >= limit) return false

	entry.count++
	return true
}
