import { Box, Text } from '@chakra-ui/react';
import toast from 'react-hot-toast';
import { useMutation, useQuery } from '@tanstack/react-query';
import client from '~api/client';
import { clearLogFile } from '~api/log';
import { getLogFileMeta } from '~api/log';
import Button from '~ui/Button';
import { formatBytes } from '~util/format';

export function LogStats() {
	const { data: logMeta } = useQuery(['getLogFileMeta'], () =>
		getLogFileMeta().then((res) => res.data),
	);

	const { mutateAsync } = useMutation(['clearStumpLogs'], clearLogFile);

	function handleClearLogs() {
		toast
			.promise(mutateAsync(), {
				loading: 'Clearing...',
				success: 'Cleared logs!',
				error: 'Error clearing logs.',
			})
			.then(() => client.invalidateQueries(['getLogFileMeta']));
	}

	return (
		<Box>
			<Text>{formatBytes(logMeta?.size)}</Text>
			<Button onClick={handleClearLogs}>Delete logs</Button>
		</Box>
	);
}

export default function ServerStats() {
	return (
		<div>
			<LogStats />
		</div>
	);
}
