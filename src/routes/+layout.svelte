<script lang="ts">
	import { page } from '$app/state';
	import { invoke } from '@tauri-apps/api/core';
	import { t, getLocale, setLocale } from '$lib/i18n.svelte';
	import '../app.css';
	import favicon from '$lib/assets/favicon.svg';

	let { children } = $props();

	async function minimizeWindow(): Promise<void> {
		await invoke('window_minimize');
	}

	async function toggleMaximize(): Promise<void> {
		await invoke('window_toggle_maximize');
	}

	async function closeWindow(): Promise<void> {
		await invoke('window_close');
	}

	async function dragWindow(event: MouseEvent): Promise<void> {
		if (event.button !== 0) {
			return;
		}

		const target = event.target;
		if (!(target instanceof HTMLElement)) {
			return;
		}

		if (target.closest('[data-no-window-drag]')) {
			return;
		}

		await invoke('window_start_dragging');
	}
</script>

<svelte:head>
	<link rel="icon" href={favicon} />
</svelte:head>


<div class="flex h-screen flex-col overflow-hidden bg-gray-50">
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
	<header
		class="flex h-10 shrink-0 items-stretch border-b border-slate-300 bg-white/95 shadow-sm backdrop-blur"
		onmousedown={dragWindow}
	>
		<div class="flex min-w-0 flex-1 items-center gap-2 px-3">
			<span class="truncate text-xs font-semibold tracking-wide text-slate-700">
				PLI PAT Schema Builder
			</span>

			<a
				href="/"
				data-no-window-drag
				class={`rounded px-2.5 py-1 text-xs font-semibold tracking-wide transition-colors ${page.url.pathname === '/'
					? 'bg-slate-900 text-white'
					: 'text-slate-700 hover:bg-slate-100'}`}
			>
				{t('nav.home')}
			</a>
			<a
				href="/products"
				data-no-window-drag
				class={`rounded px-2.5 py-1 text-xs font-semibold tracking-wide transition-colors ${page.url.pathname.startsWith('/products')
					? 'bg-slate-900 text-white'
					: 'text-slate-700 hover:bg-slate-100'}`}
			>
				{t('nav.products')}
			</a>
			<a
				href="/customers"
				data-no-window-drag
				class={`rounded px-2.5 py-1 text-xs font-semibold tracking-wide transition-colors ${page.url.pathname.startsWith('/customers')
					? 'bg-slate-900 text-white'
					: 'text-slate-700 hover:bg-slate-100'}`}
			>
				{t('nav.customers')}
			</a>
			<a
				href="/templates"
				data-no-window-drag
				class={`rounded px-2.5 py-1 text-xs font-semibold tracking-wide transition-colors ${page.url.pathname.startsWith('/templates')
					? 'bg-slate-900 text-white'
					: 'text-slate-700 hover:bg-slate-100'}`}
			>
				{t('nav.templates')}
			</a>

			<div class="min-w-4 flex-1"></div>
		</div>

		<div class="flex items-stretch">
			<button
				type="button"
				data-no-window-drag
				class="grid w-10 place-items-center text-xs font-semibold text-slate-700 transition-colors hover:bg-slate-200"
				onclick={() => setLocale(getLocale() === 'en' ? 'it' : 'en')}
				aria-label={t('lang.switch')}
			>
				{getLocale() === 'en' ? 'IT' : 'EN'}
			</button>
			<button
				type="button"
				data-no-window-drag
				class="grid w-12 place-items-center text-slate-700 transition-colors hover:bg-slate-200"
				onclick={minimizeWindow}
				aria-label={t('win.minimize')}
			>
				<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" class="h-3.5 w-3.5" fill="currentColor">
					<rect x="5" y="11" width="14" height="2" />
				</svg>
			</button>
			<button
				type="button"
				data-no-window-drag
				class="grid w-12 place-items-center text-slate-700 transition-colors hover:bg-slate-200"
				onclick={toggleMaximize}
				aria-label={t('win.maximize')}
			>
				<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" class="h-3.5 w-3.5" fill="none" stroke="currentColor" stroke-width="2">
					<rect x="6" y="6" width="12" height="12" />
				</svg>
			</button>
			<button
				type="button"
				data-no-window-drag
				class="grid w-12 place-items-center text-slate-700 transition-colors hover:bg-rose-600 hover:text-white"
				onclick={closeWindow}
				aria-label={t('win.close')}
			>
				<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" class="h-3.5 w-3.5" fill="none" stroke="currentColor" stroke-width="2">
					<path d="M6 6l12 12M18 6L6 18" />
				</svg>
			</button>
		</div>
	</header>

	<main class="min-h-0 flex-1 overflow-y-auto">
		{@render children()}
	</main>
</div>
