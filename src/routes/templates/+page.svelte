<script lang="ts">
	import { open as openDialog } from '@tauri-apps/plugin-dialog';
	import {
		getTemplatesStatus,
		saveTemplate,
		type TemplateKind,
		type TemplatesStatus
	} from '$lib/template-repository';

	const TEMPLATES: { kind: TemplateKind; title: string; description: string }[] = [
		{
			kind: 'pli',
			title: 'Tracciato PLI',
			description: 'Modello ADM per i Prodotti Liquidi da Inalazione (tracciati_pli.xlsx).'
		},
		{
			kind: 'pat',
			title: 'Tracciato PAT',
			description: 'Modello ADM per i Prodotti Accessori dei Tabacchi (tracciati_pat.xlsx).'
		}
	];

	let status = $state<TemplatesStatus>({ pli: false, pat: false });
	let loading = $state(false);
	let saving = $state(false);
	let errorMsg = $state<string | null>(null);
	let successMsg = $state<string | null>(null);

	async function loadStatus(): Promise<void> {
		loading = true;
		errorMsg = null;
		try {
			status = await getTemplatesStatus();
		} catch (err) {
			errorMsg = String(err);
		} finally {
			loading = false;
		}
	}

	async function onUpload(kind: TemplateKind): Promise<void> {
		errorMsg = null;
		successMsg = null;

		const selected = await openDialog({
			multiple: false,
			directory: false,
			filters: [{ name: 'Excel (.xlsx)', extensions: ['xlsx'] }]
		});

		if (!selected || Array.isArray(selected)) {
			return;
		}

		saving = true;
		try {
			await saveTemplate(kind, selected);
			successMsg = `Modello ${kind.toUpperCase()} salvato.`;
			await loadStatus();
		} catch (err) {
			errorMsg = String(err);
		} finally {
			saving = false;
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
				<h2 class="text-lg font-semibold text-slate-900">Modelli ADM</h2>
				<p class="mt-1 text-sm text-slate-600">
					Carica i due modelli Excel usati per generare i tracciati. Vengono salvati nei dati
					dell'app e riutilizzati ad ogni generazione.
				</p>
			</div>

			{#if errorMsg}
				<div class="mb-4 rounded-lg border border-rose-200 bg-rose-50 px-4 py-3 text-sm text-rose-700">
					{errorMsg}
				</div>
			{/if}
			{#if successMsg}
				<div class="mb-4 rounded-lg border border-emerald-200 bg-emerald-50 px-4 py-3 text-sm text-emerald-700">
					{successMsg}
				</div>
			{/if}

			<div class="space-y-4">
				{#each TEMPLATES as template (template.kind)}
					<div class="flex items-center justify-between gap-4 rounded-xl border border-slate-200 p-4">
						<div class="min-w-0">
							<div class="flex items-center gap-2">
								<h3 class="font-semibold text-slate-900">{template.title}</h3>
								{#if status[template.kind]}
									<span class="rounded-full bg-emerald-100 px-2 py-0.5 text-xs font-medium text-emerald-700">
										Caricato
									</span>
								{:else}
									<span class="rounded-full bg-slate-100 px-2 py-0.5 text-xs font-medium text-slate-500">
										Non caricato
									</span>
								{/if}
							</div>
							<p class="mt-1 text-sm text-slate-600">{template.description}</p>
						</div>
						<button
							type="button"
							onclick={() => onUpload(template.kind)}
							class="shrink-0 rounded-lg bg-emerald-700 px-4 py-2 text-sm font-medium text-white hover:bg-emerald-800 disabled:opacity-50"
							disabled={saving || loading}
						>
							{status[template.kind] ? 'Sostituisci' : 'Carica'}
						</button>
					</div>
				{/each}
			</div>
		</section>
	</main>
</div>
