# CLAUDE.md

Guida per agenti AI che lavorano su questo repository. Prosa in italiano, identificatori/codice in inglese (come nel codebase).

---

## Cos'è questo progetto

App **desktop offline** che aiuta una specifica azienda a compilare i file Excel da inviare all'**Agenzia delle Dogane e dei Monopoli (ADM)** per la vendita di:

- **PLI** – Prodotti Liquidi da Inalazione
- **PAT** – Prodotti Accessori dei Tabacchi

L'ADM richiede, **ogni 15 e 30 del mese**, dei prospetti Excel con un **template a struttura fissa che NON può essere modificata**. L'app genera questi Excel pronti per l'invio.

**Flusso d'uso previsto** (diagramma di alto livello nel [README](README.md#workflow)):

1. **Catalogo prodotti** (una tantum, o quando cambiano):
   - Si carica l'elenco prodotti in struttura **info3/info4**: un file **PLI**, uno **PAT**, oppure **un unico file con entrambi**.
   - PLI vs PAT si distinguono dalla colonna **`gruppo`**: valore **`5`** = PLI, **altro numero** = PAT. Riga **senza numero** in `gruppo` → **ignorata**.
   - Da questi file si salvano `code`, `units` e `packages` (per PAT: `units` dalla colonna **info4**, `packages` dalla colonna **info3**). La **`description` NON viene da questi file**.
   - Poi si caricano gli **scheletri** (`skeleton_pli`, `skeleton_pat`): matchando sul **`code`** si riempiono `description` (PLI e PAT) e `capacity` + `nicotine` (solo PLI).
2. **Clienti**: import **già implementato** (`service/customer.rs`, `routes/customers/+page.svelte`).
3. **Generazione output** (ogni scadenza): si caricano **tutte le fatture**; si compilano due file partendo dai template salvati **`tracciati_pli`** e **`tracciati_pat`**. Per ogni fattura:
   - **Cliente**: ricerca prima per **codice fiscale**, altrimenti per **partita IVA**.
   - **Numero fattura**: dalla colonna **AN** (di norma cella **`AN18`**); dev'essere un **intero**, altrimenti errore.
   - Solo le righe con un valore nella colonna **`Accise`**: si legge il `code` dalla colonna **`Articolo`** e si recupera il prodotto dal DB.
     - **PLI** → `tracciati_pli`: `numero di confezioni` = `units` × quantità riga.
     - **PAT** → `tracciati_pat`: `N° confezioni immesse in consumo` = `packages` × quantità riga.
   - **`CMNR Rivendita generi di monopolio`** = tax code del cliente **solo se** `typology` = **`RIVENDITA`**, altrimenti **vuoto**. Per PLI il tax code va in una **colonna dedicata separata**.

**Vincoli di scope — importanti:**
- È un tool **su misura** per la struttura dei file di QUESTA azienda. Non deve essere generico/multi-azienda. Non aggiungere astrazioni "per supportare altri casi".
- Gira **offline**, è **desktop** (no mobile, no web hosting).
- La struttura dei template Excel ADM è **immutabile**: l'output deve rispettarla cella per cella, incluse le formule.

---

## Stack

| Layer | Tecnologia |
|-------|-----------|
| Shell desktop | **Tauri v2** (Rust) |
| Frontend | **SvelteKit** (Svelte 5, runes) + **Tailwind CSS v4** |
| Adapter | `@sveltejs/adapter-static`, SSR disabilitato, `prerender = true` (`src/routes/+layout.ts`) |
| DB | **SQLite** via `rusqlite` (feature `bundled`) |
| Excel lettura | `calamine` |
| Excel scrittura | `rust_xlsxwriter` |
| Errori Rust | `thiserror` |

Toolchain minima: Node 18+, Rust stable (1.77.2+).

---

## Comandi

```sh
npm install            # dipendenze JS
npm run tauri dev      # app desktop in dev (hot reload)
npm run tauri build    # bundle di produzione
npm run check          # svelte-check + sync (type-check frontend)
```

Per il backend Rust: `cargo build` / `cargo clippy` dentro `src-tauri/`.

---

## Architettura

### Backend Rust (`src-tauri/src/`) — a tre layer

```
controller/   → handler dei comandi #[tauri::command] (thin)
service/      → logica di business, parsing/validazione, trasformazioni
repository/    → accesso dati: SQLite (rusqlite) + I/O Excel
utils.rs      → helper condivisi (resolve_db_path, parse_i64)
lib.rs        → registrazione comandi, setup, creazione tabelle allo startup
```

Un dominio = un file per layer (es. `controller/product.rs`, `service/product.rs`, `repository/product.rs`). Domini attuali: **product**, **customer**, **excel**.

### Frontend (`src/`)

```
lib/*-repository.ts   → wrapper tipizzati su invoke() (un modulo per dominio)
lib/page-actions.ts    → logica di pagina estraibile/testabile (dependency injection via ActionDeps)
lib/index.ts          → re-export dei repository ($lib)
routes/               → +page.svelte per ogni schermata (home, products, customers)
```

---

## Pattern da seguire (osservati nel codice — replicarli)

### Backend

- **Controller sottili.** Risolvono il db path con `resolve_db_path(&app_handle)?`, delegano al service/repository, e convertono l'errore al confine con `.map_err(|e| e.to_string())`. Niente logica qui.
- **Comandi `async`** che ritornano `Result<T, String>` (la stringa è il messaggio mostrato al frontend).
- **Repository = wrapper async + `spawn_blocking` + funzione `*_sync`.** Ogni operazione DB ha una `pub async fn foo(...)` che fa `tauri::async_runtime::spawn_blocking(move || foo_sync(...))` e una `fn foo_sync(...)` sincrona che apre la `Connection` e fa il lavoro. rusqlite è bloccante: **non** chiamarlo direttamente da contesto async.
- **Errori interni** con l'enum `AppError` (`thiserror`) — varianti `Io` e `Processing`. Si converte a `String` solo al confine del comando.
- **serde:**
  - struct esposte al frontend → `#[serde(rename_all = "camelCase")]`
  - enum come `ProductType` → `#[serde(rename_all = "lowercase")]` (`"pli"`/`"pat"`)
- **Validazione e normalizzazione:**
  - guard clause / early return per gli edge case (file mancante, estensione non `.xlsx`, ecc.)
  - i codici prodotto si normalizzano con `trim().to_uppercase()` (`normalize_product_code`)
  - clamp dei parametri di paginazione (`page.max(1)`, `page_size.max(1)`, tetto `MAX_*_PAGE_SIZE = 1000`)
- **SQLite:**
  - tabelle create allo startup con `CREATE TABLE IF NOT EXISTS` in `lib.rs` (`ensure_*_tables_on_startup`)
  - sempre query parametrizzate (`params![]` / `params_from_iter`), mai string interpolation di valori
  - insert massivi in transazione + batch (`INSERT_BATCH_SIZE`) con **UPSERT** (`ON CONFLICT(code) DO UPDATE ... WHERE` aggiorna solo se qualcosa è cambiato)
- **Costanti** in `SCREAMING_SNAKE_CASE`, inclusi gli indici di colonna Excel (es. `CODE_COLUMN_INDEX: usize = 5`).
- **Lettura header Excel resiliente:** gli header si normalizzano (`normalize_header`) e si accettano più alias italiani/inglesi (`find_required_header` / `find_optional_header`). Vedi `service/customer.rs`.

### Frontend

- **Svelte 5 runes:** stato con `let x = $state(...)`. Niente store legacy salvo necessità.
- **Un repository per dominio** in `src/lib/` che incapsula `invoke('command_name', { args })` e tipizza input/output. I tipi TS rispecchiano le struct Rust in **camelCase**.
- **Gestione errori UI:** `try/catch` attorno alle invoke, errore salvato come stringa in `errorMsg` (`String(e)`), banner mostrato in cima alla card.
- **Flag di stato** booleani in question-form (`loading`, `saving`, `hasNextPage`).
- Dialog file/cartella via `@tauri-apps/plugin-dialog`; filtri estensione `xlsx`.

### Stile generale

Vale la skill **`clean-code`** in `.agents/skills/clean-code/SKILL.md` (è marcata mandatory): conciso, diretto, funzioni piccole/SRP, guard clause, niente commenti ovvi, niente over-engineering (YAGNI/KISS), nomi che rivelano l'intento. Non scrivere helper per one-liner né astrazioni speculative.

---

## Schema database

DB SQLite in `app_data_dir`, file **`pli_pat.db`** (vedi `utils::DB_FILE_NAME`).

| Tabella | Note |
|---------|------|
| `product` | `product_type` ('pli'/'pat', CHECK), `code` UNIQUE non vuoto, `description`, `units`, e i campi opzionali per tipo: **`capacity`**+**`nicotine`** (PLI), **`packages`** (PAT). Più `adm_code` (codice ADM, PAT → tracciati_pat col L) e `tabella` (PAT → tracciati_pat col K), entrambi dallo **scheletro**. Migrazione idempotente in `lib.rs` (`add_column_if_missing`) per DB esistenti |
| `customer` | `tax_code` UNIQUE, `ordinal_number`, `typology` (CHECK enum), `vat_number` UNIQUE nullable, `address`, `municipality_id` FK |
| `municipality` | `name` + `province_name`, UNIQUE(name, province_name) |

- **PLI vs PAT:** un'unica tabella `product` discriminata da `product_type`. I campi specifici sono colonne nullable, con un `CHECK` che impone solo la *forma* per tipo (PLI ha `packages` NULL; PAT ha `capacity`/`nicotine` NULL). I campi posseduti dallo scheletro restano NULL fino all'upload: un prodotto **incompleto** (PLI senza capacity/nicotine, PAT senza adm_code) è taggato in UI e **saltato** nella generazione tracciati — vedi `Product::is_skeleton_complete`. `code` è UNIQUE globale: la ricerca per codice è una sola query indicizzata. Esposta al frontend come tipo `Product` con campi opzionali.
- **`customer.typology`** ammette solo: `'ESERCIZIO DI VICINATO'`, `'RIVENDITA'`, `'FARMACIA'`, `'PARAFARMACIA'`.
- **Provincia ambigua:** in import clienti, se un comune mappa a più province, la riga è "ambigua" e il frontend deve far scegliere la provincia. Flusso a due fasi: `validate_customers_excel` → utente risolve → `confirm_customers_excel_upload` (vedi `service/customer.rs`).

---

## Dati di esempio (fuori dal repo)

Cartelle di lavoro aggiuntive con i template/esempi reali ADM:
- `../excel_examples/` — template PLI/PAT, scheletri, fatture (`FT ...`; `esempio_fattura` ), anagrafiche, output di prova.
- `../elenco_prodotti_info3/` — `pli codici.xlsx`, `pat codici.xlsx` (anagrafica prodotti).

Usali come riferimento per la struttura esatta delle celle. **Non** assumere strutture: aprili e verifica.

---

## Stato attuale / gotchas (verificare prima di lavorarci)

- **Flusso completo implementato.** Import info3/info4 (`service/product::upload_products_excel`), scheletri (`upload_skeleton_excel`), parsing fatture (`service/invoice`), generazione `service/excel::generate_tracciati` che riempie i template salvati in `src-tauri/resources/` via `repository/excel::fill_template` (umya-spreadsheet, modifica in-place: costanti/formule preservate, formule copiate verso il basso). Comando `generate_tracciati` (multi-fattura + periodo unico).
- **Da verificare con i template/scheletri REALI** (in `src-tauri/resources/` ora ci sono i campioni ADM vuoti): righe di inizio dati, quali colonne sono formule (col `P`), layout reale di `skeleton_pat` (code col Q, desc col M, adm col L). Il `period` è un'unica stringa scritta uguale su PLI ("Data mese") e PAT ("Data fine quindicina"): potrebbero servire formati diversi.
- **Ownership campi prodotto:** import possiede `{product_type, units, packages}`, scheletro possiede `{description, capacity, nicotine, adm_code, tabella}`. La UPSERT di re-import (`build_product_batch_insert_sql`) aggiorna solo i campi import; lo scheletro fa UPDATE-by-code (`update_products_from_skeleton`) con COALESCE per non azzerare l'altro lato. I campi dello scheletro restano **NULL** all'import (niente più placeholder `0`) finché non arriva lo scheletro. `gruppo` serve solo a classificare PLI/PAT; il valore `tabella` viene da `skeleton_pat`.
- **`repository/excel::write_excel` non è più usato** (warning `never used`): la generazione usa `fill_template`. Eliminabile.
- Branch di lavoro corrente: `dev` (main = `main`).

---

## Convenzioni Git

- Lavorare su branch feature, non direttamente su `main`.
- Messaggi di commit in stile conventional (`feat:`, `fix:`, `chore:`, `refactor:`), come nello storico.
