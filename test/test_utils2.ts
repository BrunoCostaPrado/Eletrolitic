export function success(data: string): any {
	return {
		success: true,
		data,
		meta: {
			timestamp: 1,
			requestId: 2,
		},
	}
}
