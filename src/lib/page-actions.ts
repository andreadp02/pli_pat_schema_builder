export type GenerateResult = {
	tracciatiPli: string;
	tracciatiPat: string;
	warnings: string[];
};

export type PageState = {
	selectedFiles: string[];
	outputDir: string | null;
	fortnightEnd: string; // ISO "YYYY-MM-DD" of the selected fortnight end date
	processing: boolean;
	result: GenerateResult | null;
	errorMsg: string | null;
};

export type Fortnight = { value: string; label: string }; // value = ISO end date

const pad = (n: number) => String(n).padStart(2, '0');
const toIso = (d: Date) => `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;

/** Fortnight end dates (15th + last day of month) from 6 months back to 6 months ahead. */
export function fortnightOptions(today = new Date()): Fortnight[] {
	const y = today.getFullYear();
	const m = today.getMonth();
	const dates: Date[] = [];
	for (let i = -6; i <= 6; i++) {
		dates.push(new Date(y, m + i, 15));
		dates.push(new Date(y, m + i + 1, 0)); // last day of month (m+i)
	}
	return dates
		.sort((a, b) => a.getTime() - b.getTime())
		.map((d) => {
			const iso = toIso(d);
			const [yy, mm, dd] = iso.split('-');
			return { value: iso, label: `${dd}/${mm}/${yy}` };
		});
}

/** Next fortnight end on or after today, falling back to the last option. */
export function defaultFortnight(options: Fortnight[], today = new Date()): string {
	const iso = toIso(today);
	return (options.find((o) => o.value >= iso) ?? options[options.length - 1]).value;
}

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
	if (state.selectedFiles.length === 0 || !state.outputDir || !state.fortnightEnd) return;

	state.processing = true;
	state.result = null;
	state.errorMsg = null;

	try {
		state.result = await deps.invokeCommand<GenerateResult>('generate_tracciati', {
			invoicePaths: state.selectedFiles,
			fortnightEnd: state.fortnightEnd,
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
	state.fortnightEnd = defaultFortnight(fortnightOptions());
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
