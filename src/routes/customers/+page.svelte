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
	import TableSkeleton from '$lib/TableSkeleton.svelte';
	import { btnPrimary, btnSecondary, btnPrimarySm, btnSecondarySm, iconBtn, iconBtnDanger, inputBase, inputCell } from '$lib/ui';

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
	let pageSize = $state(20);
	let hasNextPage = $state(false);
	let totalCount = $state(0);
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
	let dialogEl = $state<HTMLDialogElement | null>(null);

	const allProvincesResolved = $derived(
		ambiguousRows.length > 0 &&
			ambiguousRows.every((row) => (provinceSelections[row.rowNumber] ?? '').trim().length > 0)
	);

	const hasActiveFilters = $derived(
		taxCodeSearch.trim().length > 0 || vatSearch.trim().length > 0 || typologyFilter !== 'all'
	);

	const totalPages = $derived(Math.max(1, Math.ceil(totalCount / pageSize)));

	// Drive the native <dialog> from state so ESC / backdrop / focus-trap come for free.
	$effect(() => {
		const el = dialogEl;
		if (!el) return;
		if (showProvinceResolutionModal && !el.open) el.showModal();
		else if (!showProvinceResolutionModal && el.open) el.close();
	});

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
			totalCount = result.totalCount;
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
		n.warning = null;

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
			n.warning = skippedMessage;
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
		if (!pendingUploadPath || !allProvincesResolved) {
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
			n.warning = pendingSkippedMessage;
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

<div class="h-full bg-slate-50">
	<main class="mx-auto max-w-7xl px-6 py-10">
		<header class="mb-6 flex flex-wrap items-start justify-between gap-4">
			<div class="min-w-0">
				<h1 class="text-xl font-semibold tracking-tight text-slate-900">{t('customers.title')}</h1>
				<p class="mt-1 max-w-prose text-sm text-slate-500">{t('customers.subtitle')}</p>
			</div>
			<div class="flex flex-wrap items-center gap-2">
				<button type="button" onclick={openCreateForm} class={btnSecondary} disabled={saving}>
					{t('customers.add')}
				</button>
				<button type="button" onclick={onUploadCustomersExcel} class={btnPrimary} disabled={saving}>
					{#if uploadingExcel}<Spinner class="h-4 w-4" />{/if}
					{t('common.uploadExcel')}
				</button>
			</div>
		</header>

		<Notice notice={n} />

		<form
			onsubmit={(event) => {
				event.preventDefault();
				onApplyFilters();
			}}
			class="mb-4 grid gap-2 md:grid-cols-[2fr_2fr_1fr_auto_auto]"
		>
			<input
				type="text"
				placeholder={t('customers.searchByTax')}
				bind:value={taxCodeSearch}
				oninput={onSearchInput}
				class={inputBase}
			/>
			<input
				type="text"
				placeholder={t('customers.searchByVat')}
				bind:value={vatSearch}
				oninput={onSearchInput}
				class={inputBase}
			/>
			<select bind:value={typologyFilter} class={inputBase}>
				<option value="all">{t('customers.allTypologies')}</option>
				{#each CUSTOMER_TYPOLOGIES as typology}
					<option value={typology}>{typology}</option>
				{/each}
			</select>
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
						onCreateCustomer();
					}}
					class="grid gap-3 border-b border-slate-200 bg-slate-50 p-4 md:grid-cols-4"
				>
					<label class="flex flex-col gap-1 text-xs font-medium text-slate-600">
						{t('customers.taxCode')}
						<input type="number" required bind:value={newForm.taxCode} class="{inputBase} font-normal" />
					</label>
					<label class="flex flex-col gap-1 text-xs font-medium text-slate-600">
						{t('customers.ordinal')}
						<input type="number" required bind:value={newForm.ordinalNumber} class="{inputBase} font-normal" />
					</label>
					<label class="flex flex-col gap-1 text-xs font-medium text-slate-600">
						{t('customers.typology')}
						<select bind:value={newForm.typology} class="{inputBase} font-normal">
							{#each CUSTOMER_TYPOLOGIES as typology}
								<option value={typology}>{typology}</option>
							{/each}
						</select>
					</label>
					<label class="flex flex-col gap-1 text-xs font-medium text-slate-600">
						{t('customers.vat')}
						<input type="text" bind:value={newForm.vatNumber} class="{inputBase} font-normal" />
					</label>
					<label class="flex flex-col gap-1 text-xs font-medium text-slate-600 md:col-span-2">
						{t('customers.address')}
						<input type="text" required bind:value={newForm.address} class="{inputBase} font-normal" />
					</label>
					<label class="flex flex-col gap-1 text-xs font-medium text-slate-600">
						{t('customers.province')}
						<input type="text" required bind:value={newForm.provinceName} class="{inputBase} font-normal" />
					</label>
					<label class="flex flex-col gap-1 text-xs font-medium text-slate-600">
						{t('customers.municipality')}
						<input type="text" required bind:value={newForm.municipalityName} class="{inputBase} font-normal" />
					</label>

					<div class="flex gap-2 md:col-span-4">
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
							<TableSkeleton cols={8} />
						{:else if customers.length === 0}
							<tr>
								<td colspan="8" class="px-3 py-16">
									<div class="mx-auto flex max-w-sm flex-col items-center text-center">
										{#if hasActiveFilters}
											<p class="text-sm text-slate-500">{t('customers.emptyFiltered')}</p>
											<button type="button" onclick={onResetFilters} class="{btnSecondarySm} mt-3">{t('common.reset')}</button>
										{:else}
											<p class="text-sm font-medium text-slate-700">{t('customers.emptyTitle')}</p>
											<p class="mt-1 text-sm text-slate-500">{t('customers.emptyHint')}</p>
											<button type="button" onclick={onUploadCustomersExcel} class="{btnPrimary} mt-4" disabled={saving}>
												{#if uploadingExcel}<Spinner class="h-4 w-4" />{/if}
												{t('common.uploadExcel')}
											</button>
										{/if}
									</div>
								</td>
							</tr>
						{:else}
							{#each customers as customer}
								<tr class="border-b border-slate-100 align-top {editingId === customer.id ? 'bg-slate-50' : ''}">
									{#if editingId === customer.id}
										<td class="px-3 py-3"><input type="number" bind:value={editForm.taxCode} class="w-28 font-mono {inputCell}" /></td>
										<td class="px-3 py-3"><input type="number" bind:value={editForm.ordinalNumber} class="w-28 {inputCell}" /></td>
										<td class="px-3 py-3">
											<select bind:value={editForm.typology} class={inputCell}>
												{#each CUSTOMER_TYPOLOGIES as typology}
													<option value={typology}>{typology}</option>
												{/each}
											</select>
										</td>
										<td class="px-3 py-3"><input bind:value={editForm.vatNumber} class="w-32 font-mono {inputCell}" /></td>
										<td class="px-3 py-3"><input bind:value={editForm.address} class="w-52 {inputCell}" /></td>
										<td class="px-3 py-3"><input bind:value={editForm.municipalityName} class="w-40 {inputCell}" /></td>
										<td class="px-3 py-3"><input bind:value={editForm.provinceName} class="w-32 {inputCell}" /></td>
										<td class="px-3 py-3">
											<div class="flex gap-2">
												<button onclick={() => onSaveEdit(customer.id)} class={btnPrimarySm} disabled={saving}>{t('common.save')}</button>
												<button onclick={cancelEdit} class={btnSecondarySm}>{t('common.cancel')}</button>
											</div>
										</td>
									{:else}
										<td class="px-3 py-3 font-mono text-sm text-slate-700">{customer.taxCode}</td>
										<td class="px-3 py-3 text-sm text-slate-700">{customer.ordinalNumber}</td>
										<td class="px-3 py-3 text-sm text-slate-700">{customer.typology}</td>
										<td class="px-3 py-3 font-mono text-sm text-slate-700">{customer.vatNumber ?? '-'}</td>
										<td class="px-3 py-3 text-sm text-slate-700">{customer.address}</td>
										<td class="px-3 py-3 text-sm text-slate-700">{customer.municipalityName}</td>
										<td class="px-3 py-3 text-sm text-slate-700">{customer.provinceName}</td>
										<td class="px-3 py-3">
											<div class="flex gap-1">
												<button onclick={() => startEdit(customer)} class={iconBtn} title={t('common.edit')} aria-label={t('common.edit')}>
													<svg viewBox="0 0 20 20" class="size-4" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"><path d="M4 13.5V16h2.5l7-7-2.5-2.5-7 7z" /><path d="M11.5 6.5l2 2" /></svg>
												</button>
												<button type="button" onclick={(event) => onDeleteCustomer(event, customer.id)} class={iconBtnDanger} title={t('common.delete')} aria-label={t('common.delete')}>
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

	<dialog
		bind:this={dialogEl}
		onclose={closeProvinceResolutionModal}
		class="m-auto w-full max-w-5xl rounded-xl border border-slate-200 bg-white p-0 shadow-xl backdrop:bg-slate-900/40"
	>
		<div class="p-5">
			<h2 class="text-base font-semibold text-slate-900">{t('customers.needProvince', { n: ambiguousRows.length })}</h2>
			<p class="mt-1 text-sm text-slate-600">{t('customers.modalHint')}</p>

			{#if provinceError}
				<p role="alert" class="mt-3 whitespace-pre-line rounded-lg border border-rose-200 bg-rose-50 px-3 py-2 text-sm text-rose-700">{provinceError}</p>
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
							{@const resolved = (provinceSelections[row.rowNumber] ?? '').trim().length > 0}
							<tr class="border-b border-slate-100 align-top">
								<td class="px-3 py-2 text-sm text-slate-700">{row.rowNumber}</td>
								<td class="px-3 py-2 font-mono text-sm text-slate-700">{row.taxCode}</td>
								<td class="px-3 py-2 text-sm text-slate-700">{row.typology}</td>
								<td class="px-3 py-2 text-sm text-slate-700">{row.municipalityName}</td>
								<td class="px-3 py-2 text-sm text-slate-700">{row.address}</td>
								<td class="px-3 py-2">
									<select
										value={provinceSelections[row.rowNumber] ?? ''}
										onchange={(event) =>
											setProvinceSelection(row.rowNumber, (event.currentTarget as HTMLSelectElement).value)}
										class="w-full rounded-md border bg-white px-2 py-1 text-sm text-slate-800 focus:border-slate-500 focus:outline-none focus-visible:ring-2 focus-visible:ring-slate-400 {resolved ? 'border-slate-300' : 'border-amber-400 ring-1 ring-amber-200'}"
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
				<button type="button" onclick={closeProvinceResolutionModal} class={btnSecondary} disabled={saving}>
					{t('customers.abortImport')}
				</button>
				<button type="button" onclick={onConfirmProvinceResolutions} class={btnPrimary} disabled={saving || !allProvincesResolved}>
					{#if uploadingExcel}<Spinner class="h-4 w-4" />{/if}
					{t('customers.confirmUpload')}
				</button>
			</div>
		</div>
	</dialog>
</div>
