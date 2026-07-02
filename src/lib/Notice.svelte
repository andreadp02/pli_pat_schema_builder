<script lang="ts">
	import type { Notice } from '$lib/notifications.svelte';
	let { notice }: { notice: Notice } = $props();
</script>

{#if notice.error}
	<div class="mb-4 flex items-start gap-2 rounded-lg border border-red-200 bg-red-50 px-3 py-2 text-sm text-red-700">
		<p class="flex-1 whitespace-pre-line">{notice.error}</p>
		<button type="button" aria-label="Dismiss" class="shrink-0 text-lg leading-none text-red-400 hover:text-red-700" onclick={() => (notice.error = null)}>&times;</button>
	</div>
{/if}
{#if notice.success}
	<div class="mb-4 flex items-start gap-2 rounded-lg border border-emerald-200 bg-emerald-50 px-3 py-2 text-sm text-emerald-700">
		<p class="flex-1 whitespace-pre-line">{notice.success}</p>
		<button type="button" aria-label="Dismiss" class="shrink-0 text-lg leading-none text-emerald-500 hover:text-emerald-800" onclick={() => (notice.success = null)}>&times;</button>
	</div>
{/if}
