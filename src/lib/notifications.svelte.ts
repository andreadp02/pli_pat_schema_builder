export type Notice = { error: string | null; success: string | null };

// One notice per page; module-level $state survives client-side route navigation,
// so a banner is still shown when the user leaves the page and comes back.
export const notices: Record<'home' | 'products' | 'customers' | 'templates', Notice> = $state({
	home: { error: null, success: null },
	products: { error: null, success: null },
	customers: { error: null, success: null },
	templates: { error: null, success: null }
});
