export type ProcessResult = {
	output1: string;
	output2: string;
};

export type PageState = {
	selectedFile: string | null;
	outputDir: string | null;
	processing: boolean;
	result: ProcessResult | null;
	errorMsg: string | null;
};

type ActionDeps = {
	openDialog: (options: Record<string, unknown>) => Promise<string | string[] | null>;
	dirnamePath: (path: string) => Promise<string>;
	invokeCommand: <T>(
		command: string,
		args?: Record<string, unknown>
	) => Promise<T>;
};

export async function pickInputFile(state: PageState, deps: ActionDeps): Promise<void> {
	const file = await deps.openDialog({
		multiple: false,
		filters: [{ name: 'Excel Files', extensions: ['xlsx', 'xls'] }]
	});

	if (typeof file === 'string') {
		state.selectedFile = file;
		if (!state.outputDir) {
			state.outputDir = await deps.dirnamePath(file);
		}
		state.result = null;
		state.errorMsg = null;
	}
}

export async function pickOutputDir(state: PageState, deps: ActionDeps): Promise<void> {
	const dir = await deps.openDialog({ directory: true, multiple: false });
	if (typeof dir === 'string') {
		state.outputDir = dir;
		if (typeof window !== 'undefined') {
			window.localStorage.setItem('defaultOutputDir', dir);
		}
	}
}

export async function processFile(state: PageState, deps: ActionDeps): Promise<void> {
	if (!state.selectedFile || !state.outputDir) return;

	state.processing = true;
	state.result = null;
	state.errorMsg = null;

	try {
		const res = await deps.invokeCommand<ProcessResult>('process_excel_file', {
			inputPath: state.selectedFile,
			outputDir: state.outputDir
		});
		state.result = res;
	} catch (e: unknown) {
		state.errorMsg = String(e);
	} finally {
		state.processing = false;
	}
}

export function reset(state: PageState): void {
	state.selectedFile = null;
	if (typeof window !== 'undefined') {
		const savedDir = window.localStorage.getItem('defaultOutputDir');
		state.outputDir = savedDir || null;
	} else {
		state.outputDir = null;
	}
	state.result = null;
	state.errorMsg = null;
}

export function shortenPath(path: string, maxLen = 60): string {
	return path.length <= maxLen ? path : '...' + path.slice(-(maxLen - 3));
}
