<script lang="ts">
	import { onMount } from 'svelte';
	import { invoke } from '@tauri-apps/api/core';
	import { open } from '@tauri-apps/plugin-dialog';
	import { dirname } from '@tauri-apps/api/path';
	import {
		pickInvoiceFiles,
		pickOutputDir,
		generate,
		removeFile,
		openOutput,
		reset,
		shortenPath,
		fortnightOptions,
		defaultFortnight,
		type PageState
	} from '$lib/page-actions';
	import { notices } from '$lib/notifications.svelte';
	import { t } from '$lib/i18n.svelte';
	import Spinner from '$lib/Spinner.svelte';
	import Notice from '$lib/Notice.svelte';

	const n = notices.home;
	const fortnights = fortnightOptions();

	let state = $state<PageState>({
		selectedFiles: [],
		outputDir: null,
		fortnightEnd: defaultFortnight(fortnights),
		processing: false,
		result: null,
		errorMsg: null
	});

	onMount(() => {
		const savedDir = window.localStorage.getItem('defaultOutputDir');
		if (savedDir) {
			state.outputDir = savedDir;
		}
	});

	const deps = {
		openDialog: open,
		dirnamePath: dirname,
		invokeCommand: invoke
	};

	async function onPickInvoiceFiles(): Promise<void> {
		await pickInvoiceFiles(state, deps);
		n.error = null;
		n.warning = null;
	}

	async function onGenerate(): Promise<void> {
		await generate(state, deps);
		n.error = state.errorMsg;
		const warnings = state.result?.warnings ?? [];
		n.warning = warnings.length
			? [t('home.warnings', { n: warnings.length }), ...warnings.map((w) => `• ${w}`)].join('\n')
			: null;
	}

	function onRemoveFile(file: string): void {
		removeFile(state, file);
		n.error = null;
		n.warning = null;
	}

	function onReset(): void {
		reset(state);
		n.error = null;
		n.warning = null;
	}
</script>

<div class="h-full bg-gray-50 flex flex-col">
	<main class="flex-1 max-w-3xl w-full mx-auto px-6 py-10 space-y-8">
		<!-- Step 1 – Select invoices -->
		<section class="bg-white rounded-2xl shadow p-6 space-y-4">
			<h2 class="text-lg font-semibold text-gray-800">{t('home.step1')}</h2>

			<button
				onclick={onPickInvoiceFiles}
				class="inline-flex items-center gap-2 px-5 py-2.5 rounded-lg bg-blue-600 text-white font-medium
				       hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 transition-colors"
			>
				{t('home.chooseInvoices')}
			</button>

			{#if state.selectedFiles.length > 0}
				<div class="space-y-1.5">
					<p class="text-sm font-medium text-gray-600">{t('home.selected', { n: state.selectedFiles.length })}</p>
					{#each state.selectedFiles as file}
						<div class="flex items-center gap-2 text-sm text-gray-600 bg-gray-50 rounded-lg px-4 py-2">
							<span class="truncate font-mono flex-1" title={file}>{shortenPath(file)}</span>
							<button
								onclick={() => onRemoveFile(file)}
								title={t('home.remove')}
								aria-label={t('home.removeFile')}
								class="shrink-0 text-gray-400 hover:text-red-600 focus:outline-none text-lg leading-none px-1"
							>
								&times;
							</button>
						</div>
					{/each}
				</div>
			{/if}
		</section>

		<!-- Step 2 – Fortnight end date -->
		<section class="bg-white rounded-2xl shadow p-6 space-y-4">
			<h2 class="text-lg font-semibold text-gray-800">{t('home.step2')}</h2>
			<select
				bind:value={state.fortnightEnd}
				class="w-full rounded-lg border border-gray-300 px-4 py-2.5 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
			>
				{#each fortnights as f}
					<option value={f.value}>{f.label}</option>
				{/each}
			</select>
			<p class="text-xs text-gray-400">
				{t('home.fortnightHint')}
			</p>
		</section>

		<!-- Step 3 – Output directory -->
		<section class="bg-white rounded-2xl shadow p-6 space-y-4">
			<h2 class="text-lg font-semibold text-gray-800">{t('home.step3')}</h2>

			<button
				onclick={() => pickOutputDir(state, deps)}
				class="inline-flex items-center gap-2 px-5 py-2.5 rounded-lg bg-gray-200 text-gray-800 font-medium
				       hover:bg-gray-300 focus:outline-none focus:ring-2 focus:ring-gray-400 focus:ring-offset-2 transition-colors"
			>
				{t('home.chooseFolder')}
			</button>

			{#if state.outputDir}
				<div class="flex items-center gap-2 text-sm text-gray-600 bg-gray-50 rounded-lg px-4 py-2.5">
					<span class="truncate font-mono flex-1" title={state.outputDir}>{shortenPath(state.outputDir)}</span>
					<button
						onclick={() => openOutput(state.outputDir ?? '', deps)}
						title={t('home.openTitle', { path: state.outputDir })}
						aria-label={t('home.openTitle', { path: state.outputDir })}
						class="shrink-0 text-gray-400 hover:text-blue-600 focus:outline-none"
					>
						{t('home.open')}
					</button>
				</div>
			{/if}
		</section>

		<!-- Step 4 – Generate -->
		<section class="bg-white rounded-2xl shadow p-6 space-y-4">
			<h2 class="text-lg font-semibold text-gray-800">{t('home.step4')}</h2>

			<div class="flex gap-3">
				<button
					onclick={onGenerate}
					disabled={state.selectedFiles.length === 0 || !state.outputDir || !state.fortnightEnd || state.processing}
					class="inline-flex items-center gap-2 px-6 py-2.5 rounded-lg bg-green-600 text-white font-semibold
					       hover:bg-green-700 focus:outline-none focus:ring-2 focus:ring-green-500 focus:ring-offset-2
					       disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
				>
					{#if state.processing}<Spinner class="h-4 w-4" />{/if}
					{state.processing ? t('home.generating') : t('home.generate')}
				</button>

				{#if state.result || n.error || n.warning}
					<button
						onclick={onReset}
						class="px-4 py-2.5 rounded-lg border border-gray-300 text-gray-600 font-medium
						       hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-gray-400 transition-colors"
					>
						{t('common.reset')}
					</button>
				{/if}
			</div>

			<Notice notice={n} />

			{#if state.result}
				<div class="space-y-3">
					<div class="text-green-700 font-medium">{t('home.filesGenerated')}</div>

					<div class="space-y-2 text-sm">
						{#each [{ label: 'tracciati_pli', path: state.result.tracciatiPli }, { label: 'tracciati_pat', path: state.result.tracciatiPat }] as file}
							<button
								onclick={() => openOutput(file.path, deps)}
								title={t('home.openTitle', { path: file.path })}
								class="w-full flex items-center gap-3 bg-green-50 border border-green-200 rounded-lg px-4 py-2.5 text-left hover:bg-green-100 focus:outline-none focus:ring-2 focus:ring-green-500 transition-colors"
							>
								<span class="font-medium text-gray-700 w-28 shrink-0">{file.label}</span>
								<span class="truncate font-mono text-gray-600 flex-1">{shortenPath(file.path)}</span>
								<span class="shrink-0 text-green-700 text-xs font-medium">{t('home.open')}</span>
							</button>
						{/each}
					</div>
				</div>
			{/if}
		</section>
	</main>

	<footer class="text-center text-xs text-gray-400 py-4">PLI PAT Schema Builder &middot; v0.1.0</footer>
</div>
