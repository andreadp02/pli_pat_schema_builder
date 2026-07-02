<script lang="ts">
	import { onMount, type Snippet } from 'svelte';
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
	import packageJson from '../../package.json';

	const appVersion = packageJson.version;
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

	const step1Done = $derived(state.selectedFiles.length > 0);
	const step2Done = $derived(!!state.fortnightEnd);
	const step3Done = $derived(!!state.outputDir);
	const canGenerate = $derived(step1Done && step2Done && step3Done && !state.processing);
	const currentStep = $derived(!step1Done ? 1 : !step2Done ? 2 : !step3Done ? 3 : 4);

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

{#snippet badge(num: number, done: boolean, current: boolean)}
	<span
		class={`grid size-8 shrink-0 place-items-center rounded-full border text-sm font-semibold transition-colors duration-200 motion-reduce:transition-none ${
			done
				? 'border-emerald-500 bg-emerald-500 text-white'
				: current
					? 'border-slate-900 bg-white text-slate-900 ring-4 ring-slate-900/5'
					: 'border-slate-200 bg-white text-slate-400'
		}`}
	>
		{#if done}
			<svg viewBox="0 0 20 20" class="size-4" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="M4 10.5l4 4 8-9" /></svg>
		{:else}
			{num}
		{/if}
	</span>
{/snippet}

{#snippet step(num: number, title: string, done: boolean, current: boolean, last: boolean, body: Snippet)}
	<div class="flex gap-4 sm:gap-5">
		<div class="flex flex-col items-center">
			{@render badge(num, done, current)}
			{#if !last}
				<div class={`mt-1.5 w-px flex-1 transition-colors duration-200 motion-reduce:transition-none ${done ? 'bg-emerald-400' : 'bg-slate-200'}`}></div>
			{/if}
		</div>
		<div class={`min-w-0 flex-1 ${last ? '' : 'pb-8'}`}>
			<h2 class="mb-3 text-sm font-semibold tracking-tight text-slate-900">{title}</h2>
			{@render body()}
		</div>
	</div>
{/snippet}

<div class="flex h-full flex-col bg-slate-50">
	<main class="mx-auto w-full max-w-2xl flex-1 px-6 py-10">
		<header class="mb-8">
			<h1 class="text-xl font-semibold tracking-tight text-slate-900">{t('home.pageTitle')}</h1>
			<p class="mt-1 max-w-prose text-sm text-slate-500">{t('home.pageSubtitle')}</p>
		</header>

		{@render step(1, t('home.step1'), step1Done, currentStep === 1, false, step1Body)}
		{@render step(2, t('home.step2'), false, false, false, step2Body)}
		{@render step(3, t('home.step3'), step3Done, currentStep === 3, false, step3Body)}
		{@render step(4, t('home.step4'), !!state.result, currentStep === 4, true, step4Body)}
	</main>

	<footer class="py-4 text-center text-xs text-slate-500">PLI PAT Schema Builder &middot; v{appVersion}</footer>
</div>

{#snippet step1Body()}
	<button
		onclick={onPickInvoiceFiles}
		class="inline-flex items-center gap-2 rounded-lg border border-slate-300 bg-white px-4 py-2 text-sm font-medium text-slate-700 shadow-sm transition-colors hover:border-slate-400 hover:bg-slate-50 focus:outline-none focus-visible:ring-2 focus-visible:ring-slate-400"
	>
		<svg viewBox="0 0 20 20" class="size-4 text-slate-400" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"><path d="M4 13v2a1 1 0 001 1h10a1 1 0 001-1v-2M10 4v9m0-9L6.5 7.5M10 4l3.5 3.5" /></svg>
		{t('home.chooseInvoices')}
	</button>

	{#if state.selectedFiles.length > 0}
		<div class="mt-3 space-y-1.5">
			<p class="text-xs font-medium text-slate-500">{t('home.selected', { n: state.selectedFiles.length })}</p>
			{#each state.selectedFiles as file}
				<div class="flex items-center gap-2 rounded-lg border border-slate-200 bg-white px-3 py-2 text-sm text-slate-600">
					<span class="flex-1 truncate font-mono text-xs" title={file}>{shortenPath(file)}</span>
					<button
						onclick={() => onRemoveFile(file)}
						title={t('home.remove')}
						aria-label={t('home.removeFile')}
						class="shrink-0 rounded p-0.5 text-slate-400 transition-colors hover:text-rose-600 focus:outline-none focus-visible:ring-2 focus-visible:ring-rose-400"
					>
						<svg viewBox="0 0 20 20" class="size-4" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"><path d="M6 6l8 8M14 6l-8 8" /></svg>
					</button>
				</div>
			{/each}
		</div>
	{/if}
{/snippet}

{#snippet step2Body()}
	<select
		bind:value={state.fortnightEnd}
		class="w-full rounded-lg border border-slate-300 bg-white px-3 py-2 text-sm text-slate-800 shadow-sm transition-colors hover:border-slate-400 focus:border-slate-500 focus:outline-none focus-visible:ring-2 focus-visible:ring-slate-400"
	>
		{#each fortnights as f}
			<option value={f.value}>{f.label}</option>
		{/each}
	</select>
	<p class="mt-2 text-xs text-slate-500">{t('home.fortnightHint')}</p>
{/snippet}

{#snippet step3Body()}
	<button
		onclick={() => pickOutputDir(state, deps)}
		class="inline-flex items-center gap-2 rounded-lg border border-slate-300 bg-white px-4 py-2 text-sm font-medium text-slate-700 shadow-sm transition-colors hover:border-slate-400 hover:bg-slate-50 focus:outline-none focus-visible:ring-2 focus-visible:ring-slate-400"
	>
		<svg viewBox="0 0 20 20" class="size-4 text-slate-400" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"><path d="M3 6a1 1 0 011-1h3.5l1.5 1.5H16a1 1 0 011 1V15a1 1 0 01-1 1H4a1 1 0 01-1-1V6z" /></svg>
		{t('home.chooseFolder')}
	</button>

	{#if state.outputDir}
		<div class="mt-3 flex items-center gap-2 rounded-lg border border-slate-200 bg-white px-3 py-2 text-sm text-slate-600">
			<span class="flex-1 truncate font-mono text-xs" title={state.outputDir}>{shortenPath(state.outputDir)}</span>
			<button
				onclick={() => openOutput(state.outputDir ?? '', deps)}
				title={t('home.openTitle', { path: state.outputDir })}
				aria-label={t('home.openTitle', { path: state.outputDir })}
				class="shrink-0 rounded px-1.5 py-0.5 text-xs font-medium text-slate-500 transition-colors hover:bg-slate-100 hover:text-slate-900 focus:outline-none focus-visible:ring-2 focus-visible:ring-slate-400"
			>
				{t('home.open')}
			</button>
		</div>
	{/if}
{/snippet}

{#snippet step4Body()}
	<div class="flex flex-wrap items-center gap-3">
		<button
			onclick={onGenerate}
			disabled={!canGenerate}
			class="inline-flex items-center gap-2 rounded-lg bg-slate-900 px-5 py-2.5 text-sm font-semibold text-white shadow-sm transition-colors hover:bg-slate-800 focus:outline-none focus-visible:ring-2 focus-visible:ring-slate-900 focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-40 disabled:hover:bg-slate-900"
		>
			{#if state.processing}<Spinner class="h-4 w-4" />{/if}
			{state.processing ? t('home.generating') : t('home.generate')}
		</button>

		{#if state.result || n.error || n.warning}
			<button
				onclick={onReset}
				class="rounded-lg border border-slate-300 bg-white px-4 py-2.5 text-sm font-medium text-slate-600 transition-colors hover:bg-slate-50 focus:outline-none focus-visible:ring-2 focus-visible:ring-slate-400"
			>
				{t('common.reset')}
			</button>
		{/if}
	</div>

	<div class="mt-4"><Notice notice={n} /></div>

	{#if state.result}
		<div class="space-y-2">
			<p class="text-sm font-medium text-emerald-700">{t('home.filesGenerated')}</p>
			{#each [{ label: 'tracciati_pli', path: state.result.tracciatiPli }, { label: 'tracciati_pat', path: state.result.tracciatiPat }] as file}
				<button
					onclick={() => openOutput(file.path, deps)}
					title={t('home.openTitle', { path: file.path })}
					class="flex w-full items-center gap-3 rounded-lg border border-emerald-200 bg-emerald-50 px-3 py-2.5 text-left text-sm transition-colors hover:bg-emerald-100 focus:outline-none focus-visible:ring-2 focus-visible:ring-emerald-500"
				>
					<span class="w-28 shrink-0 font-medium text-slate-700">{file.label}</span>
					<span class="flex-1 truncate font-mono text-xs text-slate-600">{shortenPath(file.path)}</span>
					<span class="shrink-0 text-xs font-medium text-emerald-700">{t('home.open')}</span>
				</button>
			{/each}
		</div>
	{/if}
{/snippet}
