import { invoke } from '@tauri-apps/api/core';

export type ProductType = 'pli' | 'pat';

export type Product = {
	id: number;
	code: string;
	description: string;
	units: number;
	productType: ProductType;
	capacity: number | null;
	nicotine: number | null;
	packages: number | null;
	admCode: string | null;
	tabella: number | null;
};

export type NewProduct = {
	productType: ProductType;
	code: string;
	description: string;
	units: number;
	capacity?: number | null;
	nicotine?: number | null;
	packages?: number | null;
	admCode?: string | null;
	tabella?: number | null;
};

export type UpdateProduct = {
	code?: string;
	description?: string;
	units?: number;
	capacity?: number | null;
	nicotine?: number | null;
	packages?: number | null;
};

export type PaginatedProducts = {
	items: Product[];
	page: number;
	pageSize: number;
	hasNextPage: boolean;
};

export async function createProduct(input: NewProduct): Promise<number> {
	return invoke<number>('create_product', { input });
}

export async function getProducts(
	page = 1,
	pageSize = 50,
	productTypeFilter?: ProductType | null,
	incompleteOnly = false,
	codeSearch?: string
): Promise<PaginatedProducts> {
	return invoke<PaginatedProducts>('get_products', {
		page,
		pageSize,
		productTypeFilter: productTypeFilter ?? undefined,
		incompleteOnly,
		codeSearch: codeSearch?.trim() || undefined
	});
}

export async function getProductByCode(
	code: string,
	productTypeFilter?: ProductType | null
): Promise<Product | null> {
	if (!code.trim()) return null;
	return invoke<Product | null>('get_product_by_code', {
		code,
		productTypeFilter: productTypeFilter ?? undefined
	});
}

export async function getProductById(id: number): Promise<Product | null> {
	return invoke<Product | null>('get_product_by_id', { id });
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

export async function uploadSkeletonExcel(filePath: string): Promise<string> {
	return invoke<string>('upload_skeleton_excel', { filePath });
}
