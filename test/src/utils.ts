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

export function error(message: string): ApiResponse<never> {
	return {
		success: false,
		error: message,
		meta: {
			timestamp: new Date().toISOString(),
			requestId: crypto.randomUUID(),
		},
	}
}
