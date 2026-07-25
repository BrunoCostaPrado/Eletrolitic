import type { ApiResponse } from "./types.js"

export function success<T>(data: T): ApiResponse<T> {
	return {
		success: true,
		data,
		meta: {
			timestamp: new Date().toISOString(),
			requestId: crypto.randomUUID(),
		},
	}
}
