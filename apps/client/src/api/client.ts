import { QueryClient } from '@tanstack/react-query';

const client = new QueryClient({
	defaultOptions: {
		queries: {
			retry: false,
			refetchOnWindowFocus: false,
			suspense: true,
		},
	},
});

export default client;
