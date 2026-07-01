import { invoke } from '@tauri-apps/api/core';

export type TemplateKind = 'pli' | 'pat';

export type TemplatesStatus = {
	pli: boolean;
	pat: boolean;
};

export async function getTemplatesStatus(): Promise<TemplatesStatus> {
	return invoke<TemplatesStatus>('get_templates_status');
}

export async function saveTemplate(kind: TemplateKind, filePath: string): Promise<void> {
	return invoke<void>('save_template', { kind, filePath });
}
