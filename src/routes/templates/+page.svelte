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

<div class="h-full bg-gray-50">
	<main class="mx-auto max-w-3xl px-6 py-8">
		<section class="rounded-2xl border border-slate-200 bg-white p-5 shadow-sm md:p-6">
			<div class="mb-5">
				<h2 class="text-lg font-semibold text-slate-900">{t('templates.title')}</h2>
				<p class="mt-1 text-sm text-slate-600">
					{t('templates.intro')}
				</p>
			</div>

			<Notice notice={n} />

			<div class="space-y-4">
				{#each TEMPLATES as template (template.kind)}
					<div class="flex items-center justify-between gap-4 rounded-xl border border-slate-200 p-4">
						<div class="min-w-0">
							<div class="flex items-center gap-2">
								<h3 class="font-semibold text-slate-900">{t(template.titleKey)}</h3>
								{#if status[template.kind]}
									<span class="rounded-full bg-emerald-100 px-2 py-0.5 text-xs font-medium text-emerald-700">
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
							class="inline-flex shrink-0 items-center gap-2 rounded-lg bg-emerald-700 px-4 py-2 text-sm font-medium text-white hover:bg-emerald-800 disabled:opacity-50"
							disabled={saving || loading}
						>
							{#if uploadingKind === template.kind}<Spinner class="h-4 w-4" />{/if}
							{status[template.kind] ? t('templates.replace') : t('templates.upload')}
						</button>
					</div>
				{/each}
			</div>
		</section>
	</main>
</div>
