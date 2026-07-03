<script lang="ts">
	import { confirm as confirmDialog, open as openDialog } from '@tauri-apps/plugin-dialog';
	import {
		createProduct,
		deleteProduct,
		getProducts,
		uploadProductsExcel,
		uploadSkeletonExcel,
		updateProduct,
		type NewProduct,
		type Product,
		type ProductType,
		type ProductSortBy,
		type SortDir
	} from '$lib/product-repository';
	import { notices } from '$lib/notifications.svelte';
	import { t } from '$lib/i18n.svelte';
	import Notice from '$lib/Notice.svelte';
	import Spinner from '$lib/Spinner.svelte';
	import TableSkeleton from '$lib/TableSkeleton.svelte';
	import { btnPrimary, btnSecondary, btnAccent, btnPrimarySm, btnSecondarySm, iconBtn, iconBtnDanger, inputBase, inputCell } from '$lib/ui';

	const n = notices.products;

	type ProductForm = {
		code: string;
		description: string;
		units: number;
		productType: ProductType;
		capacity: number;
		nicotine: number;
		packages: number;
		admCode: string;
		tabella: number;
	};

	const defaultForm: ProductForm = {
		code: '',
		description: '',
		units: 0,
		productType: 'pli',
		capacity: 0,
		nicotine: 0,
		packages: 0,
		admCode: '',
		tabella: 0
	};

	let products = $state<Product[]>([]);
	let currentPage = $state(1);
	let pageSize = $state(20);
	let hasNextPage = $state(false);
	let totalCount = $state(0);
	let sortBy = $state<ProductSortBy | null>(null);
	let sortDir = $state<SortDir>('asc');
	let loading = $state(false);
	let saving = $state(false);
	let uploadingExcel = $state(false);
	let uploadingSkeleton = $state(false);
	let codeSearch = $state('');
	let productTypeFilter = $state<'all' | 'pli' | 'pat'>('all');
	let incompleteOnly = $state(false);

	let showCreateForm = $state(false);
	let newForm = $state<ProductForm>({ ...defaultForm });

	type EditingTarget = {
		id: number;
		productType: ProductType;
	} | null;
	let editingTarget = $state<EditingTarget>(null);
	let editForm = $state<ProductForm>({ ...defaultForm });

	function selectedProductTypeFilter(): ProductType | null {
		if (productTypeFilter === 'pli') return 'pli';
		if (productTypeFilter === 'pat') return 'pat';
		return null;
	}

	// Mirrors Product::is_skeleton_complete in the Rust backend: a product is usable for tracciati
	// only once the skeleton has supplied its fields (PLI capacity+nicotine, PAT adm code).
	function isComplete(product: Product): boolean {
		return product.productType === 'pli'
			? product.capacity != null && product.nicotine != null
			: !!product.admCode;
	}

	// Base columns (Code, Description, Units, Type, ADM Code, Actions) + the type-specific ones:
	// PLI → capacity+nicotine, PAT → packages+tabella, All → all four (shown only on wide screens).
	// colspan may overshoot the visible count when columns are responsively hidden — harmless.
	const columnCount = $derived(
		productTypeFilter === 'pli' ? 8 : productTypeFilter === 'pat' ? 8 : 10
	);

	const hasActiveFilters = $derived(
		codeSearch.trim().length > 0 || productTypeFilter !== 'all' || incompleteOnly
	);

	const totalPages = $derived(Math.max(1, Math.ceil(totalCount / pageSize)));

	async function loadPage(page: number): Promise<void> {
		loading = true;

		try {
			const result = await getProducts(
				page,
				pageSize,
				selectedProductTypeFilter(),
				incompleteOnly,
				codeSearch,
				sortBy ?? undefined,
				sortDir
			);
			products = result.items;
			currentPage = result.page;
			hasNextPage = result.hasNextPage;
			totalCount = result.totalCount;
		} catch (err) {
			n.error = String(err);
		} finally {
			loading = false;
		}
	}

	function toggleSort(column: ProductSortBy): void {
		if (sortBy === column) {
			sortDir = sortDir === 'asc' ? 'desc' : 'asc';
		} else {
			sortBy = column;
			sortDir = 'asc';
		}
		loadPage(1);
	}

	async function onApplyFilters(): Promise<void> {
		n.success = null;
		await loadPage(1);
	}

	// Live search while typing, but skip a single-character term (too broad) — Enter/Search forces it.
	let searchTimer: ReturnType<typeof setTimeout> | undefined;
	function onSearchInput(): void {
		clearTimeout(searchTimer);
		const term = codeSearch.trim();
		if (term.length === 1) return;
		searchTimer = setTimeout(() => onApplyFilters(), 250);
	}

	async function onResetFilters(): Promise<void> {
		codeSearch = '';
		productTypeFilter = 'all';
		incompleteOnly = false;
		n.success = null;
		await loadPage(1);
	}

	function openCreateForm(): void {
		showCreateForm = true;
		newForm = { ...defaultForm };
	}

	function closeCreateForm(): void {
		showCreateForm = false;
		newForm = { ...defaultForm };
	}

	async function onCreateProduct(): Promise<void> {
		saving = true;
		n.error = null;
		n.success = null;

		const trimmedCode = newForm.code.trim();
		if (!trimmedCode) {
			n.error = t('products.codeEmpty');
			saving = false;
			return;
		}

		try {
			const payload: NewProduct = {
				productType: newForm.productType,
				code: trimmedCode,
				description: newForm.description.trim(),
				units: Number(newForm.units),
				capacity: newForm.productType === 'pli' ? Number(newForm.capacity) : undefined,
				nicotine: newForm.productType === 'pli' ? Number(newForm.nicotine) : undefined,
				packages: newForm.productType === 'pat' ? Number(newForm.packages) : undefined,
				admCode: newForm.admCode.trim() || undefined,
				tabella: newForm.productType === 'pat' ? Number(newForm.tabella) : undefined
			};
			await createProduct(payload);
			closeCreateForm();
			await loadPage(1);
		} catch (err) {
			n.error = String(err);
		} finally {
			saving = false;
		}
	}

	function startEdit(product: Product): void {
		editingTarget = { id: product.id, productType: product.productType };
		editForm = {
			code: product.code,
			description: product.description,
			units: product.units,
			productType: product.productType,
			capacity: product.capacity ?? 0,
			nicotine: product.nicotine ?? 0,
			packages: product.packages ?? 0,
			admCode: product.admCode ?? '',
			tabella: product.tabella ?? 0
		};
	}

	function cancelEdit(): void {
		editingTarget = null;
		editForm = { ...defaultForm };
	}

	function isEditing(product: Product): boolean {
		return (
			editingTarget?.id === product.id && editingTarget?.productType === product.productType
		);
	}

	async function onSaveEdit(id: number): Promise<void> {
		saving = true;
		n.error = null;
		n.success = null;

		const trimmedCode = editForm.code.trim();
		if (!trimmedCode) {
			n.error = t('products.codeEmpty');
			saving = false;
			return;
		}

		try {
			await updateProduct(id, {
				code: trimmedCode,
				description: editForm.description.trim(),
				units: Number(editForm.units),
				capacity: editForm.productType === 'pli' ? Number(editForm.capacity) : undefined,
				nicotine: editForm.productType === 'pli' ? Number(editForm.nicotine) : undefined,
				packages: editForm.productType === 'pat' ? Number(editForm.packages) : undefined,
				admCode: editForm.admCode.trim(),
				tabella: editForm.productType === 'pat' ? Number(editForm.tabella) : undefined
			});
			cancelEdit();
			await loadPage(currentPage);
		} catch (err) {
			n.error = String(err);
		} finally {
			saving = false;
		}
	}

	async function onDeleteProduct(event: MouseEvent, id: number): Promise<void> {
		event.preventDefault();
		event.stopPropagation();

		const confirmed = await confirmDialog(t('products.deleteConfirm'), {
			title: t('common.confirmDeletion'),
			kind: 'warning',
			okLabel: t('common.delete'),
			cancelLabel: t('common.cancel')
		});
		if (!confirmed) return;

		saving = true;
		n.error = null;
		n.success = null;

		try {
			await deleteProduct(id);
			const targetPage = products.length === 1 && currentPage > 1 ? currentPage - 1 : currentPage;
			await loadPage(targetPage);
		} catch (err) {
			n.error = String(err);
		} finally {
			saving = false;
		}
	}

	async function onUploadProductsExcel(): Promise<void> {
		n.error = null;
		n.success = null;

		const selected = await openDialog({
			multiple: false,
			directory: false,
			filters: [{ name: t('common.excelFilter'), extensions: ['xlsx'] }]
		});

		if (!selected || Array.isArray(selected)) {
			return;
		}

		saving = true;
		uploadingExcel = true;

		try {
			n.success = await uploadProductsExcel(selected);
			await loadPage(1);
		} catch (err) {
			n.error = String(err);
		} finally {
			saving = false;
			uploadingExcel = false;
		}
	}

	async function onUploadSkeletonExcel(): Promise<void> {
		n.error = null;
		n.success = null;

		const selected = await openDialog({
			multiple: false,
			directory: false,
			filters: [{ name: t('common.excelFilter'), extensions: ['xlsx'] }]
		});

		if (!selected || Array.isArray(selected)) {
			return;
		}

		saving = true;
		uploadingSkeleton = true;

		try {
			n.success = await uploadSkeletonExcel(selected);
			await loadPage(1);
		} catch (err) {
			n.error = String(err);
		} finally {
			saving = false;
			uploadingSkeleton = false;
		}
	}

	$effect(() => {
		loadPage(1);
	});
</script>

<div class="h-full bg-slate-50">
	<main class="mx-auto max-w-7xl px-6 py-10">
		<header class="mb-6 flex flex-wrap items-start justify-between gap-4">
			<div class="min-w-0">
				<h1 class="text-xl font-semibold tracking-tight text-slate-900">{t('products.title')}</h1>
				<p class="mt-1 max-w-prose text-sm text-slate-500">{t('products.subtitle')}</p>
			</div>
			<div class="flex flex-wrap items-center gap-2">
				<button type="button" onclick={openCreateForm} class={btnSecondary} disabled={saving}>
					{t('products.add')}
				</button>
				<!-- Import is a fixed two-step order: the product list first, the skeleton second.
				     Step badges + the connector arrow make that sequence legible at a glance. -->
				<div class="flex items-center gap-2">
					<button
						type="button"
						onclick={onUploadProductsExcel}
						class={btnPrimary}
						disabled={saving}
						title={t('products.uploadInfo3Hint')}
					>
						{#if uploadingExcel}
							<Spinner class="h-4 w-4" />
						{:else}
							<span class="grid size-4 shrink-0 place-items-center rounded-full bg-white text-[0.6875rem] font-bold text-slate-900" aria-label={t('products.stepAria', { n: 1 })}>1</span>
						{/if}
						{t('products.uploadInfo3')}
					</button>
					<svg viewBox="0 0 20 20" class="size-4 shrink-0 text-slate-400" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M4 10h11M11 6l4 4-4 4" /></svg>
					<button
						type="button"
						onclick={onUploadSkeletonExcel}
						class={btnAccent}
						disabled={saving}
						title={t('products.uploadSkeletonHint')}
					>
						{#if uploadingSkeleton}
							<Spinner class="h-4 w-4" />
						{:else}
							<span class="grid size-4 shrink-0 place-items-center rounded-full bg-white text-[0.6875rem] font-bold text-indigo-700" aria-label={t('products.stepAria', { n: 2 })}>2</span>
						{/if}
						{t('products.uploadSkeleton')}
					</button>
				</div>
			</div>
		</header>

		<Notice notice={n} />

		<form
			onsubmit={(event) => {
				event.preventDefault();
				onApplyFilters();
			}}
			class="mb-4 grid gap-2 md:grid-cols-[2fr_1fr_auto_auto_auto]"
		>
			<input
				type="text"
				placeholder={t('products.searchByCode')}
				bind:value={codeSearch}
				oninput={onSearchInput}
				class={inputBase}
			/>
			<select bind:value={productTypeFilter} class={inputBase}>
				<option value="all">{t('common.all')}</option>
				<option value="pli">PLI</option>
				<option value="pat">PAT</option>
			</select>
			<label class="flex items-center gap-2 rounded-lg border border-slate-300 bg-white px-3 py-2 text-sm text-slate-700 whitespace-nowrap shadow-sm">
				<input type="checkbox" bind:checked={incompleteOnly} class="rounded border-slate-300" />
				{t('products.incompleteOnly')}
			</label>
			<button type="submit" disabled={loading || saving} class={btnPrimary}>
				{t('common.search')}
			</button>
			<button type="button" onclick={onResetFilters} disabled={loading || saving} class={btnSecondary}>
				{t('common.reset')}
			</button>
		</form>

		<div class="rounded-xl border border-slate-200 bg-white shadow-sm">
			{#if showCreateForm}
				<form
					onsubmit={(event) => {
						event.preventDefault();
						onCreateProduct();
					}}
					class="grid gap-3 border-b border-slate-200 bg-slate-50 p-4 md:grid-cols-8"
				>
					<label class="flex flex-col gap-1 text-xs font-medium text-slate-600">
						{t('products.code')}
						<input
							type="text"
							required
							bind:value={newForm.code}
							class="{inputBase} font-normal"
						/>
					</label>
					<label class="flex flex-col gap-1 text-xs font-medium text-slate-600 md:col-span-2">
						{t('products.description')}
						<input
							type="text"
							required
							bind:value={newForm.description}
							class="{inputBase} font-normal"
						/>
					</label>
					<label class="flex flex-col gap-1 text-xs font-medium text-slate-600">
						{t('products.units')}
						<input
							type="number"
							required
							min="0"
							bind:value={newForm.units}
							class="{inputBase} font-normal"
						/>
					</label>
					<label class="flex flex-col gap-1 text-xs font-medium text-slate-600">
						{t('products.type')}
						<select bind:value={newForm.productType} class="{inputBase} font-normal">
							<option value="pli">PLI</option>
							<option value="pat">PAT</option>
						</select>
					</label>
					<label class="flex flex-col gap-1 text-xs font-medium text-slate-600">
						{t('products.admCode')}
						<input
							type="text"
							bind:value={newForm.admCode}
							class="{inputBase} font-normal"
						/>
					</label>
					{#if newForm.productType === 'pli'}
						<label class="flex flex-col gap-1 text-xs font-medium text-slate-600">
							{t('products.capacity')}
							<input
								type="number"
								required
								min="0"
								bind:value={newForm.capacity}
								class="{inputBase} font-normal"
							/>
						</label>
						<label class="flex flex-col gap-1 text-xs font-medium text-slate-600">
							{t('products.nicotine')}
							<input
								type="number"
								required
								min="0"
								bind:value={newForm.nicotine}
								class="{inputBase} font-normal"
							/>
						</label>
					{:else}
						<label class="flex flex-col gap-1 text-xs font-medium text-slate-600">
							{t('products.packages')}
							<input
								type="number"
								required
								min="0"
								bind:value={newForm.packages}
								class="{inputBase} font-normal"
							/>
						</label>
						<label class="flex flex-col gap-1 text-xs font-medium text-slate-600">
							{t('products.tabella')}
							<input
								type="number"
								min="0"
								bind:value={newForm.tabella}
								class="{inputBase} font-normal"
							/>
						</label>
					{/if}
					<div class="md:col-span-8 flex gap-2">
						<button type="submit" class={btnPrimary} disabled={saving}>
							{t('common.save')}
						</button>
						<button type="button" onclick={closeCreateForm} class={btnSecondary}>
							{t('common.cancel')}
						</button>
					</div>
				</form>
			{/if}

			<div class="overflow-x-auto">
				<table class="min-w-full border-collapse">
					<thead>
						<tr class="border-b border-slate-200 text-left text-xs uppercase tracking-wide text-slate-500">
							<th class="px-3 py-3">
								<button type="button" onclick={() => toggleSort('code')} class="flex items-center gap-1 uppercase tracking-wide">
									{t('products.code')}{#if sortBy === 'code'}<span>{sortDir === 'asc' ? '▲' : '▼'}</span>{/if}
								</button>
							</th>
							<th class="px-3 py-3">
								<button type="button" onclick={() => toggleSort('description')} class="flex items-center gap-1 uppercase tracking-wide">
									{t('products.description')}{#if sortBy === 'description'}<span>{sortDir === 'asc' ? '▲' : '▼'}</span>{/if}
								</button>
							</th>
							<th class="px-3 py-3">
								<button type="button" onclick={() => toggleSort('units')} class="flex items-center gap-1 uppercase tracking-wide">
									{t('products.units')}{#if sortBy === 'units'}<span>{sortDir === 'asc' ? '▲' : '▼'}</span>{/if}
								</button>
							</th>
							<th class="px-3 py-3">
								<button type="button" onclick={() => toggleSort('productType')} class="flex items-center gap-1 uppercase tracking-wide">
									{t('products.type')}{#if sortBy === 'productType'}<span>{sortDir === 'asc' ? '▲' : '▼'}</span>{/if}
								</button>
							</th>
							<th class="px-3 py-3">
								<button type="button" onclick={() => toggleSort('admCode')} class="flex items-center gap-1 uppercase tracking-wide">
									{t('products.admCode')}{#if sortBy === 'admCode'}<span>{sortDir === 'asc' ? '▲' : '▼'}</span>{/if}
								</button>
							</th>
							{#if productTypeFilter === 'pli'}
								<th class="px-3 py-3">{t('products.capacity')}</th>
								<th class="px-3 py-3">{t('products.nicotine')}</th>
							{:else if productTypeFilter === 'pat'}
								<th class="px-3 py-3">{t('products.packages')}</th>
								<th class="px-3 py-3">{t('products.tabella')}</th>
							{:else}
								<th class="hidden px-3 py-3 xl:table-cell">{t('products.capacity')}</th>
								<th class="hidden px-3 py-3 xl:table-cell">{t('products.nicotine')}</th>
								<th class="hidden px-3 py-3 xl:table-cell">{t('products.packages')}</th>
								<th class="hidden px-3 py-3 xl:table-cell">{t('products.tabella')}</th>
							{/if}
							<th class="px-3 py-3">{t('common.actions')}</th>
						</tr>
					</thead>
					<tbody>
						{#if loading}
							<TableSkeleton cols={columnCount} />
						{:else if products.length === 0}
							<tr>
								<td colspan={columnCount} class="px-3 py-16">
									<div class="mx-auto flex max-w-sm flex-col items-center text-center">
										{#if hasActiveFilters}
											<p class="text-sm text-slate-500">{t('products.emptyFiltered')}</p>
											<button type="button" onclick={onResetFilters} class="{btnSecondarySm} mt-3">{t('common.reset')}</button>
										{:else}
											<p class="text-sm font-medium text-slate-700">{t('products.emptyTitle')}</p>
											<p class="mt-1 text-sm text-slate-500">{t('products.emptyHint')}</p>
											<button type="button" onclick={onUploadProductsExcel} class="{btnPrimary} mt-4" disabled={saving}>
												{#if uploadingExcel}<Spinner class="h-4 w-4" />{/if}
												{t('products.uploadInfo3')}
											</button>
										{/if}
									</div>
								</td>
							</tr>
						{:else}
							{#each products as product (`${product.productType}-${product.id}`)}
								<tr class="border-b border-slate-100 align-top {isEditing(product) ? 'bg-slate-50' : ''}">
									{#if isEditing(product)}
										<td class="px-3 py-3">
											<input bind:value={editForm.code} class="w-full rounded-md border border-slate-300 px-2 py-1 font-mono text-sm focus:border-slate-500 focus:outline-none focus-visible:ring-2 focus-visible:ring-slate-400" />
										</td>
										<td class="px-3 py-3">
											<input bind:value={editForm.description} class="w-full rounded-md border border-slate-300 px-2 py-1 text-sm focus:border-slate-500 focus:outline-none focus-visible:ring-2 focus-visible:ring-slate-400" />
										</td>
										<td class="px-3 py-3">
											<input type="number" min="0" bind:value={editForm.units} class="w-24 rounded-md border border-slate-300 px-2 py-1 text-sm focus:border-slate-500 focus:outline-none focus-visible:ring-2 focus-visible:ring-slate-400" />
										</td>
										<td class="px-3 py-3">
											<span class="text-sm text-slate-600">{editForm.productType.toUpperCase()}</span>
										</td>
										<td class="px-3 py-3">
											<div class="flex flex-col gap-2">
												<input type="text" bind:value={editForm.admCode} class="w-28 rounded-md border border-slate-300 px-2 py-1 font-mono text-sm focus:border-slate-500 focus:outline-none focus-visible:ring-2 focus-visible:ring-slate-400" />
												{#if productTypeFilter === 'all'}
													<!-- Narrow-screen fallback: type-specific columns are hidden < xl, so stack them here. -->
													<div class="flex flex-col gap-2 xl:hidden">
														{#if editForm.productType === 'pli'}
															<label class="flex flex-col gap-1 text-xs text-slate-500">
																{t('products.capacity')}
																<input type="number" min="0" bind:value={editForm.capacity} class="w-28 rounded-md border border-slate-300 px-2 py-1 text-sm focus:border-slate-500 focus:outline-none focus-visible:ring-2 focus-visible:ring-slate-400 text-slate-900" />
															</label>
															<label class="flex flex-col gap-1 text-xs text-slate-500">
																{t('products.nicotine')}
																<input type="number" min="0" bind:value={editForm.nicotine} class="w-28 rounded-md border border-slate-300 px-2 py-1 text-sm focus:border-slate-500 focus:outline-none focus-visible:ring-2 focus-visible:ring-slate-400 text-slate-900" />
															</label>
														{:else}
															<label class="flex flex-col gap-1 text-xs text-slate-500">
																{t('products.packages')}
																<input type="number" min="0" bind:value={editForm.packages} class="w-28 rounded-md border border-slate-300 px-2 py-1 text-sm focus:border-slate-500 focus:outline-none focus-visible:ring-2 focus-visible:ring-slate-400 text-slate-900" />
															</label>
															<label class="flex flex-col gap-1 text-xs text-slate-500">
																{t('products.tabella')}
																<input type="number" min="0" bind:value={editForm.tabella} class="w-28 rounded-md border border-slate-300 px-2 py-1 text-sm focus:border-slate-500 focus:outline-none focus-visible:ring-2 focus-visible:ring-slate-400 text-slate-900" />
															</label>
														{/if}
													</div>
												{/if}
											</div>
										</td>
										{#if productTypeFilter === 'pli'}
											<td class="px-3 py-3">
												<input type="number" min="0" bind:value={editForm.capacity} class="w-24 rounded-md border border-slate-300 px-2 py-1 text-sm focus:border-slate-500 focus:outline-none focus-visible:ring-2 focus-visible:ring-slate-400" />
											</td>
											<td class="px-3 py-3">
												<input type="number" min="0" bind:value={editForm.nicotine} class="w-24 rounded-md border border-slate-300 px-2 py-1 text-sm focus:border-slate-500 focus:outline-none focus-visible:ring-2 focus-visible:ring-slate-400" />
											</td>
										{:else if productTypeFilter === 'pat'}
											<td class="px-3 py-3">
												<input type="number" min="0" bind:value={editForm.packages} class="w-24 rounded-md border border-slate-300 px-2 py-1 text-sm focus:border-slate-500 focus:outline-none focus-visible:ring-2 focus-visible:ring-slate-400" />
											</td>
											<td class="px-3 py-3">
												<input type="number" min="0" bind:value={editForm.tabella} class="w-24 rounded-md border border-slate-300 px-2 py-1 text-sm focus:border-slate-500 focus:outline-none focus-visible:ring-2 focus-visible:ring-slate-400" />
											</td>
										{:else}
											<td class="hidden px-3 py-3 xl:table-cell">
												{#if editForm.productType === 'pli'}
													<input type="number" min="0" bind:value={editForm.capacity} class="w-24 rounded-md border border-slate-300 px-2 py-1 text-sm focus:border-slate-500 focus:outline-none focus-visible:ring-2 focus-visible:ring-slate-400" />
												{:else}<span class="text-sm text-slate-400">-</span>{/if}
											</td>
											<td class="hidden px-3 py-3 xl:table-cell">
												{#if editForm.productType === 'pli'}
													<input type="number" min="0" bind:value={editForm.nicotine} class="w-24 rounded-md border border-slate-300 px-2 py-1 text-sm focus:border-slate-500 focus:outline-none focus-visible:ring-2 focus-visible:ring-slate-400" />
												{:else}<span class="text-sm text-slate-400">-</span>{/if}
											</td>
											<td class="hidden px-3 py-3 xl:table-cell">
												{#if editForm.productType === 'pat'}
													<input type="number" min="0" bind:value={editForm.packages} class="w-24 rounded-md border border-slate-300 px-2 py-1 text-sm focus:border-slate-500 focus:outline-none focus-visible:ring-2 focus-visible:ring-slate-400" />
												{:else}<span class="text-sm text-slate-400">-</span>{/if}
											</td>
											<td class="hidden px-3 py-3 xl:table-cell">
												{#if editForm.productType === 'pat'}
													<input type="number" min="0" bind:value={editForm.tabella} class="w-24 rounded-md border border-slate-300 px-2 py-1 text-sm focus:border-slate-500 focus:outline-none focus-visible:ring-2 focus-visible:ring-slate-400" />
												{:else}<span class="text-sm text-slate-400">-</span>{/if}
											</td>
										{/if}
										<td class="px-3 py-3">
											<div class="flex gap-2">
												<button onclick={() => onSaveEdit(product.id)} class={btnPrimarySm} disabled={saving}>
													{t('common.save')}
												</button>
												<button onclick={cancelEdit} class={btnSecondarySm}>
													{t('common.cancel')}
												</button>
											</div>
										</td>
									{:else}
										<td class="px-3 py-3 text-sm text-slate-700">
											<div class="flex items-center gap-2">
												<span class="font-mono">{product.code}</span>
												{#if !isComplete(product)}
													<span
														class="rounded-md border border-amber-200 bg-amber-50 px-1.5 py-0.5 text-xs font-medium text-amber-700"
														title={t('products.incompleteTitle')}
													>
														{t('products.incompleteTag')}
													</span>
												{/if}
											</div>
										</td>
										<td class="px-3 py-3 text-sm text-slate-700">{product.description}</td>
										<td class="px-3 py-3 text-sm text-slate-700">{product.units}</td>
										<td class="px-3 py-3 text-sm text-slate-700">{product.productType.toUpperCase()}</td>
										<td class="px-3 py-3 font-mono text-sm text-slate-700">{product.admCode ?? '-'}</td>
										{#if productTypeFilter === 'pli'}
											<td class="px-3 py-3 text-sm text-slate-700">{product.capacity ?? '-'}</td>
											<td class="px-3 py-3 text-sm text-slate-700">{product.nicotine ?? '-'}</td>
										{:else if productTypeFilter === 'pat'}
											<td class="px-3 py-3 text-sm text-slate-700">{product.packages ?? '-'}</td>
											<td class="px-3 py-3 text-sm text-slate-700">{product.tabella ?? '-'}</td>
										{:else}
											<td class="hidden px-3 py-3 text-sm text-slate-700 xl:table-cell">{product.capacity ?? '-'}</td>
											<td class="hidden px-3 py-3 text-sm text-slate-700 xl:table-cell">{product.nicotine ?? '-'}</td>
											<td class="hidden px-3 py-3 text-sm text-slate-700 xl:table-cell">{product.packages ?? '-'}</td>
											<td class="hidden px-3 py-3 text-sm text-slate-700 xl:table-cell">{product.tabella ?? '-'}</td>
										{/if}
										<td class="px-3 py-3">
											<div class="flex gap-1">
												<button onclick={() => startEdit(product)} class={iconBtn} title={t('common.edit')} aria-label={t('common.edit')}>
													<svg viewBox="0 0 20 20" class="size-4" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"><path d="M4 13.5V16h2.5l7-7-2.5-2.5-7 7z" /><path d="M11.5 6.5l2 2" /></svg>
												</button>
												<button type="button" onclick={(event) => onDeleteProduct(event, product.id)} class={iconBtnDanger} title={t('common.delete')} aria-label={t('common.delete')}>
													<svg viewBox="0 0 20 20" class="size-4" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"><path d="M4 6h12M8 6V4h4v2M6 6l.7 9a1 1 0 001 .9h4.6a1 1 0 001-.9L15 6" /></svg>
												</button>
											</div>
										</td>
									{/if}
								</tr>
							{/each}
						{/if}
					</tbody>
				</table>
			</div>

			<div class="flex items-center justify-end gap-3 border-t border-slate-200 px-4 py-3">
				<button onclick={() => loadPage(currentPage - 1)} disabled={currentPage === 1 || loading || saving} class={btnSecondarySm}>
					{t('common.previous')}
				</button>
				<label class="flex items-center gap-2 text-sm text-slate-600">
					{t('common.pageOf', { n: currentPage, total: totalPages })}
					<select
						value={currentPage}
						onchange={(event) => loadPage(Number(event.currentTarget.value))}
						disabled={loading || saving}
						class="rounded-md border border-slate-300 bg-white px-2 py-1 text-sm focus:border-slate-500 focus:outline-none focus-visible:ring-2 focus-visible:ring-slate-400"
					>
						{#each Array(totalPages) as _, i}
							<option value={i + 1}>{i + 1}</option>
						{/each}
					</select>
				</label>
				<button onclick={() => loadPage(currentPage + 1)} disabled={!hasNextPage || loading || saving} class={btnSecondarySm}>
					{t('common.next')}
				</button>
			</div>
		</div>
	</main>
</div>
