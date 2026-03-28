import { invoke } from '@tauri-apps/api/core';

export type Product = {
	id: number;
	code: string;
	description: string;
	units: number;
	pli: boolean;
};

export type NewProduct = {
	code: string;
	description: string;
	units: number;
	pli: boolean;
};

export type UpdateProduct = Partial<NewProduct>;

export type PaginatedProducts = {
	items: Product[];
	page: number;
	pageSize: number;
	hasNextPage: boolean;
};

type ProductRow = {
	id: number;
	code: string;
	description: string;
	units: number;
	pli: number;
};

function mapProduct(row: ProductRow): Product {
	return {
		id: row.id,
		code: row.code,
		description: row.description,
		units: row.units,
		pli: !!row.pli
	};
}

export async function createProduct(input: NewProduct): Promise<number> {
	return invoke<number>('create_product', { input });
}

export async function getProducts(
	page = 1,
	pageSize = 50,
	pliFilter?: boolean | null
): Promise<PaginatedProducts> {
	return invoke<PaginatedProducts>('get_products', {
		page,
		pageSize,
		pliFilter: pliFilter === null ? undefined : pliFilter
	});
}

export async function getProductByCode(code: string): Promise<Product | null> {
	if (!code.trim()) return null;

	const row = await invoke<ProductRow | Product | null>('get_product_by_code', { code });
	if (!row) return null;
	if (typeof row.pli === 'boolean') {
		return row as Product;
	}
	return mapProduct(row as ProductRow);
}

export async function getProductById(id: number): Promise<Product | null> {
	const row = await invoke<ProductRow | Product | null>('get_product_by_id', { id });
	if (!row) return null;
	if (typeof row.pli === 'boolean') {
		return row as Product;
	}
	return mapProduct(row as ProductRow);
}

export async function updateProduct(id: number, input: UpdateProduct): Promise<boolean> {
	return invoke<boolean>('update_product', { id, input });
}

export async function deleteProduct(id: number): Promise<boolean> {
	return invoke<boolean>('delete_product', { id });
}

export async function uploadProductsExcel(filePath: string): Promise<string> {
	return invoke<string>('upload_products_excel', { filePath });
}
