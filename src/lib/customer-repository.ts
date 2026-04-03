import { invoke } from '@tauri-apps/api/core';

export const CUSTOMER_TYPOLOGIES = [
	'ESERCIZIO DI VICINATO',
	'RIVENDITA',
	'FARMACIA',
	'PARAFARMACIA'
] as const;

export type CustomerTypology = (typeof CUSTOMER_TYPOLOGIES)[number];

export type Customer = {
	id: number;
	taxCode: number;
	ordinalNumber: number;
	typology: CustomerTypology;
	vatNumber: string | null;
	address: string;
	municipalityId: number;
	municipalityName: string;
	provinceName: string;
};

export type NewCustomer = {
	taxCode: number;
	ordinalNumber: number;
	typology: CustomerTypology;
	vatNumber?: string | null;
	address: string;
	provinceName: string;
	municipalityName: string;
};

export type UpdateCustomer = Partial<
	Omit<NewCustomer, 'typology'> & {
		typology: CustomerTypology;
	}
>;

export type PaginatedCustomers = {
	items: Customer[];
	page: number;
	pageSize: number;
	hasNextPage: boolean;
};

export type InvalidUploadRow = {
	rowNumber: number;
	message: string;
};

export type AmbiguousUploadRow = {
	rowNumber: number;
	taxCode: number;
	ordinalNumber: number;
	typology: CustomerTypology;
	vatNumber: string | null;
	address: string;
	municipalityName: string;
	candidateProvinces: string[];
};

export type ValidateCustomersExcelResult = {
	validRowsCount: number;
	ambiguousRows: AmbiguousUploadRow[];
	invalidRows: InvalidUploadRow[];
};

export type ProvinceResolution = {
	rowNumber: number;
	provinceName: string;
};

export async function createCustomer(input: NewCustomer): Promise<number> {
	return invoke<number>('create_customer', { input });
}

export async function getCustomers(
	page = 1,
	pageSize = 50,
	typologyFilter?: CustomerTypology | null
): Promise<PaginatedCustomers> {
	return invoke<PaginatedCustomers>('get_customers', {
		page,
		pageSize,
		typologyFilter: typologyFilter === null ? undefined : typologyFilter
	});
}

export async function getCustomerByTaxCode(taxCode: number): Promise<Customer | null> {
	if (!Number.isFinite(taxCode)) return null;

	return invoke<Customer | null>('get_customer_by_tax_code', { taxCode });
}

export async function getCustomerById(id: number): Promise<Customer | null> {
	return invoke<Customer | null>('get_customer_by_id', { id });
}

export async function updateCustomer(id: number, input: UpdateCustomer): Promise<boolean> {
	return invoke<boolean>('update_customer', { id, input });
}

export async function deleteCustomer(id: number): Promise<boolean> {
	return invoke<boolean>('delete_customer', { id });
}

export async function uploadCustomersExcel(filePath: string): Promise<string> {
	return invoke<string>('upload_customers_excel', { filePath });
}

export async function validateCustomersExcel(filePath: string): Promise<ValidateCustomersExcelResult> {
	return invoke<ValidateCustomersExcelResult>('validate_customers_excel', { filePath });
}

export async function confirmCustomersExcelUpload(
	filePath: string,
	resolutions: ProvinceResolution[]
): Promise<string> {
	return invoke<string>('confirm_customers_excel_upload', { filePath, resolutions });
}
