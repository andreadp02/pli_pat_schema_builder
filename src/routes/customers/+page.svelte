<script lang="ts">
	import { confirm as confirmDialog, open as openDialog } from '@tauri-apps/plugin-dialog';
	import {
		type AmbiguousUploadRow,
		type InvalidUploadRow,
		type ProvinceResolution,
		confirmCustomersExcelUpload,
		createCustomer,
		deleteCustomer,
		getCustomers,
		updateCustomer,
		validateCustomersExcel,
		type Customer,
		type CustomerTypology,
		CUSTOMER_TYPOLOGIES,
		type NewCustomer
	} from '$lib/customer-repository';
	import { notices } from '$lib/notifications.svelte';
	import { t } from '$lib/i18n.svelte';
	import Notice from '$lib/Notice.svelte';
	import Spinner from '$lib/Spinner.svelte';

	const n = notices.customers;

	type CustomerForm = {
		taxCode: number;
		ordinalNumber: number;
		typology: CustomerTypology;
		vatNumber: string;
		address: string;
		provinceName: string;
		municipalityName: string;
	};

	const defaultForm: CustomerForm = {
		taxCode: 0,
		ordinalNumber: 0,
		typology: 'ESERCIZIO DI VICINATO',
		vatNumber: '',
		address: '',
		provinceName: '',
		municipalityName: ''
	};

	let customers = $state<Customer[]>([]);
	let currentPage = $state(1);
	let pageSize = $state(50);
	let hasNextPage = $state(false);
	let loading = $state(false);
	let saving = $state(false);
	let uploadingExcel = $state(false);
	let taxCodeSearch = $state('');
	let vatSearch = $state('');
	let typologyFilter = $state<'all' | CustomerTypology>('all');
	let pendingUploadPath = $state<string | null>(null);
	let pendingSkippedMessage = $state<string | null>(null);
	let ambiguousRows = $state<AmbiguousUploadRow[]>([]);
	let provinceSelections = $state<Record<number, string>>({});
	let showProvinceResolutionModal = $state(false);
	let provinceError = $state<string | null>(null);

	let showCreateForm = $state(false);
	let newForm = $state<CustomerForm>({ ...defaultForm });

	let editingId = $state<number | null>(null);
	let editForm = $state<CustomerForm>({ ...defaultForm });

function parseRequiredPositiveInteger(value: number, fieldName: string): number {
		if (value === null || value === undefined || value === 0) {
			throw new Error(t('customers.fieldRequired', { field: fieldName }));
		}
		const parsedValue = Number(value);
		if (!Number.isFinite(parsedValue) || !Number.isInteger(parsedValue) || parsedValue <= 0) {
			throw new Error(t('customers.fieldPositiveInt', { field: fieldName }));
		}
		return parsedValue;
	}
	function toPayload(form: CustomerForm): NewCustomer {
		return {
			taxCode: parseRequiredPositiveInteger(form.taxCode, t('customers.taxCode')),
			ordinalNumber: parseRequiredPositiveInteger(form.ordinalNumber, t('customers.phOrdinal')),
			typology: form.typology,
			vatNumber: form.vatNumber.trim() || null,
			address: form.address.trim(),
			provinceName: form.provinceName.trim(),
			municipalityName: form.municipalityName.trim()
		};
	}

	function loadFormFromCustomer(customer: Customer): CustomerForm {
		return {
			taxCode: customer.taxCode,
			ordinalNumber: customer.ordinalNumber,
			typology: customer.typology,
			vatNumber: customer.vatNumber ?? '',
			address: customer.address,
			provinceName: customer.provinceName,
			municipalityName: customer.municipalityName
		};
	}

	function selectedTypologyFilter(): CustomerTypology | null {
		if (typologyFilter === 'all') return null;
		return typologyFilter;
	}

	async function loadPage(page: number): Promise<void> {
		loading = true;

		try {
			const result = await getCustomers(
				page,
				pageSize,
				selectedTypologyFilter(),
				taxCodeSearch,
				vatSearch
			);
			customers = result.items;
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

	// Live search while typing, but skip when the only term entered is a single char (too broad).
	// Enter/Search forces it. Cleared fields (length 0) reload the full list.
	let searchTimer: ReturnType<typeof setTimeout> | undefined;
	function onSearchInput(): void {
		clearTimeout(searchTimer);
		const terms = [taxCodeSearch, vatSearch].map((t) => t.trim()).filter((t) => t.length > 0);
		if (terms.length > 0 && !terms.some((t) => t.length >= 2)) return;
		searchTimer = setTimeout(() => onApplyFilters(), 250);
	}

	async function onResetFilters(): Promise<void> {
		taxCodeSearch = '';
		vatSearch = '';
		typologyFilter = 'all';
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

	async function onCreateCustomer(): Promise<void> {
		saving = true;
		n.error = null;
		n.success = null;

		try {
			await createCustomer(toPayload(newForm));
			closeCreateForm();
			await loadPage(1);
		} catch (err) {
			n.error = String(err);
		} finally {
			saving = false;
		}
	}

	function startEdit(customer: Customer): void {
		editingId = customer.id;
		editForm = loadFormFromCustomer(customer);
	}

	function cancelEdit(): void {
		editingId = null;
		editForm = { ...defaultForm };
	}

	async function onSaveEdit(id: number): Promise<void> {
		saving = true;
		n.error = null;
		n.success = null;

		try {
			await updateCustomer(id, toPayload(editForm));
			cancelEdit();
			await loadPage(currentPage);
		} catch (err) {
			n.error = String(err);
		} finally {
			saving = false;
		}
	}

	async function onDeleteCustomer(event: MouseEvent, id: number): Promise<void> {
		event.preventDefault();
		event.stopPropagation();

		const confirmed = await confirmDialog(t('customers.deleteConfirm'), {
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
			await deleteCustomer(id);
			const targetPage = customers.length === 1 && currentPage > 1 ? currentPage - 1 : currentPage;
			await loadPage(targetPage);
		} catch (err) {
			n.error = String(err);
		} finally {
			saving = false;
		}
	}

	function formatSkippedRows(invalidRows: InvalidUploadRow[]): string | null {
		if (invalidRows.length === 0) return null;
		const lines = invalidRows.map((row) =>
			/^row\s+\d+:/i.test(row.message)
				? row.message
				: t('customers.rowPrefix', { n: row.rowNumber, msg: row.message })
		);
		return `${t('customers.skipped', { n: invalidRows.length })}\n${lines.join('\n')}`;
	}

	async function onUploadCustomersExcel(): Promise<void> {
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
			const validation = await validateCustomersExcel(selected);
			const skippedMessage = formatSkippedRows(validation.invalidRows);

			const confirmed = await confirmDialog(t('customers.uploadReplaceWarning'), {
				title: t('common.confirmDeletion'),
				kind: 'warning',
				okLabel: t('common.ok'),
				cancelLabel: t('common.cancel')
			});
			if (!confirmed) return;

			if (validation.ambiguousRows.length > 0) {
				pendingUploadPath = selected;
				pendingSkippedMessage = skippedMessage;
				ambiguousRows = validation.ambiguousRows;
				provinceSelections = {};
				provinceError = null;
				showProvinceResolutionModal = true;
				return;
			}

			n.success = await confirmCustomersExcelUpload(selected, []);
			n.error = skippedMessage;
			await loadPage(1);
		} catch (err) {
			n.error = String(err);
		} finally {
			saving = false;
			uploadingExcel = false;
		}
	}

	function closeProvinceResolutionModal(): void {
		showProvinceResolutionModal = false;
		pendingUploadPath = null;
		pendingSkippedMessage = null;
		ambiguousRows = [];
		provinceSelections = {};
		provinceError = null;
	}

	function setProvinceSelection(rowNumber: number, value: string): void {
		provinceSelections = {
			...provinceSelections,
			[rowNumber]: value
		};
	}

	async function onConfirmProvinceResolutions(): Promise<void> {
		if (!pendingUploadPath) {
			return;
		}

		const missingRows = ambiguousRows.filter((row) => {
			const selectedProvince = provinceSelections[row.rowNumber] ?? '';
			return selectedProvince.trim().length === 0;
		});

		if (missingRows.length > 0) {
			const rows = missingRows.map((row) => row.rowNumber).join(', ');
			provinceError = t('customers.selectProvinceFor', { rows });
			return;
		}

		saving = true;
		uploadingExcel = true;
		provinceError = null;
		n.error = null;
		n.success = null;

		try {
			const resolutions: ProvinceResolution[] = ambiguousRows.map((row) => ({
				rowNumber: row.rowNumber,
				provinceName: provinceSelections[row.rowNumber]
			}));

			n.success = await confirmCustomersExcelUpload(pendingUploadPath, resolutions);
			n.error = pendingSkippedMessage;
			closeProvinceResolutionModal();
			await loadPage(1);
		} catch (err) {
			provinceError = String(err);
		} finally {
			saving = false;
			uploadingExcel = false;
		}
	}

	$effect(() => {
		loadPage(1);
	});
</script>

<div class="h-full bg-gray-50">
	<main class="mx-auto max-w-7xl px-6 py-8">
		<section class="rounded-2xl border border-slate-200 bg-white p-5 shadow-sm md:p-6">
			<div class="mb-5 flex items-center justify-between gap-4">
				<h2 class="text-lg font-semibold text-slate-900">{t('customers.title')}</h2>
				<div class="flex items-center gap-2">
					<button
						type="button"
						onclick={onUploadCustomersExcel}
						class="inline-flex items-center gap-2 rounded-lg bg-emerald-700 px-4 py-2 text-sm font-medium text-white hover:bg-emerald-800 disabled:opacity-50"
						disabled={saving}
					>
						{#if uploadingExcel}<Spinner class="h-4 w-4" />{/if}
						{t('common.uploadExcel')}
					</button>
					<button
						type="button"
						onclick={openCreateForm}
						class="rounded-lg bg-blue-700 px-4 py-2 text-sm font-medium text-white hover:bg-blue-800 disabled:opacity-50"
						disabled={saving}
					>
						{t('customers.add')}
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

			<Notice notice={n} />

			<form
				onsubmit={(event) => {
					event.preventDefault();
					onApplyFilters();
				}}
				class="mb-4 grid gap-2 rounded-xl border border-slate-200 bg-slate-50 p-3 md:grid-cols-[2fr_2fr_1fr_auto_auto]"
			>
				<input
					type="text"
					placeholder={t('customers.searchByTax')}
					bind:value={taxCodeSearch}
					oninput={onSearchInput}
					class="rounded-md border border-slate-300 px-3 py-2 text-sm"
				/>
				<input
					type="text"
					placeholder={t('customers.searchByVat')}
					bind:value={vatSearch}
					oninput={onSearchInput}
					class="rounded-md border border-slate-300 px-3 py-2 text-sm"
				/>
				<select bind:value={typologyFilter} class="rounded-md border border-slate-300 px-3 py-2 text-sm">
					<option value="all">{t('customers.allTypologies')}</option>
					{#each CUSTOMER_TYPOLOGIES as typology}
						<option value={typology}>{typology}</option>
					{/each}
				</select>
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
						onCreateCustomer();
					}}
					class="mb-6 grid gap-3 rounded-xl border border-blue-200 bg-blue-50 p-4 md:grid-cols-4"
				>
					<input type="number" required placeholder={t('customers.phTaxCode')} bind:value={newForm.taxCode} class="rounded-md border border-slate-300 px-3 py-2 text-sm" />
					<input type="number" required placeholder={t('customers.phOrdinal')} bind:value={newForm.ordinalNumber} class="rounded-md border border-slate-300 px-3 py-2 text-sm" />
					<select bind:value={newForm.typology} class="rounded-md border border-slate-300 px-3 py-2 text-sm">
						{#each CUSTOMER_TYPOLOGIES as typology}
							<option value={typology}>{typology}</option>
						{/each}
					</select>
					<input type="text" placeholder={t('customers.phVat')} bind:value={newForm.vatNumber} class="rounded-md border border-slate-300 px-3 py-2 text-sm" />
					<input type="text" required placeholder={t('customers.phAddress')} bind:value={newForm.address} class="rounded-md border border-slate-300 px-3 py-2 text-sm md:col-span-2" />
					<input type="text" required placeholder={t('customers.phProvince')} bind:value={newForm.provinceName} class="rounded-md border border-slate-300 px-3 py-2 text-sm" />
					<input type="text" required placeholder={t('customers.phMunicipality')} bind:value={newForm.municipalityName} class="rounded-md border border-slate-300 px-3 py-2 text-sm" />

					<div class="flex gap-2 md:col-span-4">
						<button type="submit" class="rounded-md bg-blue-700 px-3 py-2 text-sm font-medium text-white hover:bg-blue-800 disabled:opacity-50" disabled={saving}>
							{t('common.save')}
						</button>
						<button type="button" onclick={closeCreateForm} class="rounded-md border border-slate-300 px-3 py-2 text-sm text-slate-700 hover:bg-slate-100">
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
							<th class="px-3 py-3">{t('customers.taxCode')}</th>
							<th class="px-3 py-3">{t('customers.ordinal')}</th>
							<th class="px-3 py-3">{t('customers.typology')}</th>
							<th class="px-3 py-3">{t('customers.vat')}</th>
							<th class="px-3 py-3">{t('customers.address')}</th>
							<th class="px-3 py-3">{t('customers.municipality')}</th>
							<th class="px-3 py-3">{t('customers.province')}</th>
							<th class="px-3 py-3">{t('common.actions')}</th>
						</tr>
					</thead>
					<tbody>
						{#if loading}
							<tr>
								<td colspan="9" class="px-3 py-6 text-center text-sm text-slate-500">{t('customers.loading')}</td>
							</tr>
						{:else if customers.length === 0}
							<tr>
								<td colspan="9" class="px-3 py-6 text-center text-sm text-slate-500">{t('customers.none')}</td>
							</tr>
						{:else}
							{#each customers as customer}
								<tr class="border-b border-slate-100 align-top">
									<td class="px-3 py-3 text-sm text-slate-700">{customer.id}</td>
									{#if editingId === customer.id}
										<td class="px-3 py-3"><input type="number" bind:value={editForm.taxCode} class="w-28 rounded-md border border-slate-300 px-2 py-1 text-sm" /></td>
										<td class="px-3 py-3"><input type="number" bind:value={editForm.ordinalNumber} class="w-28 rounded-md border border-slate-300 px-2 py-1 text-sm" /></td>
										<td class="px-3 py-3">
											<select bind:value={editForm.typology} class="rounded-md border border-slate-300 px-2 py-1 text-sm">
												{#each CUSTOMER_TYPOLOGIES as typology}
													<option value={typology}>{typology}</option>
												{/each}
											</select>
										</td>
										<td class="px-3 py-3"><input bind:value={editForm.vatNumber} class="w-32 rounded-md border border-slate-300 px-2 py-1 text-sm" /></td>
										<td class="px-3 py-3"><input bind:value={editForm.address} class="w-52 rounded-md border border-slate-300 px-2 py-1 text-sm" /></td>
										<td class="px-3 py-3"><input bind:value={editForm.municipalityName} class="w-40 rounded-md border border-slate-300 px-2 py-1 text-sm" /></td>
										<td class="px-3 py-3"><input bind:value={editForm.provinceName} class="w-32 rounded-md border border-slate-300 px-2 py-1 text-sm" /></td>
										<td class="px-3 py-3">
											<div class="flex gap-2">
												<button onclick={() => onSaveEdit(customer.id)} class="rounded-md bg-emerald-700 px-3 py-1 text-xs font-medium text-white hover:bg-emerald-800">{t('common.save')}</button>
												<button onclick={cancelEdit} class="rounded-md border border-slate-300 px-3 py-1 text-xs text-slate-700 hover:bg-slate-100">{t('common.cancel')}</button>
											</div>
										</td>
									{:else}
										<td class="px-3 py-3 text-sm text-slate-700">{customer.taxCode}</td>
										<td class="px-3 py-3 text-sm text-slate-700">{customer.ordinalNumber}</td>
										<td class="px-3 py-3 text-sm text-slate-700">{customer.typology}</td>
										<td class="px-3 py-3 text-sm text-slate-700">{customer.vatNumber ?? '-'}</td>
										<td class="px-3 py-3 text-sm text-slate-700">{customer.address}</td>
										<td class="px-3 py-3 text-sm text-slate-700">{customer.municipalityName}</td>
										<td class="px-3 py-3 text-sm text-slate-700">{customer.provinceName}</td>
										<td class="px-3 py-3">
											<div class="flex gap-2">
												<button onclick={() => startEdit(customer)} class="rounded-md bg-amber-600 px-3 py-1 text-xs font-medium text-white hover:bg-amber-700">{t('common.edit')}</button>
												<button type="button" onclick={(event) => onDeleteCustomer(event, customer.id)} class="rounded-md bg-rose-700 px-3 py-1 text-xs font-medium text-white hover:bg-rose-800">{t('common.delete')}</button>
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

	{#if showProvinceResolutionModal}
		<div class="fixed inset-0 z-50 flex items-center justify-center bg-slate-900/40 p-4">
			<div class="w-full max-w-5xl rounded-xl border border-slate-200 bg-white p-5 shadow-xl">
				<h3 class="text-base font-semibold text-slate-900">{t('customers.modalTitle')}</h3>
				<p class="mt-1 text-sm text-slate-600">{t('customers.modalHint')}</p>

				{#if provinceError}
					<p class="mt-3 whitespace-pre-line rounded-lg border border-red-200 bg-red-50 px-3 py-2 text-sm text-red-700">{provinceError}</p>
				{/if}

				<div class="mt-4 max-h-[55vh] overflow-auto rounded-lg border border-slate-200">
					<table class="min-w-full border-collapse">
						<thead>
							<tr class="border-b border-slate-200 text-left text-xs uppercase tracking-wide text-slate-500">
								<th class="px-3 py-2">{t('customers.row')}</th>
								<th class="px-3 py-2">{t('customers.taxCode')}</th>
								<th class="px-3 py-2">{t('customers.typology')}</th>
								<th class="px-3 py-2">{t('customers.municipality')}</th>
								<th class="px-3 py-2">{t('customers.address')}</th>
								<th class="px-3 py-2">{t('customers.province')}</th>
							</tr>
						</thead>
						<tbody>
							{#each ambiguousRows as row}
								<tr class="border-b border-slate-100 align-top">
									<td class="px-3 py-2 text-sm text-slate-700">{row.rowNumber}</td>
									<td class="px-3 py-2 text-sm text-slate-700">{row.taxCode}</td>
									<td class="px-3 py-2 text-sm text-slate-700">{row.typology}</td>
									<td class="px-3 py-2 text-sm text-slate-700">{row.municipalityName}</td>
									<td class="px-3 py-2 text-sm text-slate-700">{row.address}</td>
									<td class="px-3 py-2">
										<select
											value={provinceSelections[row.rowNumber] ?? ''}
											onchange={(event) =>
												setProvinceSelection(row.rowNumber, (event.currentTarget as HTMLSelectElement).value)}
											class="w-full rounded-md border border-slate-300 px-2 py-1 text-sm"
										>
											<option value="">{t('customers.selectProvince')}</option>
											{#each row.candidateProvinces as province}
												<option value={province}>{province}</option>
											{/each}
										</select>
									</td>
								</tr>
							{/each}
						</tbody>
					</table>
				</div>

				<div class="mt-4 flex justify-end gap-2">
					<button
						type="button"
						onclick={closeProvinceResolutionModal}
						class="rounded-md border border-slate-300 px-3 py-2 text-sm text-slate-700 hover:bg-slate-100"
						disabled={saving}
					>
						{t('common.cancel')}
					</button>
					<button
						type="button"
						onclick={onConfirmProvinceResolutions}
						class="inline-flex items-center gap-2 rounded-md bg-blue-700 px-3 py-2 text-sm font-medium text-white hover:bg-blue-800 disabled:opacity-50"
						disabled={saving}
					>
						{#if uploadingExcel}<Spinner class="h-4 w-4" />{/if}
						{t('customers.confirmUpload')}
					</button>
				</div>
			</div>
		</div>
	{/if}
</div>
