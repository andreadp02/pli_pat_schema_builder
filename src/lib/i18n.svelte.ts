// Minimal reactive i18n: two static languages, no dependency needed.
// Read strings via t()/getLocale() (reactive because the read happens during render);
// switch with setLocale(). Keys are typed from `en`, so `it` must cover exactly the
// same keys and every t() callsite is compile-checked (`npm run check`).

export type Locale = 'en' | 'it';

const en = {
	'nav.home': 'Home',
	'nav.products': 'Products',
	'nav.customers': 'Customers',
	'nav.templates': 'Templates',
	'win.minimize': 'Minimize window',
	'win.maximize': 'Maximize or restore window',
	'win.close': 'Close window',
	'lang.switch': 'Switch language',

	'common.save': 'Save',
	'common.cancel': 'Cancel',
	'common.delete': 'Delete',
	'common.edit': 'Edit',
	'common.reset': 'Reset',
	'common.search': 'Search',
	'common.previous': 'Previous',
	'common.next': 'Next',
	'common.page': 'Page {n}',
	'common.dismiss': 'Dismiss',
	'common.uploadExcel': 'Upload Excel',
	'common.id': 'ID',
	'common.actions': 'Actions',
	'common.confirmDeletion': 'Confirm deletion',
	'common.all': 'All',
	'common.excelFilter': 'Excel (.xlsx)',

	'home.pageTitle': 'Generate ADM records',
	'home.pageSubtitle': 'Compile the tracciati_pli and tracciati_pat files from this fortnight’s invoices.',
	'home.step1': 'Select invoices',
	'home.chooseInvoices': 'Choose .xlsx invoices',
	'home.selected': '{n} invoice(s) selected',
	'home.remove': 'Remove',
	'home.removeFile': 'Remove file',
	'home.step2': 'Fortnight end date',
	'home.fortnightHint': 'PAT: Data fine quindicina (this date). PLI: Data mese (its month).',
	'home.step3': 'Output folder',
	'home.chooseFolder': 'Choose folder',
	'home.step4': 'Generate',
	'home.generating': 'Generating…',
	'home.generate': 'Generate records',
	'home.filesGenerated': 'Files generated.',
	'home.open': 'Open ↗',
	'home.openTitle': 'Open {path}',
	'home.warnings': '{n} warning(s):',

	'products.title': 'Product Table',
	'products.uploadSkeleton': 'Upload Skeleton',
	'products.add': 'Add Product',
	'products.searchByCode': 'Search by code',
	'products.incompleteOnly': 'Incomplete only',
	'products.code': 'Code',
	'products.description': 'Description',
	'products.units': 'Units',
	'products.type': 'Type',
	'products.admCode': 'ADM Code',
	'products.capacity': 'Capacity (ml)',
	'products.nicotine': 'Nicotine (mg/ml)',
	'products.packages': 'Packages',
	'products.tabella': 'Tabella',
	'products.loading': 'Loading products...',
	'products.none': 'No products found.',
	'products.incompleteTag': 'Incomplete',
	'products.incompleteTitle': 'Skeleton not uploaded — excluded from tracciati',
	'products.codeEmpty': 'Product code cannot be empty.',
	'products.deleteConfirm': 'Delete this product?',

	'customers.title': 'Customer Table',
	'customers.add': 'Add Customer',
	'customers.searchByTax': 'Search by tax code',
	'customers.searchByVat': 'Search by VAT',
	'customers.allTypologies': 'All typologies',
	'customers.taxCode': 'Tax Code',
	'customers.ordinal': 'Ordinal',
	'customers.typology': 'Typology',
	'customers.vat': 'VAT',
	'customers.address': 'Address',
	'customers.municipality': 'Municipality',
	'customers.province': 'Province',
	'customers.phTaxCode': 'Tax code',
	'customers.phOrdinal': 'Ordinal number',
	'customers.phVat': 'VAT number (optional)',
	'customers.phAddress': 'Address',
	'customers.phProvince': 'Province name',
	'customers.phMunicipality': 'Municipality name',
	'customers.loading': 'Loading customers...',
	'customers.none': 'No customers found.',
	'customers.deleteConfirm': 'Delete this customer?',
	'customers.modalTitle': 'Resolve Province For Ambiguous Rows',
	'customers.modalHint':
		'Province is missing and municipality maps to multiple provinces. Select one province for each row.',
	'customers.row': 'Row',
	'customers.selectProvince': 'Select province',
	'customers.confirmUpload': 'Confirm Upload',
	'customers.fieldRequired': '{field} is required and must be greater than 0.',
	'customers.fieldPositiveInt': '{field} must be a positive integer.',
	'customers.skipped': '{n} row(s) skipped (not imported):',
	'customers.rowPrefix': 'Row {n}: {msg}',
	'customers.selectProvinceFor': 'Select a province for row(s): {rows}',

	'templates.title': 'ADM Templates',
	'templates.intro':
		'Upload the two Excel templates used to generate the records. They are saved in the app data and reused on every generation.',
	'templates.pliTitle': 'PLI Record',
	'templates.patTitle': 'PAT Record',
	'templates.pliDesc': 'ADM template for Prodotti Liquidi da Inalazione (tracciati_pli.xlsx).',
	'templates.patDesc': 'ADM template for Prodotti Accessori dei Tabacchi (tracciati_pat.xlsx).',
	'templates.loaded': 'Loaded',
	'templates.notLoaded': 'Not loaded',
	'templates.replace': 'Replace',
	'templates.upload': 'Upload',
	'templates.saved': '{kind} template saved.'
} as const;

const it: Record<keyof typeof en, string> = {
	'nav.home': 'Home',
	'nav.products': 'Prodotti',
	'nav.customers': 'Clienti',
	'nav.templates': 'Modelli',
	'win.minimize': 'Riduci a icona',
	'win.maximize': 'Ingrandisci o ripristina',
	'win.close': 'Chiudi finestra',
	'lang.switch': 'Cambia lingua',

	'common.save': 'Salva',
	'common.cancel': 'Annulla',
	'common.delete': 'Elimina',
	'common.edit': 'Modifica',
	'common.reset': 'Reimposta',
	'common.search': 'Cerca',
	'common.previous': 'Precedente',
	'common.next': 'Successivo',
	'common.page': 'Pagina {n}',
	'common.dismiss': 'Chiudi',
	'common.uploadExcel': 'Carica Excel',
	'common.id': 'ID',
	'common.actions': 'Azioni',
	'common.confirmDeletion': 'Conferma eliminazione',
	'common.all': 'Tutti',
	'common.excelFilter': 'Excel (.xlsx)',

	'home.pageTitle': 'Genera i tracciati ADM',
	'home.pageSubtitle': 'Compila i file tracciati_pli e tracciati_pat dalle fatture di questa quindicina.',
	'home.step1': 'Seleziona le fatture',
	'home.chooseInvoices': 'Scegli fatture .xlsx',
	'home.selected': '{n} fattura/e selezionata/e',
	'home.remove': 'Rimuovi',
	'home.removeFile': 'Rimuovi file',
	'home.step2': 'Data fine quindicina',
	'home.fortnightHint': 'PAT: Data fine quindicina (questa data). PLI: Data mese (il suo mese).',
	'home.step3': 'Cartella di output',
	'home.chooseFolder': 'Scegli cartella',
	'home.step4': 'Genera',
	'home.generating': 'Generazione…',
	'home.generate': 'Genera tracciati',
	'home.filesGenerated': 'File generati.',
	'home.open': 'Apri ↗',
	'home.openTitle': 'Apri {path}',
	'home.warnings': '{n} avviso/i:',

	'products.title': 'Tabella prodotti',
	'products.uploadSkeleton': 'Carica scheletro',
	'products.add': 'Aggiungi prodotto',
	'products.searchByCode': 'Cerca per codice',
	'products.incompleteOnly': 'Solo incompleti',
	'products.code': 'Codice',
	'products.description': 'Descrizione',
	'products.units': 'Unità',
	'products.type': 'Tipo',
	'products.admCode': 'Codice ADM',
	'products.capacity': 'Capacità (ml)',
	'products.nicotine': 'Nicotina (mg/ml)',
	'products.packages': 'Confezioni',
	'products.tabella': 'Tabella',
	'products.loading': 'Caricamento prodotti...',
	'products.none': 'Nessun prodotto trovato.',
	'products.incompleteTag': 'Incompleto',
	'products.incompleteTitle': 'Scheletro non caricato — escluso dai tracciati',
	'products.codeEmpty': 'Il codice prodotto non può essere vuoto.',
	'products.deleteConfirm': 'Eliminare questo prodotto?',

	'customers.title': 'Tabella clienti',
	'customers.add': 'Aggiungi cliente',
	'customers.searchByTax': 'Cerca per codice di imposta',
	'customers.searchByVat': 'Cerca per P.IVA / C.F.',
	'customers.allTypologies': 'Tutte le tipologie',
	'customers.taxCode': 'Codice imposta',
	'customers.ordinal': 'Progressivo',
	'customers.typology': 'Tipologia',
	'customers.vat': 'P.IVA / C.F.',
	'customers.address': 'Indirizzo',
	'customers.municipality': 'Comune',
	'customers.province': 'Provincia',
	'customers.phTaxCode': 'Codice fiscale',
	'customers.phOrdinal': 'Numero progressivo',
	'customers.phVat': 'Partita IVA (opzionale)',
	'customers.phAddress': 'Indirizzo',
	'customers.phProvince': 'Nome provincia',
	'customers.phMunicipality': 'Nome comune',
	'customers.loading': 'Caricamento clienti...',
	'customers.none': 'Nessun cliente trovato.',
	'customers.deleteConfirm': 'Eliminare questo cliente?',
	'customers.modalTitle': 'Risolvi la provincia per le righe ambigue',
	'customers.modalHint':
		'La provincia è mancante e il comune corrisponde a più province. Seleziona una provincia per ogni riga.',
	'customers.row': 'Riga',
	'customers.selectProvince': 'Seleziona provincia',
	'customers.confirmUpload': 'Conferma caricamento',
	'customers.fieldRequired': '{field} è obbligatorio e deve essere maggiore di 0.',
	'customers.fieldPositiveInt': '{field} deve essere un intero positivo.',
	'customers.skipped': '{n} riga/e saltata/e (non importata/e):',
	'customers.rowPrefix': 'Riga {n}: {msg}',
	'customers.selectProvinceFor': 'Seleziona una provincia per la/e riga/e: {rows}',

	'templates.title': 'Modelli ADM',
	'templates.intro':
		"Carica i due modelli Excel usati per generare i tracciati. Vengono salvati nei dati dell'app e riutilizzati ad ogni generazione.",
	'templates.pliTitle': 'Tracciato PLI',
	'templates.patTitle': 'Tracciato PAT',
	'templates.pliDesc': 'Modello ADM per i Prodotti Liquidi da Inalazione (tracciati_pli.xlsx).',
	'templates.patDesc': 'Modello ADM per i Prodotti Accessori dei Tabacchi (tracciati_pat.xlsx).',
	'templates.loaded': 'Caricato',
	'templates.notLoaded': 'Non caricato',
	'templates.replace': 'Sostituisci',
	'templates.upload': 'Carica',
	'templates.saved': 'Modello {kind} salvato.'
};

const messages: Record<Locale, Record<keyof typeof en, string>> = { en, it };

let locale = $state<Locale>(
	((typeof localStorage !== 'undefined' && localStorage.getItem('locale')) as Locale) || 'en'
);

export function getLocale(): Locale {
	return locale;
}

export function setLocale(next: Locale): void {
	locale = next;
	if (typeof localStorage !== 'undefined') {
		localStorage.setItem('locale', next);
	}
}

export function t(key: keyof typeof en, vars?: Record<string, string | number>): string {
	let s: string = messages[locale][key] ?? en[key];
	if (vars) {
		for (const k in vars) {
			s = s.replaceAll(`{${k}}`, String(vars[k]));
		}
	}
	return s;
}
