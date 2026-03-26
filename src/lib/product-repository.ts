import Database from '@tauri-apps/plugin-sql';

const DATABASE_URL = 'sqlite:products.db';

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

type ProductRow = {
	id: number;
	code: string;
	description: string;
	units: number;
	pli: number;
};

async function getDb() {
	return Database.load(DATABASE_URL);
}

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
	const db = await getDb();
	await db.execute(
		'INSERT INTO product (code, description, units, pli) VALUES ($1, $2, $3, $4)',
		[input.code, input.description, input.units, input.pli ? 1 : 0]
	);

	const rows = await db.select<{ id: number }[]>('SELECT last_insert_rowid() AS id');
	return rows[0]?.id ?? 0;
}

export async function getProducts(): Promise<Product[]> {
	const db = await getDb();
	const rows = await db.select<ProductRow[]>(
		'SELECT id, code, description, units, pli FROM product ORDER BY id DESC'
	);
	return rows.map(mapProduct);
}

export async function getProductById(id: number): Promise<Product | null> {
	const db = await getDb();
	const rows = await db.select<ProductRow[]>(
		'SELECT id, code, description, units, pli FROM product WHERE id = $1 LIMIT 1',
		[id]
	);
	if (rows.length === 0) return null;
	return mapProduct(rows[0]);
}

export async function updateProduct(id: number, input: UpdateProduct): Promise<boolean> {
	const db = await getDb();
	const existing = await getProductById(id);
	if (!existing) return false;

	const nextCode = input.code ?? existing.code;
	const nextDescription = input.description ?? existing.description;
	const nextUnits = input.units ?? existing.units;
	const nextPli = input.pli ?? existing.pli;

	const result = await db.execute(
		'UPDATE product SET code = $1, description = $2, units = $3, pli = $4 WHERE id = $5',
		[nextCode, nextDescription, nextUnits, nextPli ? 1 : 0, id]
	);

	return result.rowsAffected > 0;
}

export async function deleteProduct(id: number): Promise<boolean> {
	const db = await getDb();
	const result = await db.execute('DELETE FROM product WHERE id = $1', [id]);
	return result.rowsAffected > 0;
}
