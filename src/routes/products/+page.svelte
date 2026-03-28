<script lang="ts">
	import { confirm as confirmDialog, open as openDialog } from '@tauri-apps/plugin-dialog';
	import {
		createProduct,
		deleteProduct,
		getProductByCode,
		getProducts,
		uploadProductsExcel,
		updateProduct,
		type NewProduct,
		type Product
	} from '$lib/product-repository';

	type ProductForm = {
		code: string;
		description: string;
		units: number;
		pli: boolean;
	};

	const defaultForm: ProductForm = {
		code: '',
		description: '',
		units: 0,
		pli: false
	};

	let products = $state<Product[]>([]);
	let currentPage = $state(1);
	let pageSize = $state(50);
	let hasNextPage = $state(false);
	let loading = $state(false);
	let saving = $state(false);
	let errorMsg = $state<string | null>(null);
	let successMsg = $state<string | null>(null);
	let codeSearch = $state('');
	let pliFilter = $state<'all' | 'pli' | 'not-pli'>('all');

	let showCreateForm = $state(false);
	let newForm = $state<ProductForm>({ ...defaultForm });

	let editingId = $state<number | null>(null);
	let editForm = $state<ProductForm>({ ...defaultForm });

	function selectedPliFilter(): boolean | null {
		if (pliFilter === 'pli') return true;
		if (pliFilter === 'not-pli') return false;
		return null;
	}

	async function loadPage(page: number): Promise<void> {
		loading = true;
		errorMsg = null;

		try {
			const result = await getProducts(page, pageSize, selectedPliFilter());
			products = result.items;
			currentPage = result.page;
			hasNextPage = result.hasNextPage;
		} catch (err) {
			errorMsg = String(err);
		} finally {
			loading = false;
		}
	}

	async function onApplyFilters(): Promise<void> {
		loading = true;
		errorMsg = null;
		successMsg = null;

		try {
			if (codeSearch.trim()) {
				const product = await getProductByCode(codeSearch);
				const pli = selectedPliFilter();
				const matchesPli = pli === null || (product !== null && product.pli === pli);

				products = product !== null && matchesPli ? [product] : [];
				currentPage = 1;
				hasNextPage = false;
				return;
			}

			const result = await getProducts(1, pageSize, selectedPliFilter());
			products = result.items;
			currentPage = result.page;
			hasNextPage = result.hasNextPage;
		} catch (err) {
			errorMsg = String(err);
		} finally {
			loading = false;
		}
	}

	async function onResetFilters(): Promise<void> {
		codeSearch = '';
		pliFilter = 'all';
		successMsg = null;
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
		errorMsg = null;
		successMsg = null;

		const trimmedCode = newForm.code.trim();
		if (!trimmedCode) {
			errorMsg = 'Product code cannot be empty.';
			saving = false;
			return;
		}

		try {
			const payload: NewProduct = {
				code: trimmedCode,
				description: newForm.description.trim(),
				units: Number(newForm.units),
				pli: newForm.pli
			};
			await createProduct(payload);
			closeCreateForm();
			await loadPage(1);
		} catch (err) {
			errorMsg = String(err);
		} finally {
			saving = false;
		}
	}

	function startEdit(product: Product): void {
		editingId = product.id;
		editForm = {
			code: product.code,
			description: product.description,
			units: product.units,
			pli: product.pli
		};
	}

	function cancelEdit(): void {
		editingId = null;
		editForm = { ...defaultForm };
	}

	async function onSaveEdit(id: number): Promise<void> {
		saving = true;
		errorMsg = null;
		successMsg = null;

		const trimmedCode = editForm.code.trim();
		if (!trimmedCode) {
			errorMsg = 'Product code cannot be empty.';
			saving = false;
			return;
		}

		try {
			await updateProduct(id, {
				code: trimmedCode,
				description: editForm.description.trim(),
				units: Number(editForm.units),
				pli: editForm.pli
			});
			cancelEdit();
			await loadPage(currentPage);
		} catch (err) {
			errorMsg = String(err);
		} finally {
			saving = false;
		}
	}

	async function onDeleteProduct(event: MouseEvent, id: number): Promise<void> {
		event.preventDefault();
		event.stopPropagation();

		const confirmed = await confirmDialog('Delete this product?', {
			title: 'Confirm deletion',
			kind: 'warning',
			okLabel: 'Delete',
			cancelLabel: 'Cancel'
		});
		if (!confirmed) return;

		saving = true;
		errorMsg = null;
		successMsg = null;

		try {
			await deleteProduct(id);
			const targetPage = products.length === 1 && currentPage > 1 ? currentPage - 1 : currentPage;
			await loadPage(targetPage);
		} catch (err) {
			errorMsg = String(err);
		} finally {
			saving = false;
		}
	}

	async function onUploadProductsExcel(): Promise<void> {
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
			successMsg = await uploadProductsExcel(selected);
			await loadPage(1);
		} catch (err) {
			errorMsg = String(err);
		} finally {
			saving = false;
		}
	}

	$effect(() => {
		loadPage(1);
	});
</script>

<div class="h-full bg-gray-50">
	<main class="mx-auto max-w-7xl px-6 py-8">
		<section class="bg-white border border-slate-200 rounded-2xl shadow-sm p-5 md:p-6">
			{#if errorMsg}
				<p class="mb-4 rounded-lg border border-red-200 bg-red-50 px-3 py-2 text-sm text-red-700">{errorMsg}</p>
			{/if}

			{#if successMsg}
				<p class="mb-4 rounded-lg border border-emerald-200 bg-emerald-50 px-3 py-2 text-sm text-emerald-700">{successMsg}</p>
			{/if}

			<div class="flex items-center justify-between gap-4 mb-5">
				<h2 class="text-lg font-semibold text-slate-900">Product Table</h2>
				<div class="flex items-center gap-2">
					<button
						type="button"
						onclick={onUploadProductsExcel}
						class="rounded-lg bg-emerald-700 px-4 py-2 text-sm font-medium text-white hover:bg-emerald-800 disabled:opacity-50"
						disabled={saving}
					>
						Upload Excel
					</button>
					<button
						type="button"
						onclick={openCreateForm}
						class="rounded-lg bg-blue-700 px-4 py-2 text-sm font-medium text-white hover:bg-blue-800 disabled:opacity-50"
						disabled={saving}
					>
						Add Product
					</button>
				</div>
			</div>

			<form
				onsubmit={(event) => {
					event.preventDefault();
					onApplyFilters();
				}}
				class="mb-4 grid gap-2 rounded-xl border border-slate-200 bg-slate-50 p-3 md:grid-cols-[2fr_1fr_auto_auto]"
			>
				<input
					type="text"
					placeholder="Search by code"
					bind:value={codeSearch}
					class="rounded-md border border-slate-300 px-3 py-2 text-sm"
				/>
				<select bind:value={pliFilter} class="rounded-md border border-slate-300 px-3 py-2 text-sm">
					<option value="all">All</option>
					<option value="pli">PLI</option>
					<option value="not-pli">PAT</option>
				</select>
				<button
					type="submit"
					disabled={loading || saving}
					class="rounded-md bg-slate-900 px-3 py-2 text-sm font-medium text-white hover:bg-slate-700 disabled:opacity-50"
				>
					Search
				</button>
				<button
					type="button"
					onclick={onResetFilters}
					disabled={loading || saving}
					class="rounded-md border border-slate-300 px-3 py-2 text-sm text-slate-700 hover:bg-slate-100 disabled:opacity-50"
				>
					Reset
				</button>
			</form>

			{#if showCreateForm}
				<form
					onsubmit={(event) => {
						event.preventDefault();
						onCreateProduct();
					}}
					class="mb-6 grid gap-3 rounded-xl border border-blue-200 bg-blue-50 p-4 md:grid-cols-5"
				>
					<input
						type="text"
						required
						placeholder="Code"
						bind:value={newForm.code}
						class="rounded-md border border-slate-300 px-3 py-2 text-sm"
					/>
					<input
						type="text"
						required
						placeholder="Description"
						bind:value={newForm.description}
						class="rounded-md border border-slate-300 px-3 py-2 text-sm md:col-span-2"
					/>
					<input
						type="number"
						required
						min="0"
						placeholder="Units"
						bind:value={newForm.units}
						class="rounded-md border border-slate-300 px-3 py-2 text-sm"
					/>
					<label class="flex items-center gap-2 text-sm text-slate-700">
						<input type="checkbox" bind:checked={newForm.pli} />
						PLI
					</label>
					<div class="md:col-span-5 flex gap-2">
						<button
							type="submit"
							class="rounded-md bg-blue-700 px-3 py-2 text-sm font-medium text-white hover:bg-blue-800 disabled:opacity-50"
							disabled={saving}
						>
							Save
						</button>
						<button
							type="button"
							onclick={closeCreateForm}
							class="rounded-md border border-slate-300 px-3 py-2 text-sm text-slate-700 hover:bg-slate-100"
						>
							Cancel
						</button>
					</div>
				</form>
			{/if}

			<div class="overflow-x-auto">
				<table class="min-w-full border-collapse">
					<thead>
						<tr class="border-b border-slate-200 text-left text-xs uppercase tracking-wide text-slate-500">
							<th class="px-3 py-3">ID</th>
							<th class="px-3 py-3">Code</th>
							<th class="px-3 py-3">Description</th>
							<th class="px-3 py-3">Units</th>
							<th class="px-3 py-3">PLI</th>
							<th class="px-3 py-3">Actions</th>
						</tr>
					</thead>
					<tbody>
						{#if loading}
							<tr>
								<td colspan="6" class="px-3 py-6 text-center text-sm text-slate-500">Loading products...</td>
							</tr>
						{:else if products.length === 0}
							<tr>
								<td colspan="6" class="px-3 py-6 text-center text-sm text-slate-500">No products found.</td>
							</tr>
						{:else}
							{#each products as product (product.id)}
								<tr class="border-b border-slate-100 align-top">
									<td class="px-3 py-3 text-sm text-slate-700">{product.id}</td>
									{#if editingId === product.id}
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
											<input type="checkbox" bind:checked={editForm.pli} />
										</td>
										<td class="px-3 py-3">
											<div class="flex gap-2">
												<button
													onclick={() => onSaveEdit(product.id)}
													class="rounded-md bg-emerald-700 px-3 py-1 text-xs font-medium text-white hover:bg-emerald-800"
												>
													Save
												</button>
												<button
													onclick={cancelEdit}
													class="rounded-md border border-slate-300 px-3 py-1 text-xs text-slate-700 hover:bg-slate-100"
												>
													Cancel
												</button>
											</div>
										</td>
									{:else}
										<td class="px-3 py-3 text-sm text-slate-700">{product.code}</td>
										<td class="px-3 py-3 text-sm text-slate-700">{product.description}</td>
										<td class="px-3 py-3 text-sm text-slate-700">{product.units}</td>
										<td class="px-3 py-3 text-sm text-slate-700">{product.pli ? 'Yes' : 'No'}</td>
										<td class="px-3 py-3">
											<div class="flex gap-2">
												<button
													onclick={() => startEdit(product)}
													class="rounded-md bg-amber-600 px-3 py-1 text-xs font-medium text-white hover:bg-amber-700"
												>
													Edit
												</button>
												<button
													type="button"
													onclick={(event) => onDeleteProduct(event, product.id)}
													class="rounded-md bg-rose-700 px-3 py-1 text-xs font-medium text-white hover:bg-rose-800"
												>
													Delete
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
					Previous
				</button>
				<span class="text-sm text-slate-600">
					Page {currentPage}
				</span>
				<button
					onclick={() => loadPage(currentPage + 1)}
					disabled={!hasNextPage || loading || saving}
					class="rounded-lg bg-slate-900 px-4 py-2 text-sm font-medium text-white hover:bg-slate-700 disabled:opacity-40"
				>
					Next
				</button>
			</div>
		</section>
	</main>
</div>
