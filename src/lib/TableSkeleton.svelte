<script lang="ts">
	let { rows = 8, cols }: { rows?: number; cols: number } = $props();
</script>

{#each Array(rows) as _}
	<tr class="border-b border-slate-100">
		{#each Array(cols) as _}
			<td class="px-3 py-3.5">
				<div class="h-3.5 w-full max-w-32 animate-pulse rounded bg-slate-100 motion-reduce:animate-none"></div>
			</td>
		{/each}
	</tr>
{/each}
