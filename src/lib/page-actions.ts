export type GenerateResult = {
	tracciatiPli: string;
	tracciatiPat: string;
	warnings: string[];
};

export type PageState = {
	selectedFiles: string[];
	outputDir: string | null;
	period: string;
	processing: boolean;
	result: GenerateResult | null;
	errorMsg: string | null;
};

type ActionDeps = {
	openDialog: (options: Record<string, unknown>) => Promise<string | string[] | null>;
	dirnamePath: (path: string) => Promise<string>;
	invokeCommand: <T>(command: string, args?: Record<string, unknown>) => Promise<T>;
};

export async function pickInvoiceFiles(state: PageState, deps: ActionDeps): Promise<void> {
	const files = await deps.openDialog({
		multiple: true,
		filters: [{ name: 'Excel Files', extensions: ['xlsx', 'xls'] }]
	});

	const list = Array.isArray(files) ? files : files ? [files] : [];
	if (list.length === 0) return;

	state.selectedFiles = list;
	if (!state.outputDir) {
		state.outputDir = await deps.dirnamePath(list[0]);
	}
	state.result = null;
	state.errorMsg = null;
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

export async function generate(state: PageState, deps: ActionDeps): Promise<void> {
	if (state.selectedFiles.length === 0 || !state.outputDir || !state.period.trim()) return;

	state.processing = true;
	state.result = null;
	state.errorMsg = null;

	try {
		state.result = await deps.invokeCommand<GenerateResult>('generate_tracciati', {
			invoicePaths: state.selectedFiles,
			period: state.period.trim(),
			outputDir: state.outputDir
		});
	} catch (e: unknown) {
		state.errorMsg = String(e);
	} finally {
		state.processing = false;
	}
}

export function removeFile(state: PageState, file: string): void {
	state.selectedFiles = state.selectedFiles.filter((f) => f !== file);
	state.result = null;
	state.errorMsg = null;
}

export async function openOutput(path: string, deps: ActionDeps): Promise<void> {
	await deps.invokeCommand('open_path', { path });
}

export function reset(state: PageState): void {
	state.selectedFiles = [];
	state.period = '';
	if (typeof window !== 'undefined') {
		state.outputDir = window.localStorage.getItem('defaultOutputDir') || null;
	} else {
		state.outputDir = null;
	}
	state.result = null;
	state.errorMsg = null;
}

export function shortenPath(path: string, maxLen = 60): string {
	return path.length <= maxLen ? path : '...' + path.slice(-(maxLen - 3));
}
