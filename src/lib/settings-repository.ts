import { invoke } from '@tauri-apps/api/core';

export type AccisaCoefficients = {
	pliPln: number;
	pliPl: number;
	pat: number;
};

export async function getAccisaCoefficients(): Promise<AccisaCoefficients> {
	return invoke<AccisaCoefficients>('get_accisa_coefficients');
}

export async function saveAccisaCoefficients(coefficients: AccisaCoefficients): Promise<void> {
	return invoke<void>('save_accisa_coefficients', { coefficients });
}
