// ── Core domain types ────────────────────────────────────────────────

export type UserId = string
export type TodoId = string
export type TagId = string

export interface PaginatedResult<T> {
	data: T[]
	total: number
	page: number
	limit: number
	hasNext: boolean
}

export interface ApiResponse<T> {
	success: boolean
	data?: T
	error?: string
	meta?: {
		timestamp: string
		requestId: string
	}
}
