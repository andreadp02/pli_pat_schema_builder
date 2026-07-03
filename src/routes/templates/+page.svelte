<script lang="ts">
	import { open as openDialog } from '@tauri-apps/plugin-dialog';
	import {
		getTemplatesStatus,
		saveTemplate,
		type TemplateKind,
		type TemplatesStatus
	} from '$lib/template-repository';
	import {
		getAccisaCoefficients,
		saveAccisaCoefficients,
		type AccisaCoefficients
	} from '$lib/settings-repository';
	import { notices } from '$lib/notifications.svelte';
	import { t } from '$lib/i18n.svelte';
	import Notice from '$lib/Notice.svelte';
	import Spinner from '$lib/Spinner.svelte';
	import { btnPrimary, inputBase } from '$lib/ui';

	const n = notices.templates;

	const TEMPLATES: { kind: TemplateKind; titleKey: 'templates.pliTitle' | 'templates.patTitle'; descKey: 'templates.pliDesc' | 'templates.patDesc' }[] = [
		{ kind: 'pli', titleKey: 'templates.pliTitle', descKey: 'templates.pliDesc' },
		{ kind: 'pat', titleKey: 'templates.patTitle', descKey: 'templates.patDesc' }
	];

	let status = $state<TemplatesStatus>({ pli: false, pat: false });
	let loading = $state(false);
	let saving = $state(false);
	let uploadingKind = $state<TemplateKind | null>(null);

	let coefficients = $state<AccisaCoefficients>({ pliPln: 0, pliPl: 0, pat: 0 });
	let loadingCoefficients = $state(false);
	let savingCoefficients = $state(false);

	const ACCISA_FIELDS: { key: keyof AccisaCoefficients; labelKey: 'templates.accisaPliPln' | 'templates.accisaPliPl' | 'templates.accisaPat' }[] = [
		{ key: 'pliPln', labelKey: 'templates.accisaPliPln' },
		{ key: 'pliPl', labelKey: 'templates.accisaPliPl' },
		{ key: 'pat', labelKey: 'templates.accisaPat' }
	];

	async function loadStatus(): Promise<void> {
		loading = true;
		try {
			status = await getTemplatesStatus();
		} catch (err) {
			n.error = String(err);
		} finally {
			loading = false;
		}
	}

	async function onUpload(kind: TemplateKind): Promise<void> {
		n.error = null;
		n.success = null;

		const selected = await openDialog({
			multiple: false,
			directory: false,
			filters: [{ name: 'Excel (.xlsx)', extensions: ['xlsx'] }]
		});

		if (!selected || Array.isArray(selected)) {
			return;
		}

		saving = true;
		uploadingKind = kind;
		try {
			await saveTemplate(kind, selected);
			n.success = t('templates.saved', { kind: kind.toUpperCase() });
			await loadStatus();
		} catch (err) {
			n.error = String(err);
		} finally {
			saving = false;
			uploadingKind = null;
		}
	}

	async function loadCoefficients(): Promise<void> {
		loadingCoefficients = true;
		try {
			coefficients = await getAccisaCoefficients();
		} catch (err) {
			n.error = String(err);
		} finally {
			loadingCoefficients = false;
		}
	}

	async function onSaveCoefficients(): Promise<void> {
		n.error = null;
		n.success = null;
		savingCoefficients = true;
		try {
			await saveAccisaCoefficients(coefficients);
			n.success = t('templates.accisaSaved');
		} catch (err) {
			n.error = String(err);
		} finally {
			savingCoefficients = false;
		}
	}

	$effect(() => {
		loadStatus();
		loadCoefficients();
	});
</script>

<div class="h-full bg-slate-50">
	<main class="mx-auto max-w-3xl px-6 py-10">
		<header class="mb-8">
			<h1 class="text-xl font-semibold tracking-tight text-slate-900">{t('templates.title')}</h1>
			<p class="mt-1 max-w-prose text-sm text-slate-500">{t('templates.intro')}</p>
		</header>

		<Notice notice={n} />

		<div class="divide-y divide-slate-200 rounded-xl border border-slate-200 bg-white shadow-sm">
			{#each TEMPLATES as template (template.kind)}
				<div class="flex items-center justify-between gap-4 p-4 md:p-5">
					<div class="min-w-0">
						<div class="flex items-center gap-2">
							<h2 class="font-semibold text-slate-900">{t(template.titleKey)}</h2>
							{#if loading}
								<span class="h-5 w-16 animate-pulse rounded-full bg-slate-100 motion-reduce:animate-none"></span>
							{:else if status[template.kind]}
								<span class="inline-flex items-center gap-1 rounded-full bg-emerald-100 px-2 py-0.5 text-xs font-medium text-emerald-700">
									<svg viewBox="0 0 20 20" class="size-3" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="M4 10.5l4 4 8-9" /></svg>
									{t('templates.loaded')}
								</span>
							{:else}
								<span class="rounded-full bg-slate-100 px-2 py-0.5 text-xs font-medium text-slate-500">
									{t('templates.notLoaded')}
								</span>
							{/if}
						</div>
						<p class="mt-1 text-sm text-slate-600">{t(template.descKey)}</p>
					</div>
					<button
						type="button"
						onclick={() => onUpload(template.kind)}
						class="{btnPrimary} shrink-0"
						disabled={saving || loading}
					>
						{#if uploadingKind === template.kind}<Spinner class="h-4 w-4" />{/if}
						{status[template.kind] ? t('templates.replace') : t('templates.upload')}
					</button>
				</div>
			{/each}
		</div>

		<section class="mt-8 rounded-xl border border-slate-200 bg-white p-5 shadow-sm">
			<h2 class="font-semibold text-slate-900">{t('templates.accisaTitle')}</h2>
			<p class="mt-1 max-w-prose text-sm text-slate-600">{t('templates.accisaIntro')}</p>

			<div class="mt-4 grid gap-4 sm:grid-cols-3">
				{#each ACCISA_FIELDS as field (field.key)}
					<label class="flex flex-col gap-1">
						<span class="text-sm font-medium text-slate-700">{t(field.labelKey)}</span>
						<input
							type="number"
							step="any"
							min="0"
							class="{inputBase} w-full"
							bind:value={coefficients[field.key]}
							disabled={loadingCoefficients || savingCoefficients}
						/>
					</label>
				{/each}
			</div>

			<div class="mt-4 flex justify-end">
				<button
					type="button"
					onclick={onSaveCoefficients}
					class={btnPrimary}
					disabled={loadingCoefficients || savingCoefficients}
				>
					{#if savingCoefficients}<Spinner class="h-4 w-4" />{/if}
					{t('common.save')}
				</button>
			</div>
		</section>
	</main>
</div>
