<script lang="ts">
	import type { Notice } from '$lib/notifications.svelte';
	import { t } from '$lib/i18n.svelte';
	let { notice }: { notice: Notice } = $props();
</script>

{#if notice.error}
	<div role="alert" class="mb-4 flex items-start gap-2 rounded-lg border border-rose-200 bg-rose-50 px-3 py-2 text-sm text-rose-700">
		<p class="max-h-60 flex-1 overflow-y-auto whitespace-pre-line">{notice.error}</p>
		<button type="button" aria-label={t('common.dismiss')} class="shrink-0 rounded text-lg leading-none text-rose-400 transition-colors hover:text-rose-700 focus:outline-none focus-visible:ring-2 focus-visible:ring-rose-400 motion-reduce:transition-none" onclick={() => (notice.error = null)}>&times;</button>
	</div>
{/if}
{#if notice.success}
	<div role="status" class="mb-4 flex items-start gap-2 rounded-lg border border-emerald-200 bg-emerald-50 px-3 py-2 text-sm text-emerald-700">
		<p class="max-h-60 flex-1 overflow-y-auto whitespace-pre-line">{notice.success}</p>
		<button type="button" aria-label={t('common.dismiss')} class="shrink-0 rounded text-lg leading-none text-emerald-500 transition-colors hover:text-emerald-800 focus:outline-none focus-visible:ring-2 focus-visible:ring-emerald-500 motion-reduce:transition-none" onclick={() => (notice.success = null)}>&times;</button>
	</div>
{/if}
{#if notice.warning}
	<div role="status" class="mb-4 flex items-start gap-2 rounded-lg border border-amber-200 bg-amber-50 px-3 py-2 text-sm text-amber-800">
		<p class="max-h-60 flex-1 overflow-y-auto whitespace-pre-line">{notice.warning}</p>
		<button type="button" aria-label={t('common.dismiss')} class="shrink-0 rounded text-lg leading-none text-amber-500 transition-colors hover:text-amber-800 focus:outline-none focus-visible:ring-2 focus-visible:ring-amber-500 motion-reduce:transition-none" onclick={() => (notice.warning = null)}>&times;</button>
	</div>
{/if}
