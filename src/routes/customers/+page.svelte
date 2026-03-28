<script lang="ts">
	import { confirm as confirmDialog, open as openDialog } from '@tauri-apps/plugin-dialog';
	import {
		createCustomer,
		deleteCustomer,
		getCustomers,
		uploadCustomersExcel,
		updateCustomer,
		type Customer,
		type CustomerTypology,
		CUSTOMER_TYPOLOGIES,
		type NewCustomer
	} from '$lib/customer-repository';

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
	let errorMsg = $state<string | null>(null);
	let successMsg = $state<string | null>(null);

	let showCreateForm = $state(false);
	let newForm = $state<CustomerForm>({ ...defaultForm });

	let editingId = $state<number | null>(null);
	let editForm = $state<CustomerForm>({ ...defaultForm });

	function toPayload(form: CustomerForm): NewCustomer {
		return {
			taxCode: Number(form.taxCode),
			ordinalNumber: Number(form.ordinalNumber),
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

	async function loadPage(page: number): Promise<void> {
		loading = true;
		errorMsg = null;

		try {
			const result = await getCustomers(page, pageSize);
			customers = result.items;
			currentPage = result.page;
			hasNextPage = result.hasNextPage;
		} catch (err) {
			errorMsg = String(err);
		} finally {
			loading = false;
		}
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
		errorMsg = null;
		successMsg = null;

		try {
			await createCustomer(toPayload(newForm));
			closeCreateForm();
			await loadPage(1);
		} catch (err) {
			errorMsg = String(err);
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
		errorMsg = null;
		successMsg = null;

		try {
			await updateCustomer(id, toPayload(editForm));
			cancelEdit();
			await loadPage(currentPage);
		} catch (err) {
			errorMsg = String(err);
		} finally {
			saving = false;
		}
	}

	async function onDeleteCustomer(event: MouseEvent, id: number): Promise<void> {
		event.preventDefault();
		event.stopPropagation();

		const confirmed = await confirmDialog('Delete this customer?', {
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
			await deleteCustomer(id);
			const targetPage = customers.length === 1 && currentPage > 1 ? currentPage - 1 : currentPage;
			await loadPage(targetPage);
		} catch (err) {
			errorMsg = String(err);
		} finally {
			saving = false;
		}
	}

	async function onUploadCustomersExcel(): Promise<void> {
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
			successMsg = await uploadCustomersExcel(selected);
			window.alert(successMsg);
			await loadPage(1);
		} catch (err) {
			errorMsg = String(err);
			window.alert(`Upload failed: ${errorMsg}`);
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
		<section class="rounded-2xl border border-slate-200 bg-white p-5 shadow-sm md:p-6">
			<div class="mb-5 flex items-center justify-between gap-4">
				<h2 class="text-lg font-semibold text-slate-900">Customer Table</h2>
				<div class="flex items-center gap-2">
					<button
						type="button"
						onclick={onUploadCustomersExcel}
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
						Add Customer
					</button>
				</div>
			</div>

			{#if showCreateForm}
				<form
					onsubmit={(event) => {
						event.preventDefault();
						onCreateCustomer();
					}}
					class="mb-6 grid gap-3 rounded-xl border border-blue-200 bg-blue-50 p-4 md:grid-cols-4"
				>
					<input type="number" required placeholder="Tax code" bind:value={newForm.taxCode} class="rounded-md border border-slate-300 px-3 py-2 text-sm" />
					<input type="number" required placeholder="Ordinal number" bind:value={newForm.ordinalNumber} class="rounded-md border border-slate-300 px-3 py-2 text-sm" />
					<select bind:value={newForm.typology} class="rounded-md border border-slate-300 px-3 py-2 text-sm">
						{#each CUSTOMER_TYPOLOGIES as typology}
							<option value={typology}>{typology}</option>
						{/each}
					</select>
					<input type="text" placeholder="VAT number (optional)" bind:value={newForm.vatNumber} class="rounded-md border border-slate-300 px-3 py-2 text-sm" />
					<input type="text" required placeholder="Address" bind:value={newForm.address} class="rounded-md border border-slate-300 px-3 py-2 text-sm md:col-span-2" />
					<input type="text" required placeholder="Province name" bind:value={newForm.provinceName} class="rounded-md border border-slate-300 px-3 py-2 text-sm" />
					<input type="text" required placeholder="Municipality name" bind:value={newForm.municipalityName} class="rounded-md border border-slate-300 px-3 py-2 text-sm" />

					<div class="flex gap-2 md:col-span-4">
						<button type="submit" class="rounded-md bg-blue-700 px-3 py-2 text-sm font-medium text-white hover:bg-blue-800 disabled:opacity-50" disabled={saving}>
							Save
						</button>
						<button type="button" onclick={closeCreateForm} class="rounded-md border border-slate-300 px-3 py-2 text-sm text-slate-700 hover:bg-slate-100">
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
							<th class="px-3 py-3">Tax Code</th>
							<th class="px-3 py-3">Ordinal</th>
							<th class="px-3 py-3">Typology</th>
							<th class="px-3 py-3">VAT</th>
							<th class="px-3 py-3">Address</th>
							<th class="px-3 py-3">Municipality</th>
							<th class="px-3 py-3">Province</th>
							<th class="px-3 py-3">Actions</th>
						</tr>
					</thead>
					<tbody>
						{#if loading}
							<tr>
								<td colspan="9" class="px-3 py-6 text-center text-sm text-slate-500">Loading customers...</td>
							</tr>
						{:else if customers.length === 0}
							<tr>
								<td colspan="9" class="px-3 py-6 text-center text-sm text-slate-500">No customers found.</td>
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
												<button onclick={() => onSaveEdit(customer.id)} class="rounded-md bg-emerald-700 px-3 py-1 text-xs font-medium text-white hover:bg-emerald-800">Save</button>
												<button onclick={cancelEdit} class="rounded-md border border-slate-300 px-3 py-1 text-xs text-slate-700 hover:bg-slate-100">Cancel</button>
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
												<button onclick={() => startEdit(customer)} class="rounded-md bg-amber-600 px-3 py-1 text-xs font-medium text-white hover:bg-amber-700">Edit</button>
												<button type="button" onclick={(event) => onDeleteCustomer(event, customer.id)} class="rounded-md bg-rose-700 px-3 py-1 text-xs font-medium text-white hover:bg-rose-800">Delete</button>
											</div>
										</td>
									{/if}
								</tr>
							{/each}
						{/if}
					</tbody>
				</table>
			</div>

			<div class="mt-5 flex items-center justify-end">
				{#if currentPage === 1}
					<button onclick={() => loadPage(2)} disabled={!hasNextPage || loading || saving} class="rounded-lg bg-slate-900 px-4 py-2 text-sm font-medium text-white hover:bg-slate-700 disabled:opacity-40">
						Go to the second page
					</button>
				{:else}
					<button onclick={() => loadPage(1)} disabled={loading || saving} class="rounded-lg border border-slate-300 px-4 py-2 text-sm font-medium text-slate-700 hover:bg-slate-100 disabled:opacity-40">
						Go back to the first page
					</button>
				{/if}
			</div>

			{#if errorMsg}
				<p class="mt-4 rounded-lg border border-red-200 bg-red-50 px-3 py-2 text-sm text-red-700">{errorMsg}</p>
			{/if}

			{#if successMsg}
				<p class="mt-4 rounded-lg border border-emerald-200 bg-emerald-50 px-3 py-2 text-sm text-emerald-700">{successMsg}</p>
			{/if}
		</section>
	</main>
</div>
