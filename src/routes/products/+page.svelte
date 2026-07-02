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
		type ProductType
	} from '$lib/product-repository';
	import { notices } from '$lib/notifications.svelte';
	import { t } from '$lib/i18n.svelte';
	import Notice from '$lib/Notice.svelte';
	import Spinner from '$lib/Spinner.svelte';

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
	let pageSize = $state(50);
	let hasNextPage = $state(false);
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

	// Base columns (ID, Code, Description, Units, Type, ADM Code, Actions) + the type-specific ones:
	// PLI → capacity+nicotine, PAT → packages+tabella, All → all four (shown only on wide screens).
	// colspan may overshoot the visible count when columns are responsively hidden — harmless.
	const columnCount = $derived(
		productTypeFilter === 'pli' ? 9 : productTypeFilter === 'pat' ? 9 : 11
	);

	async function loadPage(page: number): Promise<void> {
		loading = true;

		try {
			const result = await getProducts(
				page,
				pageSize,
				selectedProductTypeFilter(),
				incompleteOnly,
				codeSearch
			);
			products = result.items;
			currentPage = result.page;
			hasNextPage = result.hasNextPage;
		} catch (err) {
			n.error = String(err);
		} finally {
			loading = false;
		}
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

<div class="h-full bg-gray-50">
	<main class="mx-auto max-w-7xl px-6 py-8">
		<section class="bg-white border border-slate-200 rounded-2xl shadow-sm p-5 md:p-6">
			<Notice notice={n} />

			<div class="flex items-center justify-between gap-4 mb-5">
				<h2 class="text-lg font-semibold text-slate-900">{t('products.title')}</h2>
				<div class="flex items-center gap-2">
					<button
						type="button"
						onclick={onUploadProductsExcel}
						class="inline-flex items-center gap-2 rounded-lg bg-emerald-700 px-4 py-2 text-sm font-medium text-white hover:bg-emerald-800 disabled:opacity-50"
						disabled={saving}
					>
						{#if uploadingExcel}<Spinner class="h-4 w-4" />{/if}
						{t('products.uploadInfo3')}
					</button>
					<button
						type="button"
						onclick={onUploadSkeletonExcel}
						class="inline-flex items-center gap-2 rounded-lg bg-teal-700 px-4 py-2 text-sm font-medium text-white hover:bg-teal-800 disabled:opacity-50"
						disabled={saving}
					>
						{#if uploadingSkeleton}<Spinner class="h-4 w-4" />{/if}
						{t('products.uploadSkeleton')}
					</button>
					<button
						type="button"
						onclick={openCreateForm}
						class="rounded-lg bg-blue-700 px-4 py-2 text-sm font-medium text-white hover:bg-blue-800 disabled:opacity-50"
						disabled={saving}
					>
						{t('products.add')}
					</button>
					<div class="ml-2 flex items-center space-x-3">
						<button
							onclick={() => loadPage(currentPage - 1)}
							disabled={currentPage === 1 || loading || saving}
							class="rounded-lg border border-slate-300 px-4 py-2 text-sm font-medium text-slate-700 hover:bg-slate-100 disabled:opacity-40"
						>
							{t('common.previous')}
						</button>
						<span class="text-sm text-slate-600">{t('common.page', { n: currentPage })}</span>
						<button
							onclick={() => loadPage(currentPage + 1)}
							disabled={!hasNextPage || loading || saving}
							class="rounded-lg bg-slate-900 px-4 py-2 text-sm font-medium text-white hover:bg-slate-700 disabled:opacity-40"
						>
							{t('common.next')}
						</button>
					</div>
				</div>
			</div>

			<form
				onsubmit={(event) => {
					event.preventDefault();
					onApplyFilters();
				}}
				class="mb-4 grid gap-2 rounded-xl border border-slate-200 bg-slate-50 p-3 md:grid-cols-[2fr_1fr_auto_auto_auto]"
			>
				<input
					type="text"
					placeholder={t('products.searchByCode')}
					bind:value={codeSearch}
					oninput={onSearchInput}
					class="rounded-md border border-slate-300 px-3 py-2 text-sm"
				/>
				<select bind:value={productTypeFilter} class="rounded-md border border-slate-300 px-3 py-2 text-sm">
					<option value="all">{t('common.all')}</option>
					<option value="pli">PLI</option>
					<option value="pat">PAT</option>
				</select>
				<label class="flex items-center gap-2 rounded-md border border-slate-300 bg-white px-3 py-2 text-sm text-slate-700 whitespace-nowrap">
					<input type="checkbox" bind:checked={incompleteOnly} class="rounded border-slate-300" />
					{t('products.incompleteOnly')}
				</label>
				<button
					type="submit"
					disabled={loading || saving}
					class="rounded-md bg-slate-900 px-3 py-2 text-sm font-medium text-white hover:bg-slate-700 disabled:opacity-50"
				>
					{t('common.search')}
				</button>
				<button
					type="button"
					onclick={onResetFilters}
					disabled={loading || saving}
					class="rounded-md border border-slate-300 px-3 py-2 text-sm text-slate-700 hover:bg-slate-100 disabled:opacity-50"
				>
					{t('common.reset')}
				</button>
			</form>

			{#if showCreateForm}
				<form
					onsubmit={(event) => {
						event.preventDefault();
						onCreateProduct();
					}}
					class="mb-6 grid gap-3 rounded-xl border border-blue-200 bg-blue-50 p-4 md:grid-cols-8"
				>
					<label class="flex flex-col gap-1 text-xs font-medium text-slate-600">
						{t('products.code')}
						<input
							type="text"
							required
							bind:value={newForm.code}
							class="rounded-md border border-slate-300 px-3 py-2 text-sm font-normal"
						/>
					</label>
					<label class="flex flex-col gap-1 text-xs font-medium text-slate-600 md:col-span-2">
						{t('products.description')}
						<input
							type="text"
							required
							bind:value={newForm.description}
							class="rounded-md border border-slate-300 px-3 py-2 text-sm font-normal"
						/>
					</label>
					<label class="flex flex-col gap-1 text-xs font-medium text-slate-600">
						{t('products.units')}
						<input
							type="number"
							required
							min="0"
							bind:value={newForm.units}
							class="rounded-md border border-slate-300 px-3 py-2 text-sm font-normal"
						/>
					</label>
					<label class="flex flex-col gap-1 text-xs font-medium text-slate-600">
						{t('products.type')}
						<select bind:value={newForm.productType} class="rounded-md border border-slate-300 px-3 py-2 text-sm font-normal">
							<option value="pli">PLI</option>
							<option value="pat">PAT</option>
						</select>
					</label>
					<label class="flex flex-col gap-1 text-xs font-medium text-slate-600">
						{t('products.admCode')}
						<input
							type="text"
							bind:value={newForm.admCode}
							class="rounded-md border border-slate-300 px-3 py-2 text-sm font-normal"
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
								class="rounded-md border border-slate-300 px-3 py-2 text-sm font-normal"
							/>
						</label>
						<label class="flex flex-col gap-1 text-xs font-medium text-slate-600">
							{t('products.nicotine')}
							<input
								type="number"
								required
								min="0"
								bind:value={newForm.nicotine}
								class="rounded-md border border-slate-300 px-3 py-2 text-sm font-normal"
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
								class="rounded-md border border-slate-300 px-3 py-2 text-sm font-normal"
							/>
						</label>
						<label class="flex flex-col gap-1 text-xs font-medium text-slate-600">
							{t('products.tabella')}
							<input
								type="number"
								min="0"
								bind:value={newForm.tabella}
								class="rounded-md border border-slate-300 px-3 py-2 text-sm font-normal"
							/>
						</label>
					{/if}
					<div class="md:col-span-8 flex gap-2">
						<button
							type="submit"
							class="rounded-md bg-blue-700 px-3 py-2 text-sm font-medium text-white hover:bg-blue-800 disabled:opacity-50"
							disabled={saving}
						>
							{t('common.save')}
						</button>
						<button
							type="button"
							onclick={closeCreateForm}
							class="rounded-md border border-slate-300 px-3 py-2 text-sm text-slate-700 hover:bg-slate-100"
						>
							{t('common.cancel')}
						</button>
					</div>
				</form>
			{/if}

			<div class="overflow-x-auto">
				<table class="min-w-full border-collapse">
					<thead>
						<tr class="border-b border-slate-200 text-left text-xs uppercase tracking-wide text-slate-500">
							<th class="px-3 py-3">{t('common.id')}</th>
							<th class="px-3 py-3">{t('products.code')}</th>
							<th class="px-3 py-3">{t('products.description')}</th>
							<th class="px-3 py-3">{t('products.units')}</th>
							<th class="px-3 py-3">{t('products.type')}</th>
							<th class="px-3 py-3">{t('products.admCode')}</th>
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
							<tr>
								<td colspan={columnCount} class="px-3 py-6 text-center text-sm text-slate-500">{t('products.loading')}</td>
							</tr>
						{:else if products.length === 0}
							<tr>
								<td colspan={columnCount} class="px-3 py-6 text-center text-sm text-slate-500">{t('products.none')}</td>
							</tr>
						{:else}
							{#each products as product (`${product.productType}-${product.id}`)}
								<tr class="border-b border-slate-100 align-top">
									<td class="px-3 py-3 text-sm text-slate-700">{product.id}</td>
									{#if isEditing(product)}
										<td class="px-3 py-3">
											<input bind:value={editForm.code} class="w-full rounded-md border border-slate-300 px-2 py-1 text-sm" />
										</td>
										<td class="px-3 py-3">
											<input bind:value={editForm.description} class="w-full rounded-md border border-slate-300 px-2 py-1 text-sm" />
										</td>
										<td class="px-3 py-3">
											<input type="number" min="0" bind:value={editForm.units} class="w-24 rounded-md border border-slate-300 px-2 py-1 text-sm" />
										</td>
										<td class="px-3 py-3">
											<span class="text-sm text-slate-600">{editForm.productType.toUpperCase()}</span>
										</td>
										<td class="px-3 py-3">
											<div class="flex flex-col gap-2">
												<input type="text" bind:value={editForm.admCode} class="w-28 rounded-md border border-slate-300 px-2 py-1 text-sm" />
												{#if productTypeFilter === 'all'}
													<!-- Narrow-screen fallback: type-specific columns are hidden < xl, so stack them here. -->
													<div class="flex flex-col gap-2 xl:hidden">
														{#if editForm.productType === 'pli'}
															<label class="flex flex-col gap-1 text-xs text-slate-500">
																{t('products.capacity')}
																<input type="number" min="0" bind:value={editForm.capacity} class="w-28 rounded-md border border-slate-300 px-2 py-1 text-sm text-slate-900" />
															</label>
															<label class="flex flex-col gap-1 text-xs text-slate-500">
																{t('products.nicotine')}
																<input type="number" min="0" bind:value={editForm.nicotine} class="w-28 rounded-md border border-slate-300 px-2 py-1 text-sm text-slate-900" />
															</label>
														{:else}
															<label class="flex flex-col gap-1 text-xs text-slate-500">
																{t('products.packages')}
																<input type="number" min="0" bind:value={editForm.packages} class="w-28 rounded-md border border-slate-300 px-2 py-1 text-sm text-slate-900" />
															</label>
															<label class="flex flex-col gap-1 text-xs text-slate-500">
																{t('products.tabella')}
																<input type="number" min="0" bind:value={editForm.tabella} class="w-28 rounded-md border border-slate-300 px-2 py-1 text-sm text-slate-900" />
															</label>
														{/if}
													</div>
												{/if}
											</div>
										</td>
										{#if productTypeFilter === 'pli'}
											<td class="px-3 py-3">
												<input type="number" min="0" bind:value={editForm.capacity} class="w-24 rounded-md border border-slate-300 px-2 py-1 text-sm" />
											</td>
											<td class="px-3 py-3">
												<input type="number" min="0" bind:value={editForm.nicotine} class="w-24 rounded-md border border-slate-300 px-2 py-1 text-sm" />
											</td>
										{:else if productTypeFilter === 'pat'}
											<td class="px-3 py-3">
												<input type="number" min="0" bind:value={editForm.packages} class="w-24 rounded-md border border-slate-300 px-2 py-1 text-sm" />
											</td>
											<td class="px-3 py-3">
												<input type="number" min="0" bind:value={editForm.tabella} class="w-24 rounded-md border border-slate-300 px-2 py-1 text-sm" />
											</td>
										{:else}
											<td class="hidden px-3 py-3 xl:table-cell">
												{#if editForm.productType === 'pli'}
													<input type="number" min="0" bind:value={editForm.capacity} class="w-24 rounded-md border border-slate-300 px-2 py-1 text-sm" />
												{:else}<span class="text-sm text-slate-400">-</span>{/if}
											</td>
											<td class="hidden px-3 py-3 xl:table-cell">
												{#if editForm.productType === 'pli'}
													<input type="number" min="0" bind:value={editForm.nicotine} class="w-24 rounded-md border border-slate-300 px-2 py-1 text-sm" />
												{:else}<span class="text-sm text-slate-400">-</span>{/if}
											</td>
											<td class="hidden px-3 py-3 xl:table-cell">
												{#if editForm.productType === 'pat'}
													<input type="number" min="0" bind:value={editForm.packages} class="w-24 rounded-md border border-slate-300 px-2 py-1 text-sm" />
												{:else}<span class="text-sm text-slate-400">-</span>{/if}
											</td>
											<td class="hidden px-3 py-3 xl:table-cell">
												{#if editForm.productType === 'pat'}
													<input type="number" min="0" bind:value={editForm.tabella} class="w-24 rounded-md border border-slate-300 px-2 py-1 text-sm" />
												{:else}<span class="text-sm text-slate-400">-</span>{/if}
											</td>
										{/if}
										<td class="px-3 py-3">
											<div class="flex gap-2">
												<button
													onclick={() => onSaveEdit(product.id)}
													class="rounded-md bg-emerald-700 px-3 py-1 text-xs font-medium text-white hover:bg-emerald-800"
												>
													{t('common.save')}
												</button>
												<button
													onclick={cancelEdit}
													class="rounded-md border border-slate-300 px-3 py-1 text-xs text-slate-700 hover:bg-slate-100"
												>
													{t('common.cancel')}
												</button>
											</div>
										</td>
									{:else}
										<td class="px-3 py-3 text-sm text-slate-700">
											<div class="flex items-center gap-2">
												<span>{product.code}</span>
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
										<td class="px-3 py-3 text-sm text-slate-700">{product.admCode ?? '-'}</td>
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
											<div class="flex gap-2">
												<button
													onclick={() => startEdit(product)}
													class="rounded-md bg-amber-600 px-3 py-1 text-xs font-medium text-white hover:bg-amber-700"
												>
													{t('common.edit')}
												</button>
												<button
													type="button"
													onclick={(event) => onDeleteProduct(event, product.id)}
													class="rounded-md bg-rose-700 px-3 py-1 text-xs font-medium text-white hover:bg-rose-800"
												>
													{t('common.delete')}
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

			<div class="mt-5 flex items-center justify-end space-x-3">
				<button
					onclick={() => loadPage(currentPage - 1)}
					disabled={currentPage === 1 || loading || saving}
					class="rounded-lg border border-slate-300 px-4 py-2 text-sm font-medium text-slate-700 hover:bg-slate-100 disabled:opacity-40"
				>
					{t('common.previous')}
				</button>
				<span class="text-sm text-slate-600">
					{t('common.page', { n: currentPage })}
				</span>
				<button
					onclick={() => loadPage(currentPage + 1)}
					disabled={!hasNextPage || loading || saving}
					class="rounded-lg bg-slate-900 px-4 py-2 text-sm font-medium text-white hover:bg-slate-700 disabled:opacity-40"
				>
					{t('common.next')}
				</button>
			</div>
		</section>
	</main>
</div>
