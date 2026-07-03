<script lang="ts">
	import { open as openDialog } from '@tauri-apps/plugin-dialog';
	import {
		getTemplatesStatus,
		saveTemplate,
		type TemplateKind,
		type TemplatesStatus
	} from '$lib/template-repository';
	import { notices } from '$lib/notifications.svelte';
	import { t } from '$lib/i18n.svelte';
	import Notice from '$lib/Notice.svelte';
	import Spinner from '$lib/Spinner.svelte';
	import { btnPrimary } from '$lib/ui';

	const n = notices.templates;

	const TEMPLATES: { kind: TemplateKind; titleKey: 'templates.pliTitle' | 'templates.patTitle'; descKey: 'templates.pliDesc' | 'templates.patDesc' }[] = [
		{ kind: 'pli', titleKey: 'templates.pliTitle', descKey: 'templates.pliDesc' },
		{ kind: 'pat', titleKey: 'templates.patTitle', descKey: 'templates.patDesc' }
	];

	let status = $state<TemplatesStatus>({ pli: false, pat: false });
	let loading = $state(false);
	let saving = $state(false);
	let uploadingKind = $state<TemplateKind | null>(null);

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

	$effect(() => {
		loadStatus();
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
	</main>
</div>
