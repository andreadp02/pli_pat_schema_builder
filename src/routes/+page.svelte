<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';
	import { open } from '@tauri-apps/plugin-dialog';
	import { dirname } from '@tauri-apps/api/path';
	import {
		pickInputFile,
		pickOutputDir,
		processFile,
		reset,
		shortenPath,
		type PageState
	} from '$lib/page-actions';

	let state = $state<PageState>({
		selectedFile: null,
		outputDir: null,
		processing: false,
		result: null,
		errorMsg: null
	});

	const deps = {
		openDialog: open,
		dirnamePath: dirname,
		invokeCommand: invoke
	};
</script>

<div class="h-full bg-gray-50 flex flex-col">
	<!-- Main content -->
	<main class="flex-1 max-w-3xl w-full mx-auto px-6 py-10 space-y-8">
		<!-- Step 1 – Select input file -->
		<section class="bg-white rounded-2xl shadow p-6 space-y-4">
			<h2 class="text-lg font-semibold text-gray-800">1. Select Excel Input File</h2>

			<button
				onclick={() => pickInputFile(state, deps)}
				class="inline-flex items-center gap-2 px-5 py-2.5 rounded-lg bg-blue-600 text-white font-medium
				       hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2
				       transition-colors"
			>
				<svg
					xmlns="http://www.w3.org/2000/svg"
					class="h-5 w-5"
					fill="none"
					viewBox="0 0 24 24"
					stroke="currentColor"
					stroke-width="2"
				>
					<path
						stroke-linecap="round"
						stroke-linejoin="round"
						d="M4 16v2a2 2 0 002 2h12a2 2 0 002-2v-2M12 4v12m-4-4l4 4 4-4"
					/>
				</svg>
				Choose .xlsx file
			</button>

			{#if state.selectedFile}
				<div class="flex items-center gap-2 text-sm text-gray-600 bg-gray-50 rounded-lg px-4 py-2.5">
					<svg
						xmlns="http://www.w3.org/2000/svg"
						class="h-4 w-4 text-green-600 shrink-0"
						fill="none"
						viewBox="0 0 24 24"
						stroke="currentColor"
						stroke-width="2"
					>
						<path stroke-linecap="round" stroke-linejoin="round" d="M5 13l4 4L19 7" />
					</svg>
					<span class="truncate font-mono" title={state.selectedFile}>{shortenPath(state.selectedFile)}</span>
				</div>
			{/if}
		</section>

		<!-- Step 2 – Select output directory -->
		<section class="bg-white rounded-2xl shadow p-6 space-y-4">
			<h2 class="text-lg font-semibold text-gray-800">2. Select Output Directory</h2>

			<button
				onclick={() => pickOutputDir(state, deps)}
				class="inline-flex items-center gap-2 px-5 py-2.5 rounded-lg bg-gray-200 text-gray-800 font-medium
				       hover:bg-gray-300 focus:outline-none focus:ring-2 focus:ring-gray-400 focus:ring-offset-2
				       transition-colors"
			>
				<svg
					xmlns="http://www.w3.org/2000/svg"
					class="h-5 w-5"
					fill="none"
					viewBox="0 0 24 24"
					stroke="currentColor"
					stroke-width="2"
				>
					<path
						stroke-linecap="round"
						stroke-linejoin="round"
						d="M3 7a2 2 0 012-2h4l2 2h8a2 2 0 012 2v9a2 2 0 01-2 2H5a2 2 0 01-2-2V7z"
					/>
				</svg>
				Choose folder
			</button>

			{#if state.outputDir}
				<div class="flex items-center gap-2 text-sm text-gray-600 bg-gray-50 rounded-lg px-4 py-2.5">
					<svg
						xmlns="http://www.w3.org/2000/svg"
						class="h-4 w-4 text-blue-600 shrink-0"
						fill="none"
						viewBox="0 0 24 24"
						stroke="currentColor"
						stroke-width="2"
					>
						<path stroke-linecap="round" stroke-linejoin="round" d="M5 13l4 4L19 7" />
					</svg>
					<span class="truncate font-mono" title={state.outputDir}>{shortenPath(state.outputDir)}</span>
				</div>
			{/if}
		</section>

		<!-- Step 3 – Process -->
		<section class="bg-white rounded-2xl shadow p-6 space-y-4">
			<h2 class="text-lg font-semibold text-gray-800">3. Process</h2>

			<div class="flex gap-3">
				<button
					onclick={() => processFile(state, deps)}
					disabled={!state.selectedFile || !state.outputDir || state.processing}
					class="inline-flex items-center gap-2 px-6 py-2.5 rounded-lg bg-green-600 text-white font-semibold
					       hover:bg-green-700 focus:outline-none focus:ring-2 focus:ring-green-500 focus:ring-offset-2
					       disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
				>
					{#if state.processing}
						<svg
							class="animate-spin h-5 w-5"
							xmlns="http://www.w3.org/2000/svg"
							fill="none"
							viewBox="0 0 24 24"
						>
							<circle
								class="opacity-25"
								cx="12"
								cy="12"
								r="10"
								stroke="currentColor"
								stroke-width="4"
							></circle>
							<path
								class="opacity-75"
								fill="currentColor"
								d="M4 12a8 8 0 018-8v4l3-3-3-3v4a8 8 0 100 16v-4l-3 3 3 3v-4a8 8 0 01-8-8z"
							></path>
						</svg>
						Processing…
					{:else}
						<svg
							xmlns="http://www.w3.org/2000/svg"
							class="h-5 w-5"
							fill="none"
							viewBox="0 0 24 24"
							stroke="currentColor"
							stroke-width="2"
						>
							<path stroke-linecap="round" stroke-linejoin="round" d="M14.752 11.168l-4.586-2.651A1 1 0 009 9.384v5.232a1 1 0 001.166.983l4.586-1.05a1 1 0 00.814-.983v-2.4z" />
						</svg>
						Generate Output Files
					{/if}
				</button>

				{#if state.result || state.errorMsg}
					<button
						onclick={() => reset(state)}
						class="px-4 py-2.5 rounded-lg border border-gray-300 text-gray-600 font-medium
						       hover:bg-gray-50 focus:outline-none focus:ring-2 focus:ring-gray-400 transition-colors"
					>
						Reset
					</button>
				{/if}
			</div>

			<!-- Error -->
			{#if state.errorMsg}
				<div
					class="flex items-start gap-3 bg-red-50 border border-red-200 text-red-700 rounded-lg px-4 py-3 text-sm"
				>
					<svg
						xmlns="http://www.w3.org/2000/svg"
						class="h-5 w-5 mt-0.5 shrink-0"
						fill="none"
						viewBox="0 0 24 24"
						stroke="currentColor"
						stroke-width="2"
					>
						<path
							stroke-linecap="round"
							stroke-linejoin="round"
							d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
						/>
					</svg>
					<span>{state.errorMsg}</span>
				</div>
			{/if}

			<!-- Success -->
			{#if state.result}
				<div class="space-y-3">
					<div class="flex items-center gap-2 text-green-700 font-medium">
						<svg
							xmlns="http://www.w3.org/2000/svg"
							class="h-5 w-5"
							fill="none"
							viewBox="0 0 24 24"
							stroke="currentColor"
							stroke-width="2"
						>
							<path stroke-linecap="round" stroke-linejoin="round" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
						</svg>
						Files generated successfully!
					</div>

					<div class="space-y-2 text-sm">
						{#each [{ label: 'Output 1', path: state.result.output1 }, { label: 'Output 2', path: state.result.output2 }] as file}
							<div class="flex items-center gap-3 bg-green-50 border border-green-200 rounded-lg px-4 py-2.5">
								<svg
									xmlns="http://www.w3.org/2000/svg"
									class="h-4 w-4 text-green-600 shrink-0"
									fill="none"
									viewBox="0 0 24 24"
									stroke="currentColor"
									stroke-width="2"
								>
									<path
										stroke-linecap="round"
										stroke-linejoin="round"
										d="M9 12h6m-6 4h6m2 4H7a2 2 0 01-2-2V6a2 2 0 012-2h5l2 2h5a2 2 0 012 2v11a2 2 0 01-2 2z"
									/>
								</svg>
								<span class="font-medium text-gray-700 w-20 shrink-0">{file.label}</span>
								<span class="truncate font-mono text-gray-600" title={file.path}>
									{shortenPath(file.path)}
								</span>
							</div>
						{/each}
					</div>
				</div>
			{/if}
		</section>
	</main>

	<!-- Footer -->
	<footer class="text-center text-xs text-gray-400 py-4">
		PLI PAT Schema Builder &middot; v0.1.0
	</footer>
</div>

