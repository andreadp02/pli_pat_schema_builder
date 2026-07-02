# Product

## Register

product

## Users

A small number of back-office staff at one specific Italian company that sells PLI (Prodotti Liquidi da Inalazione) and PAT (Prodotti Accessori dei Tabacchi). They are not technical. Twice a month (the 15th and the last day) they must produce fixed-structure Excel prospetti for the Agenzia delle Dogane e dei Monopoli (ADM). They work on a desktop, offline, under deadline pressure, and cannot afford a malformed file being rejected by ADM.

## Product Purpose

A bespoke offline desktop app (Tauri) that turns the company's own invoices, product catalog, and customers into the two ADM tracciati Excel files (`tracciati_pli`, `tracciati_pat`), cell-for-cell correct against ADM's immutable templates. It exists to remove hours of manual, error-prone Excel compilation and the risk of a rejected submission. Success = the generated files are accepted by ADM without rework, and the operator trusts the output without re-checking every cell by hand.

## Brand Personality

Quiet, dependable, precise. This is a tool, not a brand statement: it should feel like a well-made instrument that disappears into the task. Three words: trustworthy, calm, exact.

## Anti-references

- Consumer SaaS marketing gloss: gradients, hero metrics, playful illustrations, celebratory confetti.
- Generic dashboard-template look (identical card grids, decorative accents, kicker eyebrows).
- Anything that adds visual noise or ambiguity to a compliance task where correctness is the whole point.

## Design Principles

1. **The task is a sequence — show it.** Generation is an ordered flow (invoices → date → folder → generate); the UI should make progress and completion legible at a glance.
2. **One confident primary action per screen.** Setup steps are secondary; the money action (Generate) is the single emphasized control.
3. **State over decoration.** Color and motion communicate status (pending, complete, error, success), never ornament.
4. **Earned familiarity.** Standard, boring, correct controls. No invented affordances for standard tasks.
5. **Fail loud, succeed quietly.** Errors and warnings must be impossible to miss; success is a calm confirmation with direct access to the files.

## Accessibility & Inclusion

- Bilingual UI (Italian / English), Italian primary.
- Body/label text meets WCAG AA contrast (≥4.5:1) against its surface; interactive controls have visible keyboard focus.
- Motion is limited to state transitions and honors `prefers-reduced-motion`.
